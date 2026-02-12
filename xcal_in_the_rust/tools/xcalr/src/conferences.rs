//! Core conference domain types.
//!
//! Models the conference-level data from the C implementation (`con.h`):
//! departure records, bounce lists, and the main conference struct.

#![allow(dead_code)]

use std::sync::Arc;
use std::time::Instant;

use tokio::net::TcpListener;

use crate::messages::Message;
use crate::users::{User, UserId};

/// Record of a user who has left the conference (for the `left` command).
#[derive(Debug, Clone)]
pub struct LeftUser {
    pub id: UserId,
    pub name: String,
    pub left_at: Instant,
    pub was_killed: bool,
}

/// A banned address.
#[derive(Debug, Clone)]
pub struct BounceEntry {
    pub addr: std::net::SocketAddr,
    pub hidden: bool,
}

/// The main conference — holds all live users, listener, and conference state.
pub struct Conference {
    /// User slots — `None` for empty slots, `Some` for connected users.
    pub users: Vec<Option<User>>,
    /// Users who have left (for the `left` command).
    pub left: Vec<LeftUser>,
    /// Listener socket for incoming connections.
    pub listener: TcpListener,
    /// Current conference warning, if any.
    pub current_warning: Option<Arc<Message>>,
    /// When the conference was started.
    pub started_at: Instant,
    /// When the conference is scheduled to end.
    pub ends_at: Instant,
    /// Which master receives new user notifications.
    pub new_user_master: UserId,
    /// Transmission block size in bytes (C: `trblk`).
    pub send_block_size: usize,
    /// Banned addresses.
    pub bounced: Vec<BounceEntry>,
    /// Current number of connected users.
    pub current_count: usize,
    /// Peak number of simultaneous users.
    pub peak_count: usize,
}
