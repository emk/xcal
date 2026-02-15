use std::time::Duration;

use miette::{miette, IntoDiagnostic, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
use tracing::{debug, instrument, trace};

use crate::telnet;

/// A TCP connection with telnet filtering and transcript recording.
pub struct Connection {
    stream: TcpStream,
    /// Accumulated received data (after telnet stripping), for pattern matching.
    recv_buf: Vec<u8>,
    /// Full transcript of received data (after telnet stripping).
    transcript: String,
}

impl Connection {
    /// Open a new TCP connection to the given host and port.
    #[instrument(level = "debug", skip_all, fields(%host, %port))]
    pub async fn connect(host: &str, port: u16) -> Result<Self> {
        let addr = format!("{host}:{port}");
        debug!("connecting to {addr}");
        let stream = TcpStream::connect(&addr).await.into_diagnostic()?;
        Ok(Self {
            stream,
            recv_buf: Vec::new(),
            transcript: String::new(),
        })
    }

    /// Send bytes over the connection.
    #[instrument(level = "trace", skip_all)]
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        let text = String::from_utf8_lossy(data);
        self.transcript.push_str(&text);
        self.stream.write_all(data).await.into_diagnostic()
    }

    /// Send a text line (appends \r\n).
    pub async fn send_line(&mut self, text: &str) -> Result<()> {
        let mut data = text.as_bytes().to_vec();
        data.extend_from_slice(b"\r\n");
        self.send(&data).await
    }

    /// Read data until the connection is quiet for `quiet_ms` milliseconds.
    /// Returns the new data received during this call.
    #[instrument(level = "debug", skip_all, fields(%quiet_ms))]
    pub async fn recv_until_quiet(&mut self, quiet_ms: u64) -> Result<Vec<u8>> {
        let quiet = Duration::from_millis(quiet_ms);
        let mut buf = [0u8; 4096];
        let mut received = Vec::new();

        loop {
            match timeout(quiet, self.stream.read(&mut buf)).await {
                Ok(Ok(0)) => break, // EOF
                Ok(Ok(n)) => {
                    let raw = &buf[..n];
                    let output = telnet::process(raw);

                    // Send any telnet responses
                    if !output.responses.is_empty() {
                        self.stream
                            .write_all(&output.responses)
                            .await
                            .into_diagnostic()?;
                    }

                    let text = String::from_utf8_lossy(&output.data);
                    trace!("recv: {text:?}");
                    self.recv_buf.extend_from_slice(&output.data);
                    self.transcript.push_str(&text);
                    received.extend_from_slice(&output.data);
                }
                Ok(Err(e)) => return Err(miette!("read error: {e}")),
                Err(_) => break, // Timeout — quiet period reached
            }
        }

        Ok(received)
    }

    /// Read data until the receive buffer contains the given substring.
    /// Times out after `timeout_ms` milliseconds (default 5000).
    #[instrument(level = "debug", skip_all, fields(%pattern, %timeout_ms))]
    pub async fn recv_until_contains(
        &mut self,
        pattern: &str,
        timeout_ms: u64,
    ) -> Result<()> {
        let deadline = Duration::from_millis(timeout_ms);
        let start = tokio::time::Instant::now();
        let mut buf = [0u8; 4096];

        // Check if we already have the pattern
        let buf_str = String::from_utf8_lossy(&self.recv_buf);
        if buf_str.contains(pattern) {
            debug!("pattern already in buffer");
            return Ok(());
        }

        loop {
            let remaining = deadline.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                let buf_str = String::from_utf8_lossy(&self.recv_buf);
                return Err(miette!(
                    "timeout waiting for pattern {pattern:?} after {timeout_ms}ms\nbuffer contents: {buf_str:?}"
                ));
            }

            match timeout(remaining, self.stream.read(&mut buf)).await {
                Ok(Ok(0)) => {
                    let buf_str = String::from_utf8_lossy(&self.recv_buf);
                    return Err(miette!(
                        "connection closed while waiting for {pattern:?}\nbuffer contents: {buf_str:?}"
                    ));
                }
                Ok(Ok(n)) => {
                    let raw = &buf[..n];
                    let output = telnet::process(raw);

                    if !output.responses.is_empty() {
                        self.stream
                            .write_all(&output.responses)
                            .await
                            .into_diagnostic()?;
                    }

                    let text = String::from_utf8_lossy(&output.data);
                    trace!("recv: {text:?}");
                    self.recv_buf.extend_from_slice(&output.data);
                    self.transcript.push_str(&text);

                    let buf_str = String::from_utf8_lossy(&self.recv_buf);
                    if buf_str.contains(pattern) {
                        debug!("found pattern");
                        return Ok(());
                    }
                }
                Ok(Err(e)) => return Err(miette!("read error: {e}")),
                Err(_) => {
                    let buf_str = String::from_utf8_lossy(&self.recv_buf);
                    return Err(miette!(
                        "timeout waiting for pattern {pattern:?} after {timeout_ms}ms\nbuffer contents: {buf_str:?}"
                    ));
                }
            }
        }
    }

    /// Clear the receive buffer (typically after a successful expect).
    pub fn clear_recv_buf(&mut self) {
        self.recv_buf.clear();
    }

    /// Get the full transcript of this connection.
    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    /// Close the connection.
    pub async fn close(mut self) -> Result<()> {
        self.stream.shutdown().await.into_diagnostic()
    }
}
