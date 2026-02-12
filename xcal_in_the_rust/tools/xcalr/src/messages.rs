//! Chat message types for the conference system.
//!
//! Models the message structures from the C implementation (`message.h`).
//! Messages are created once and shared across recipient queues via `Arc`,
//! replacing the C manual reference counting.

#![allow(dead_code)]

use crate::users::{UserId, UserSet};

/// The kind of message being delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Normal directed message.
    Direct,
    /// Tell-all message.
    TellAll,
    /// System warning — cannot be ignored.
    Warning,
    /// Join/leave/name-change notification.
    Notification,
    /// Help text.
    Help,
    /// Explanation text.
    Explanation,
}

/// A message in the conference system.
///
/// Messages are created once, then shared (via `Arc`) across the message
/// queues of all recipients. The C code used manual reference counting;
/// `Arc` serves the same purpose.
#[derive(Debug)]
pub struct Message {
    /// Who sent this message (`None` for system-generated messages).
    pub from: Option<UserId>,
    /// What kind of message this is.
    pub kind: MessageKind,
    /// Who should receive this message.
    pub recipients: UserSet,
    /// The formatted message body (headers, content, and \r\n line endings).
    pub body: Vec<u8>,
}
