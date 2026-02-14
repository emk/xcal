//! Application layer that owns the event loop and conference lifecycle.
//!
//! The TCP listener outlives any single conference. `XcaliberApp` owns the
//! mpsc channel and runs the event loop forever (like C's `-l` mode).
//! When a conference ends, it is dropped and a fresh one is created when
//! the next user connects.

use async_trait::async_trait;
use dcts::ports::{PortMessage, PortMessageSink};
use miette::Report;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    conferences::{Conference, ConferenceMessage},
    lang::Messages,
};

/// The top-level application — owns the channel and conference lifecycle.
pub struct XcaliberApp {
    receiver: mpsc::Receiver<ConferenceMessage>,
    sender: mpsc::Sender<ConferenceMessage>,
    messages: Messages,
    conference: Option<Conference>,
}

impl XcaliberApp {
    /// Create a new application instance.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32);
        Self {
            receiver,
            sender,
            messages: Messages::new(),
            conference: None,
        }
    }

    /// Construct a [`PortMessageSink`] for use by the TCP listener.
    /// Wraps each [`PortMessage`] in [`ConferenceMessage::Port`] before
    /// sending.
    pub fn port_message_sink(&self) -> Box<dyn PortMessageSink> {
        Box::new(ConferenceMessageSink {
            sender: self.sender.clone(),
        })
    }

    /// Run the main event loop forever, creating and destroying conferences
    /// as users connect and disconnect.
    pub async fn run(&mut self) -> Result<(), Report> {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ConferenceMessage::Port(port_msg) => {
                    self.handle_port_message(port_msg)?;
                }
            }
        }

        Ok(())
    }

    /// Dispatch a port message to the current conference, creating one
    /// on demand if needed.
    fn handle_port_message(
        &mut self,
        port_msg: PortMessage,
    ) -> Result<(), Report> {
        match port_msg {
            PortMessage::New { port } => {
                let conf = self.conference.get_or_insert_with(|| {
                    info!("starting new conference");
                    Conference::new()
                });
                conf.handle_new(port, &self.messages)?;
            }
            PortMessage::InputLine { port_id, input } => {
                if let Some(conf) = self.conference.as_mut() {
                    conf.handle_input(port_id, &input, &self.messages)?;
                } else {
                    warn!(?port_id, "input line with no active conference");
                }
            }
            PortMessage::SendComplete { port_id } => {
                if let Some(conf) = self.conference.as_mut() {
                    conf.handle_send_complete(port_id);
                }
            }
            PortMessage::Disconnected { port_id } => {
                if let Some(conf) = self.conference.as_mut() {
                    if conf.handle_disconnected(port_id) {
                        info!("conference ended, awaiting next connection");
                        self.conference = None;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Bridges the port layer to the app by wrapping [`PortMessage`]s
/// in [`ConferenceMessage::Port`] before sending.
struct ConferenceMessageSink {
    sender: mpsc::Sender<ConferenceMessage>,
}

#[async_trait]
impl PortMessageSink for ConferenceMessageSink {
    async fn send(&self, msg: PortMessage) -> Result<(), PortMessage> {
        self.sender
            .send(ConferenceMessage::Port(msg))
            .await
            .map_err(|e| match e.0 {
                ConferenceMessage::Port(m) => m,
            })
    }

    fn clone_sink(&self) -> Box<dyn PortMessageSink> {
        Box::new(ConferenceMessageSink {
            sender: self.sender.clone(),
        })
    }
}
