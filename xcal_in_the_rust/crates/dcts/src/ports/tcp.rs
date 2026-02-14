//! Raw TCP port implementation.
//!
//! No telnet negotiation, no IAC stripping. Designed for netcat and modern
//! TCP clients. Each connection gets a reader task and a writer task that
//! communicate with the application via [`PortMessageSink`].

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use bytes::{Bytes, BytesMut};
use miette::Report;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::{debug, trace, warn};

use super::{fantasy, Port, PortId, PortMessage, PortMessageSink};

/// A TCP connection managed by async reader/writer tasks.
#[derive(Debug)]
pub struct TcpPort {
    id: PortId,
    public_name: String,
    admin_name: String,
    /// Sender half of the capacity-1 channel to the writer task.
    /// `None` after `shutdown()` has been called.
    writer_tx: Option<mpsc::Sender<Bytes>>,
    /// Shared flag for disconnect coordination between reader and writer.
    shutdown: Arc<AtomicBool>,
}

impl Port for TcpPort {
    fn id(&self) -> PortId {
        self.id
    }

    fn public_name(&self) -> &str {
        &self.public_name
    }

    fn admin_name(&self) -> &str {
        &self.admin_name
    }

    fn try_send(&mut self, data: Bytes) -> Result<(), Bytes> {
        match &self.writer_tx {
            Some(tx) => tx.try_send(data).map_err(|e| e.into_inner()),
            None => Err(data),
        }
    }

    fn shutdown(&mut self) -> Result<(), Report> {
        self.shutdown.store(true, Ordering::Relaxed);
        // Drop the sender — the writer task sees a closed channel and exits.
        self.writer_tx.take();
        Ok(())
    }
}

/// Read lines from the socket and deliver them as [`PortMessage::InputLine`].
async fn reader_task(
    stream: tokio::net::tcp::OwnedReadHalf,
    port_id: PortId,
    sink: Box<dyn PortMessageSink>,
    disconnected: Arc<AtomicBool>,
) {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) => {
                // EOF.
                debug!(?port_id, "reader: EOF");
                break;
            }
            Ok(_n) => {
                // Strip trailing \r\n or \n.
                if buf.last() == Some(&b'\n') {
                    buf.pop();
                }
                if buf.last() == Some(&b'\r') {
                    buf.pop();
                }
                let input = BytesMut::from(buf.as_slice());
                trace!(?port_id, line = %String::from_utf8_lossy(&input), "reader: line");
                let msg = PortMessage::InputLine { port_id, input };
                if sink.send(msg).await.is_err() {
                    // Application dropped the sink — connection is orphaned.
                    warn!(?port_id, "reader: sink closed, shutting down");
                    break;
                }
            }
            Err(e) => {
                debug!(?port_id, error = %e, "reader: I/O error");
                break;
            }
        }
    }

    // Send Disconnected exactly once across reader + writer.
    if disconnected
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let _ = sink.send(PortMessage::Disconnected { port_id }).await;
    }
}

/// Convert `\n` to `\r\n` for TCP wire format.
fn lf_to_crlf(data: &[u8]) -> Bytes {
    // Fast path: if there are no bare \n (i.e. every \n is preceded by \r),
    // we'd still need to check. Simpler: just scan for \n and decide.
    if !data.contains(&b'\n') {
        return Bytes::copy_from_slice(data);
    }

    let mut out =
        Vec::with_capacity(data.len().saturating_add(data.len() / 10));
    for &b in data {
        if b == b'\n' {
            out.push(b'\r');
        }
        out.push(b);
    }
    Bytes::from(out)
}

/// Receive data from the application and write it to the socket.
async fn writer_task(
    mut writer: OwnedWriteHalf,
    port_id: PortId,
    mut rx: mpsc::Receiver<Bytes>,
    sink: Box<dyn PortMessageSink>,
    disconnected: Arc<AtomicBool>,
) {
    while let Some(data) = rx.recv().await {
        let wire_data = lf_to_crlf(&data);
        trace!(?port_id, len = wire_data.len(), "writer: sending");
        match writer.write_all(&wire_data).await {
            Ok(()) => {
                if let Err(e) = writer.flush().await {
                    debug!(?port_id, error = %e, "writer: flush error");
                    break;
                }
                let msg = PortMessage::SendComplete { port_id };
                if sink.send(msg).await.is_err() {
                    warn!(?port_id, "writer: sink closed, shutting down");
                    break;
                }
            }
            Err(e) => {
                debug!(?port_id, error = %e, "writer: I/O error");
                break;
            }
        }
    }

    // Send Disconnected exactly once across reader + writer.
    // If rx.recv() returned None, the application called shutdown() and
    // dropped the sender — no Disconnected needed (the app initiated it).
    // But if we broke out of the loop due to a write error, we do need it.
    if disconnected
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let _ = sink.send(PortMessage::Disconnected { port_id }).await;
    }
}

/// Accept TCP connections and spawn reader/writer tasks for each.
///
/// The caller creates and binds the [`TcpListener`]; this function
/// consumes it and runs until the listener is closed or an accept error
/// occurs.
///
/// For each accepted connection:
/// 1. A [`TcpPort`] is created and sent as [`PortMessage::New`].
/// 2. A reader task reads lines and sends [`PortMessage::InputLine`].
/// 3. A writer task receives [`Bytes`] via the port's channel and writes
///    to the socket, sending [`PortMessage::SendComplete`] after each write.
pub async fn listen(
    listener: TcpListener,
    sink: Box<dyn PortMessageSink>,
) -> Result<(), Report> {
    loop {
        let (stream, addr) = listener
            .accept()
            .await
            .map_err(|e| miette::miette!("TCP accept failed: {e}"))?;

        let port_id = PortId::new();
        let public_name = fantasy::fantasy_name(addr.ip()).to_string();
        let admin_name = addr.to_string();
        debug!(?port_id, %addr, public_name, "accepted connection");

        spawn_connection(stream, port_id, public_name, admin_name, &*sink)
            .await;
    }
}

/// Set up and spawn tasks for a single TCP connection.
async fn spawn_connection(
    stream: TcpStream,
    port_id: PortId,
    public_name: String,
    admin_name: String,
    sink: &dyn PortMessageSink,
) {
    let (read_half, write_half) = stream.into_split();

    let (writer_tx, writer_rx) = mpsc::channel::<Bytes>(1);
    let disconnected = Arc::new(AtomicBool::new(false));

    let tcp_port = TcpPort {
        id: port_id,
        public_name,
        admin_name,
        writer_tx: Some(writer_tx),
        shutdown: Arc::clone(&disconnected),
    };

    // Send the new port to the application. If the sink is closed,
    // just drop the connection.
    let msg = PortMessage::New {
        port: Box::new(tcp_port),
    };
    if sink.send(msg).await.is_err() {
        warn!(?port_id, "listen: sink closed, dropping connection");
        return;
    }

    // Spawn reader and writer tasks.
    let reader_sink = sink.clone_sink();
    let writer_sink = sink.clone_sink();
    let reader_disconnected = Arc::clone(&disconnected);

    tokio::spawn(reader_task(
        read_half,
        port_id,
        reader_sink,
        reader_disconnected,
    ));
    tokio::spawn(writer_task(
        write_half,
        port_id,
        writer_rx,
        writer_sink,
        disconnected,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_to_crlf_no_lf() {
        let data = b"hello world";
        let result = lf_to_crlf(data);
        assert_eq!(&result[..], b"hello world");
    }

    #[test]
    fn lf_to_crlf_single_lf() {
        let data = b"hello\nworld";
        let result = lf_to_crlf(data);
        assert_eq!(&result[..], b"hello\r\nworld");
    }

    #[test]
    fn lf_to_crlf_trailing_lf() {
        let data = b"hello\n";
        let result = lf_to_crlf(data);
        assert_eq!(&result[..], b"hello\r\n");
    }

    #[test]
    fn lf_to_crlf_multiple_lf() {
        let data = b"a\nb\nc\n";
        let result = lf_to_crlf(data);
        assert_eq!(&result[..], b"a\r\nb\r\nc\r\n");
    }

    #[test]
    fn lf_to_crlf_empty() {
        let result = lf_to_crlf(b"");
        assert_eq!(&result[..], b"");
    }
}
