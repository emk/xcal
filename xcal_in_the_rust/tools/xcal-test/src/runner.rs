use std::collections::HashMap;

use miette::{miette, Result};
use tracing::{debug, instrument};
use xcal_test_tools::{Action, ScriptLine};

use crate::connection::Connection;

const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Execute a script against a server, returning per-connection transcripts.
pub struct Runner {
    host: String,
    port: u16,
    connections: HashMap<String, Connection>,
    transcripts: Vec<(String, String)>,
}

impl Runner {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            connections: HashMap::new(),
            transcripts: Vec::new(),
        }
    }

    /// Run a full script, returning (conn_name, transcript) pairs.
    pub async fn run(
        &mut self,
        lines: &[ScriptLine],
    ) -> Result<Vec<(String, String)>> {
        for (i, line) in lines.iter().enumerate() {
            match line {
                ScriptLine::Comment { comment } => {
                    debug!("# {comment}");
                }
                ScriptLine::Action(action) => {
                    self.dispatch(action, i + 1).await?;
                }
            }
        }

        // Collect transcripts from any connections still open.
        for (name, conn) in &self.connections {
            self.transcripts
                .push((name.clone(), conn.transcript().to_string()));
        }
        self.transcripts.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(std::mem::take(&mut self.transcripts))
    }

    async fn dispatch(
        &mut self,
        action: &Action,
        line_num: usize,
    ) -> Result<()> {
        match action {
            Action::Connect { conn } => {
                self.handle_connect(conn, line_num).await
            }
            Action::Send { conn, text } => {
                self.handle_send(conn, text, line_num).await
            }
            Action::SendRaw { conn, bytes } => {
                self.handle_send_raw(conn, bytes, line_num).await
            }
            Action::Expect {
                conn,
                contains,
                matches,
                timeout_ms,
            } => {
                self.handle_expect(
                    conn,
                    contains.as_deref(),
                    matches.as_deref(),
                    *timeout_ms,
                    line_num,
                )
                .await
            }
            Action::Drain { conn, quiet_ms } => {
                self.handle_drain(conn, *quiet_ms, line_num).await
            }
            Action::Disconnect { conn } => {
                self.handle_disconnect(conn, line_num).await
            }
            Action::Sleep { ms } => {
                debug!("[line {line_num}] sleep {ms}ms");
                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                Ok(())
            }
        }
    }

    #[instrument(level = "debug", skip_all, fields(conn = %name))]
    async fn handle_connect(
        &mut self,
        name: &str,
        line_num: usize,
    ) -> Result<()> {
        debug!(
            "[line {line_num}] connect {name} -> {}:{}",
            self.host, self.port
        );
        let conn = Connection::connect(&self.host, self.port).await?;
        self.connections.insert(name.to_string(), conn);
        Ok(())
    }

    #[instrument(level = "trace", skip_all, fields(conn = %name, %text))]
    async fn handle_send(
        &mut self,
        name: &str,
        text: &str,
        line_num: usize,
    ) -> Result<()> {
        debug!("[line {line_num}] send {name}: {text:?}");
        let conn = self.connections.get_mut(name).ok_or_else(|| {
            miette!("line {line_num}: no connection named {name:?}")
        })?;
        conn.send_line(text).await
    }

    #[instrument(level = "trace", skip_all, fields(conn = %name))]
    async fn handle_send_raw(
        &mut self,
        name: &str,
        bytes: &str,
        line_num: usize,
    ) -> Result<()> {
        debug!("[line {line_num}] send_raw {name}: {bytes:?}");
        let conn = self.connections.get_mut(name).ok_or_else(|| {
            miette!("line {line_num}: no connection named {name:?}")
        })?;
        conn.send(bytes.as_bytes()).await
    }

    #[instrument(level = "debug", skip_all, fields(conn = %name))]
    async fn handle_expect(
        &mut self,
        name: &str,
        contains: Option<&str>,
        _matches: Option<&str>,
        timeout_ms: Option<u64>,
        line_num: usize,
    ) -> Result<()> {
        let timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);

        if let Some(pattern) = contains {
            debug!("[line {line_num}] expect {name}: contains {pattern:?} (timeout {timeout}ms)");
            let conn = self.connections.get_mut(name).ok_or_else(|| {
                miette!("line {line_num}: no connection named {name:?}")
            })?;
            conn.recv_until_contains(pattern, timeout).await?;
            conn.clear_recv_buf();
        } else if _matches.is_some() {
            return Err(miette!(
                "line {line_num}: 'matches' (regex) not yet implemented"
            ));
        } else {
            return Err(miette!(
                "line {line_num}: expect requires 'contains' or 'matches'"
            ));
        }

        Ok(())
    }

    #[instrument(level = "debug", skip_all, fields(conn = %name))]
    async fn handle_drain(
        &mut self,
        name: &str,
        quiet_ms: u64,
        line_num: usize,
    ) -> Result<()> {
        debug!("[line {line_num}] drain {name}: quiet_ms={quiet_ms}");
        let conn = self.connections.get_mut(name).ok_or_else(|| {
            miette!("line {line_num}: no connection named {name:?}")
        })?;
        conn.recv_until_quiet(quiet_ms).await?;
        conn.clear_recv_buf();
        Ok(())
    }

    #[instrument(level = "debug", skip_all, fields(conn = %name))]
    async fn handle_disconnect(
        &mut self,
        name: &str,
        line_num: usize,
    ) -> Result<()> {
        debug!("[line {line_num}] disconnect {name}");
        if let Some(conn) = self.connections.remove(name) {
            self.transcripts
                .push((name.to_string(), conn.transcript().to_string()));
            conn.close().await?;
        }
        Ok(())
    }
}
