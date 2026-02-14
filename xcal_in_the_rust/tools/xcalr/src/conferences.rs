//! Core conference domain types.
//!
//! Models the conference-level data from the C implementation (`con.h`):
//! departure records, bounce lists, and the main conference struct.

#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc, time::Instant};

use async_trait::async_trait;
use dcts::ports::{PortId, PortMessage, PortMessageSink};
use tokio::sync::mpsc;

use crate::{
    messages::Message,
    users::{User, UserId},
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
    /// Receiver for external messages (new connections, user input, etc.).
    pub receiver: mpsc::Receiver<ConferenceMessage>,
    /// Sender for external messages (cloned and handed out as needed).
    pub sender: mpsc::Sender<ConferenceMessage>,
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
    /// Construct a [`PortMessageSink`] for use by
    /// [`dcts::ports::Port`] implementations. Wraps each
    /// [`PortMessage`] in [`ConferenceMessage::Port`] before sending.
    fn port_message_sink(&self) -> Box<dyn PortMessageSink> {
        Box::new(ConferencePortMessageSink {
            sender: self.sender.clone(),
        })
    }
}

/// Bridges the port layer to the conference by wrapping [`PortMessage`]s
/// in [`ConferenceMessage::Port`] before sending.
struct ConferencePortMessageSink {
    sender: mpsc::Sender<ConferenceMessage>,
}

#[async_trait]
impl PortMessageSink for ConferencePortMessageSink {
    async fn send(&self, msg: PortMessage) -> Result<(), PortMessage> {
        self.sender
            .send(ConferenceMessage::Port(msg))
            .await
            .map_err(|e| match e.0 {
                ConferenceMessage::Port(m) => m,
            })
    }

    fn clone_sink(&self) -> Box<dyn PortMessageSink> {
        Box::new(ConferencePortMessageSink {
            sender: self.sender.clone(),
        })
    }
}
