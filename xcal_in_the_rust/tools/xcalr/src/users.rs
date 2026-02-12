//! Core user domain types for the conference system.
//!
//! Models the per-user state machine, roles, and preference flags from
//! the C implementation (`user.h`), using Rust enums and structs instead
//! of bitpacked flags and integer constants.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use tokio::net::TcpStream;

use crate::messages::Message;

/// Slot index into the conference user array (0..max_users).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub u8);

/// A compact set of users, one bit per slot. Supports up to 64 users.
///
/// Replaces three separate C bitmap implementations: recipient bitmaps
/// (`byte rcpt[5]`), ignore maps (`byte *igmap`), and give-maps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UserSet(pub u64);

impl UserSet {
    /// An empty user set.
    pub fn new() -> Self {
        Self(0)
    }

    /// A set containing all users in `0..count`.
    pub fn all(count: u8) -> Self {
        if count == 0 {
            return Self(0);
        }
        if count >= 64 {
            return Self(u64::MAX);
        }
        // count is in 1..63 here (0 and >=64 handled above).
        #[allow(clippy::arithmetic_side_effects)]
        Self((1u64 << count) - 1)
    }

    /// Whether the set contains the given user.
    pub fn contains(self, id: UserId) -> bool {
        self.0 & (1u64 << id.0) != 0
    }

    /// Add a user to the set.
    pub fn insert(&mut self, id: UserId) {
        self.0 |= 1u64 << id.0;
    }

    /// Remove a user from the set.
    pub fn remove(&mut self, id: UserId) {
        self.0 &= !(1u64 << id.0);
    }

    /// The number of users in the set.
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Iterate over the user IDs in the set.
    pub fn iter(self) -> UserSetIter {
        UserSetIter(self.0)
    }
}

/// Iterator over the user IDs in a [`UserSet`].
pub struct UserSetIter(u64);

impl Iterator for UserSetIter {
    type Item = UserId;

    fn next(&mut self) -> Option<UserId> {
        if self.0 == 0 {
            return None;
        }
        // Safe: trailing_zeros of a non-zero u64 is 0..63, fits in u8.
        #[allow(clippy::cast_possible_truncation)]
        let bit = self.0.trailing_zeros() as u8;
        self.0 &= self.0.wrapping_sub(1); // clear lowest set bit
        Some(UserId(bit))
    }
}

/// What kind of output a user is currently receiving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveKind {
    Message,
    Help,
    Explanation,
}

/// What a user is currently doing — the core state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    /// Entering name at login prompt.
    Login,
    /// Just logged in — triggers join notification (transient).
    JustLoggedIn,
    /// Idle, waiting for input. Messages can be delivered in this state.
    Idle,
    /// Composing a message (lines accumulate until blank line).
    Composing,
    /// Message ready for delivery (transient).
    Composed,
    /// Processing a command (transient).
    Command,
    /// Server is sending output to the user's socket.
    Receiving(ReceiveKind),
    /// Entering a new name.
    ChangingName,
    /// Name change complete — triggers notification (transient).
    NameChanged,
    /// "Out" — rejecting messages, any input returns to Idle.
    Out,
    /// Master composing a conference warning.
    ComposingWarning,
    /// Warning composition complete (transient).
    WarningComposed,
    /// Line mode — commands only, no message delivery.
    Line,
}

/// A user's role in the subconference hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Regular conference member.
    Normal,
    /// Controls a subconference.
    Submaster { tell_all_enabled: bool },
    /// Conference master.
    Master { tell_all_enabled: bool },
}

/// Per-user preference flags (toggled by commands like `ra`, `rp`, `rn`, etc.).
#[derive(Debug, Clone, Default)]
pub struct UserPrefs {
    /// Reject tell-all messages (`ra`/`aa`).
    pub reject_tell_all: bool,
    /// Hide port/location info (`rp`).
    pub reject_ports: bool,
    /// Ignore telnet break signals (`rb`).
    pub reject_break: bool,
    /// Reject join/leave notifications (`rn`/`an`).
    pub reject_notifications: bool,
    /// Strip control characters from output (`rc`/`oc`).
    pub reject_controls: bool,
    /// User has administrative privileges (`xyzzy`).
    pub privileged: bool,
}

/// A live user connected to the conference.
pub struct User {
    /// Slot index in the conference user array.
    pub id: UserId,
    /// User's chosen display name.
    pub name: String,
    /// Current state in the user state machine.
    pub state: UserState,
    /// Role in the subconference hierarchy.
    pub role: Role,
    /// Per-user preference flags.
    pub prefs: UserPrefs,
    /// TCP connection to the user.
    pub socket: TcpStream,
    /// Remote address of the connection.
    pub remote_addr: SocketAddr,
    /// Bytes received from the user, not yet processed.
    pub input: BytesMut,
    /// Bytes waiting to be sent to the user.
    pub output: BytesMut,
    /// Message currently being composed (if any).
    pub composing: Option<Message>,
    /// Outgoing message queue (shared via `Arc` with other recipients).
    pub queue: VecDeque<Arc<Message>>,
    /// Which master (or submaster) this user belongs to.
    pub master: UserId,
    /// Users this user is ignoring.
    pub ignoring: UserSet,
    /// When this user connected.
    pub connected_at: Instant,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_set() {
        let s = UserSet::new();
        assert!(s.is_empty());
        assert_eq!(s.count(), 0);
        assert!(!s.contains(UserId(0)));
        assert_eq!(s.iter().count(), 0);
    }

    #[test]
    fn insert_remove() {
        let mut s = UserSet::new();
        s.insert(UserId(3));
        s.insert(UserId(7));
        assert!(s.contains(UserId(3)));
        assert!(s.contains(UserId(7)));
        assert!(!s.contains(UserId(0)));
        assert_eq!(s.count(), 2);

        s.remove(UserId(3));
        assert!(!s.contains(UserId(3)));
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn all_users() {
        let s = UserSet::all(5);
        assert_eq!(s.count(), 5);
        for i in 0..5 {
            assert!(s.contains(UserId(i)));
        }
        assert!(!s.contains(UserId(5)));

        assert!(UserSet::all(0).is_empty());
    }

    #[test]
    fn iter_order() {
        let mut s = UserSet::new();
        s.insert(UserId(5));
        s.insert(UserId(1));
        s.insert(UserId(3));
        let ids: Vec<_> = s.iter().collect();
        assert_eq!(ids, vec![UserId(1), UserId(3), UserId(5)]);
    }
}
