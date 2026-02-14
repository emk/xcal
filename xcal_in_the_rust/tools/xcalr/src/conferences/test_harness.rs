//! In-process test harness for [`Conference`].
//!
//! Provides [`TestPort`] (a fake [`Port`] that captures output in memory)
//! and [`TestConference`] (a wrapper with a 3-method API: `connect`,
//! `input`, `output`).

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use dcts::ports::{Port, PortId};
use miette::Report;

use super::Conference;
use crate::users::UserId;

/// Shared output buffer between a [`TestPort`] and the test harness.
type SharedBuffer = Arc<Mutex<Vec<u8>>>;

/// A fake [`Port`] that writes output into a shared in-memory buffer.
#[derive(Debug)]
pub(crate) struct TestPort {
    id: PortId,
    name: String,
    buffer: SharedBuffer,
    shut_down: bool,
}

impl TestPort {
    /// Create a new test port and its shared output buffer.
    pub(crate) fn new(name: &str) -> (Self, SharedBuffer) {
        let buffer: SharedBuffer = Arc::new(Mutex::new(Vec::new()));
        let port = Self {
            id: PortId::new(),
            name: name.to_string(),
            buffer: Arc::clone(&buffer),
            shut_down: false,
        };
        (port, buffer)
    }
}

impl Port for TestPort {
    fn id(&self) -> PortId {
        self.id
    }

    fn public_name(&self) -> &str {
        &self.name
    }

    fn admin_name(&self) -> &str {
        &self.name
    }

    fn try_send(&mut self, data: Bytes) -> Result<(), Bytes> {
        if self.shut_down {
            return Err(data);
        }
        #[allow(clippy::expect_used)]
        self.buffer
            .lock()
            .expect("lock poisoned")
            .extend_from_slice(&data);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Report> {
        self.shut_down = true;
        Ok(())
    }
}

/// A test wrapper around [`Conference`] that manages fake ports and
/// provides a simple 3-method API.
pub(crate) struct TestConference {
    conference: Conference,
    /// Indexed by `UserId.0` — each slot holds the port ID and shared buffer.
    connections: Vec<Option<(PortId, SharedBuffer)>>,
}

impl TestConference {
    /// Create a new empty test conference.
    pub(crate) fn new() -> Self {
        Self {
            conference: Conference::new(),
            connections: Vec::new(),
        }
    }

    /// Connect a user with the given name.
    ///
    /// Creates a [`TestPort`], registers it with the conference, sends
    /// the name as login input, and drains the welcome/intro output.
    /// Returns the assigned [`UserId`].
    pub(crate) fn connect(&mut self, name: &str) -> UserId {
        let (port, buffer) = TestPort::new(name);
        let port_id = port.id();

        self.conference
            .handle_new(Box::new(port))
            .expect("handle_new failed");

        // The user slot is the last non-None entry.
        let user_id = self
            .conference
            .port_to_user
            .get(&port_id)
            .copied()
            .expect("user not registered");

        // Ensure our connections vec is large enough.
        let idx = usize::from(user_id.0);
        if self.connections.len() <= idx {
            self.connections.resize_with(idx.saturating_add(1), || None);
        }
        self.connections[idx] = Some((port_id, buffer));

        // Send the name as login input.
        let name_bytes = bytes::BytesMut::from(name.as_bytes());
        self.conference
            .handle_input(port_id, &name_bytes)
            .expect("handle_input (login) failed");

        // Drain welcome output so tests start clean.
        self.drain(user_id);

        user_id
    }

    /// Send a line of input from the given user.
    pub(crate) fn input(&mut self, id: UserId, line: &str) {
        let idx = usize::from(id.0);
        let (port_id, _) =
            self.connections[idx].as_ref().expect("user not connected");
        let port_id = *port_id;
        let input = bytes::BytesMut::from(line.as_bytes());
        self.conference
            .handle_input(port_id, &input)
            .expect("handle_input failed");
    }

    /// Read and clear all pending output for the given user.
    pub(crate) fn output(&mut self, id: UserId) -> String {
        let idx = usize::from(id.0);
        let (_, buffer) =
            self.connections[idx].as_ref().expect("user not connected");
        #[allow(clippy::expect_used)]
        let mut buf = buffer.lock().expect("lock poisoned");
        let out = String::from_utf8_lossy(&buf).into_owned();
        buf.clear();
        out
    }

    /// Drain (discard) all pending output for the given user.
    pub(crate) fn drain(&mut self, id: UserId) {
        let _ = self.output(id);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn harness_connect_works() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        // Verify we got a valid UserId.
        assert_eq!(master, UserId(0));

        // Output should be empty after connect drains the welcome.
        let out = tc.output(master);
        assert!(out.is_empty(), "expected empty output after connect: {out}");
    }
}
