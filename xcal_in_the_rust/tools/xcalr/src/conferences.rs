//! Core conference domain types.
//!
//! Models the conference-level data from the C implementation (`con.h`):
//! departure records, bounce lists, and the main conference struct.

#![allow(dead_code)]

use std::{collections::HashMap, fmt::Write as _, sync::Arc, time::Instant};

use bytes::BytesMut;
use dcts::ports::{Port, PortId, PortMessage};
use miette::Report;
use tracing::{debug, info, warn};

use crate::{
    lang::Messages,
    messages::Message,
    users::{Role, User, UserId, UserState},
};

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

/// A message sent to the conference from the outside world.
#[derive(Debug)]
pub enum ConferenceMessage {
    /// An event from the port/transport layer.
    Port(PortMessage),
}

/// The main conference — holds all live users and conference state.
pub struct Conference {
    /// Map from [`PortId`] to [`UserId`].
    pub port_to_user: HashMap<PortId, UserId>,
    /// User slots — `None` for empty slots, `Some` for connected users.
    pub users: Vec<Option<User>>,
    /// Users who have left (for the `left` command).
    pub left: Vec<LeftUser>,
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

impl Conference {
    /// Create a new empty conference.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            port_to_user: HashMap::new(),
            users: Vec::new(),
            left: Vec::new(),
            current_warning: None,
            started_at: now,
            ends_at: now,
            new_user_master: UserId(0),
            send_block_size: 512,
            bounced: Vec::new(),
            current_count: 0,
            peak_count: 0,
        }
    }

    // ── Event handlers ──────────────────────────────────────────────

    /// Handle a new connection.
    pub fn handle_new(
        &mut self,
        port: Box<dyn Port>,
        messages: &Messages,
    ) -> Result<(), Report> {
        let idx =
            self.users
                .iter()
                .position(|u| u.is_none())
                .unwrap_or_else(|| {
                    self.users.push(None);
                    self.users.len().saturating_sub(1)
                });

        let id = UserId(
            u8::try_from(idx).map_err(|_| miette::miette!("too many users"))?,
        );
        let port_id = port.id();

        let role = if self.current_count == 0 && self.peak_count == 0 {
            Role::Master {
                tell_all_enabled: false,
            }
        } else {
            Role::Normal
        };

        let welcome = match role {
            Role::Master { .. } => messages.master_welcome(),
            _ => messages.welcome(),
        };

        let mut user = User::new(id, port, role);
        user.output.extend_from_slice(welcome.as_bytes());
        Self::flush_user_output(&mut user);

        self.port_to_user.insert(port_id, id);
        self.users[idx] = Some(user);
        self.current_count = self.current_count.saturating_add(1);
        if self.current_count > self.peak_count {
            self.peak_count = self.current_count;
        }

        info!(%id, "user connected");
        Ok(())
    }

    /// Handle a line of input from a user.
    pub fn handle_input(
        &mut self,
        port_id: PortId,
        input: &BytesMut,
        messages: &Messages,
    ) -> Result<(), Report> {
        let Some(&id) = self.port_to_user.get(&port_id) else {
            warn!(?port_id, "input from unknown port");
            return Ok(());
        };

        let idx = usize::from(id.0);
        let Some(Some(ref user)) = self.users.get(idx) else {
            return Ok(());
        };
        let state = user.state;

        match state {
            UserState::Login => {
                let name = String::from_utf8_lossy(input).to_string();
                let intro = messages.intro();
                let talking = messages.talking_with(id, &name);

                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.name = name;
                    user.prefs.reject_controls = true;
                    user.state = UserState::Idle;
                    user.output.extend_from_slice(intro.as_bytes());
                    user.output.extend_from_slice(talking.as_bytes());
                    Self::flush_user_output(user);
                }
            }
            UserState::Idle => {
                let line = String::from_utf8_lossy(input);
                let cmd = line.trim().to_lowercase();

                // Transition to Command for dispatch.
                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.state = UserState::Command;
                }

                match cmd.as_str() {
                    "who" => {
                        let who = self.format_who(id);
                        self.append_output(id, &who);
                    }
                    "bye" => {
                        let conterm = messages.conference_terminated();
                        self.append_output(id, &conterm);
                        self.flush_output(id);
                        if let Some(Some(user)) = self.users.get_mut(idx) {
                            let _ = user.port.shutdown();
                        }
                        return Ok(());
                    }
                    _ => {
                        let err = messages.command_error();
                        self.append_output(id, &err);
                    }
                }

                // Back to Idle.
                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.state = UserState::Idle;
                }
                self.flush_output(id);
            }
            _ => {
                debug!(%id, ?state, "ignoring input in unexpected state");
            }
        }

        Ok(())
    }

    /// Handle send completion — flush any remaining output.
    pub fn handle_send_complete(&mut self, port_id: PortId) {
        if let Some(&id) = self.port_to_user.get(&port_id) {
            self.flush_output(id);
        }
    }

    /// Handle a disconnection. Returns `true` if the conference should end.
    pub fn handle_disconnected(&mut self, port_id: PortId) -> bool {
        let Some(&id) = self.port_to_user.get(&port_id) else {
            return false;
        };

        let idx = usize::from(id.0);
        self.port_to_user.remove(&port_id);

        if let Some(slot) = self.users.get_mut(idx) {
            *slot = None;
        }

        self.current_count = self.current_count.saturating_sub(1);
        info!(%id, "user disconnected");

        self.current_count == 0 && self.peak_count > 0
    }

    // ── Output helpers ──────────────────────────────────────────────

    /// Append text to a user's output buffer.
    fn append_output(&mut self, id: UserId, text: &str) {
        let idx = usize::from(id.0);
        if let Some(Some(user)) = self.users.get_mut(idx) {
            user.output.extend_from_slice(text.as_bytes());
        }
    }

    /// Try to send the user's buffered output to the port.
    fn flush_output(&mut self, id: UserId) {
        let idx = usize::from(id.0);
        if let Some(Some(user)) = self.users.get_mut(idx) {
            Self::flush_user_output(user);
        }
    }

    /// Flush output for a user reference (usable before insertion into vec).
    fn flush_user_output(user: &mut User) {
        if !user.output.is_empty() {
            let data = user.output.split().freeze();
            if let Err(data) = user.port.try_send(data) {
                user.output.extend_from_slice(&data);
            }
        }
    }

    // ── Command formatting ──────────────────────────────────────────

    /// Format the WHO listing.
    fn format_who(&self, requester_id: UserId) -> String {
        let mut output = String::from("\n");

        for slot in &self.users {
            let Some(user) = slot else { continue };

            let marker = if user.id == requester_id { "=>" } else { "  " };

            let role = match user.role {
                Role::Master { .. } => "M",
                Role::Submaster { .. } => "S",
                Role::Normal => " ",
            };

            let state = match user.state {
                UserState::Login | UserState::JustLoggedIn => "LOG",
                UserState::Idle => "IDL",
                UserState::Composing | UserState::Composed => "CMP",
                UserState::Command => "COM",
                UserState::Receiving(_) => "RCV",
                UserState::ChangingName | UserState::NameChanged => "NAM",
                UserState::Out => "OUT",
                UserState::ComposingWarning | UserState::WarningComposed => {
                    "WRN"
                }
                UserState::Line => "LIN",
            };

            let r = if user.prefs.reject_tell_all { "R" } else { " " };
            let i = if !user.ignoring.is_empty() { "I" } else { " " };
            let p = if user.prefs.reject_ports { "P" } else { " " };
            let c = if user.prefs.reject_controls { "C" } else { " " };
            let n = if !user.prefs.reject_notifications {
                "N"
            } else {
                " "
            };

            // Ignore writeln! errors on String (infallible).
            let _ = writeln!(
                output,
                "{marker}[#{id}] {role} {state} {r}{i}{p}{c}{n}  {name}",
                id = user.id,
                name = user.name,
            );
        }

        output
    }
}
