//! Transport-agnostic connection identity and I/O.

#![allow(dead_code)]

pub mod fantasy;
pub mod tcp;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use miette::Report;
use uuid::Uuid;

/// A unique port ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortId(Uuid);

impl Default for PortId {
    fn default() -> Self {
        Self::new()
    }
}

impl PortId {
    /// Create a new unique port ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Abstraction over a connection's identity and outbound I/O.
///
/// This is inspired by DCTS "ports", an I/O abstraction that's probably older
/// than Unix. The first ports were likely hard-wired teletypes and then
/// physical VT100-style terminals through multiplexer machines. At some point
/// in the 80s, the remaining DCTS machines could be accessed via virtual ports
/// over Kiewit Stream Protocol and AppleTalk, as well as ports from a modem
/// pool.
///
/// In our modern version, different transports (TCP, web, etc.) can implement
/// [`Port`] to provide display names and to provide a way to send data back to
/// the client.
///
/// The application code calls these methods synchronously from its
/// message-processing loop. **No method on this trait may block.**
/// Implementations should write into a buffer or channel that an async
/// transport task drains.
///
/// ## I/O model
///
/// Input and output operate at different abstraction levels:
///
/// - **Input** ([`PortMessage::InputLine`]): line-oriented. The port
///   delivers complete lines with transport-specific terminators stripped.
/// - **Output** ([`Port::try_send`]): byte-oriented. The application
///   sends bytes with `\n` for line breaks; the port converts to the
///   transport's wire format (e.g. `\r\n` for TCP).
pub trait Port: std::fmt::Debug + Send {
    /// Unique identifier for this port.
    fn id(&self) -> PortId;

    /// Human-readable port name shown to regular users (e.g. "Avalon").
    ///
    /// WARNING: This method must never block.
    fn public_name(&self) -> &str;

    /// Detailed port name shown to xyzzy-privileged users (e.g. "192.168.1.1:4532").
    ///
    /// WARNING: This method must never block.
    fn admin_name(&self) -> &str;

    /// Try to enqueue bytes for sending to the connected client.
    ///
    /// Maps to C `net_write(up->sd, buf, len)` — called pervasively for command
    /// responses, message delivery, prompts, and control chars.
    ///
    /// Returns `Ok(())` if the data was accepted, or `Err(data)` if the
    /// port cannot accept data right now (queue full or connection dead).
    /// The caller keeps the data and can retry later.
    ///
    /// Most buffering and flow control lives in `User` in the "application"
    /// layer, so implementations should use very small queues (ideally
    /// capacity 1). The transport task sends [`PortMessage::SendComplete`]
    /// when it finishes writing.
    ///
    /// Disconnects are not reported through this return value. Port
    /// implementations are responsible for sending
    /// [`PortMessage::Disconnected`] through the [`PortMessageSink`] —
    /// that is the single disconnect code path.
    ///
    /// The application sends bytes with `\n` for line breaks. The port
    /// implementation converts to the transport's wire format (e.g. TCP
    /// converts `\n` to `\r\n`). The application never includes `\r`.
    ///
    /// WARNING: This method must never block. Implementations should enqueue
    /// data into an outbound buffer or channel, not perform I/O directly.
    fn try_send(&mut self, data: Bytes) -> Result<(), Bytes>;

    /// Shut down the connection.
    ///
    /// Maps to C `net_close(up->sd); up->sd = -1`. After this, the port
    /// should not be used for further sends.
    ///
    /// WARNING: This method must never block. Implementations should
    /// signal the transport task to close the connection, not close it
    /// directly.
    fn shutdown(&mut self) -> Result<(), Report>;
}

/// A channel for sending [`PortMessage`]s back to the application layer.
///
/// Transport tasks (TCP listeners, websocket acceptors, etc.) use this to
/// deliver events — new connections, input lines, send completions, and
/// disconnects — to the application. The application provides an
/// implementation that routes messages into its own event loop.
///
/// This uses async `send` to support backpressure: if the application is
/// behind on processing, the transport naturally slows down reading from
/// the socket.
#[async_trait]
pub trait PortMessageSink: Send + Sync {
    /// Send a port message to the application, waiting if necessary.
    async fn send(&self, msg: PortMessage) -> Result<(), PortMessage>;

    /// Clone this sink into a new boxed instance.
    fn clone_sink(&self) -> Box<dyn PortMessageSink>;
}

/// A message from the port/transport layer to the application layer.
///
/// These are transport-level events that any port implementation can produce.
/// The application layer wraps them in an application-specific enum variant.
#[derive(Debug)]
pub enum PortMessage {
    /// A new connection has arrived and needs to be registered with the
    /// application.
    New {
        /// The newly connected port.
        port: Box<dyn Port>,
    },
    /// A complete line of input from a port, with line endings stripped.
    ///
    /// Port implementations split the incoming byte stream into lines and
    /// remove transport-specific line terminators (`\r\n`, `\n`, etc.)
    /// before delivering. The application receives line content only.
    InputLine {
        /// Which port sent the input.
        port_id: PortId,
        /// The line content, without line endings.
        input: BytesMut,
    },
    /// The transport has finished writing the last chunk to this port.
    /// The application can now send the next block if any data is queued.
    SendComplete {
        /// Which port completed its send.
        port_id: PortId,
    },
    /// The transport detected that this port's connection is gone.
    Disconnected {
        /// Which port disconnected.
        port_id: PortId,
    },
}
