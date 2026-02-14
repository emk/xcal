//! In-process fixture replay runner for [`Conference`].
//!
//! Replays JSONL scripts against `Conference` + `TestPort` directly,
//! producing per-connection transcripts that can be compared against
//! saved `.txt` files from `xcal-test` TCP runs.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use bytes::BytesMut;
use xcal_test_tools::{
    diff::diff_transcript, normalize::Normalizer, parse_script, Action,
    ScriptLine,
};

use super::{test_harness::TestPort, Conference};

/// Shared output buffer between a [`TestPort`] and the fixture runner.
type SharedBuffer = Arc<Mutex<Vec<u8>>>;

/// An in-process connection to the conference.
struct InProcessConnection {
    /// The port ID assigned by the conference.
    port_id: dcts::ports::PortId,
    /// Shared buffer where TestPort writes output.
    buffer: SharedBuffer,
    /// Unread output accumulated from flushes (like TCP recv_buf).
    recv_buf: Vec<u8>,
    /// Full chronological transcript.
    transcript: String,
}

/// Replays a JSONL fixture script against a [`Conference`] in-process.
struct FixtureRunner {
    conference: Conference,
    connections: HashMap<String, InProcessConnection>,
    /// Transcripts saved from disconnected connections.
    finished_transcripts: Vec<(String, String)>,
    conference_ended: bool,
}

impl FixtureRunner {
    fn new() -> Self {
        Self {
            conference: Conference::new(),
            connections: HashMap::new(),
            finished_transcripts: Vec::new(),
            conference_ended: false,
        }
    }

    /// Run a parsed script, consuming actions in order.
    fn run(
        &mut self,
        lines: &[ScriptLine],
    ) -> Result<HashMap<String, String>, String> {
        for (i, line) in lines.iter().enumerate() {
            let line_num = i.saturating_add(1);
            match line {
                ScriptLine::Comment { .. } => {}
                ScriptLine::Action(action) => {
                    self.dispatch(action, line_num)?;
                }
            }
        }

        // Collect transcripts: first from disconnected connections, then
        // from any still open.
        let mut transcripts = HashMap::new();
        for (name, transcript) in &self.finished_transcripts {
            transcripts.insert(name.clone(), transcript.clone());
        }
        for (name, conn) in &self.connections {
            transcripts.insert(name.clone(), conn.transcript.clone());
        }
        Ok(transcripts)
    }

    fn dispatch(
        &mut self,
        action: &Action,
        line_num: usize,
    ) -> Result<(), String> {
        match action {
            Action::Connect { conn } => self.handle_connect(conn, line_num),
            Action::Send { conn, text } => {
                self.handle_send(conn, text, line_num)
            }
            Action::SendRaw { conn, bytes } => {
                self.handle_send_raw(conn, bytes, line_num)
            }
            Action::SendBytes { conn, hex } => {
                self.handle_send_bytes(conn, hex, line_num)
            }
            Action::Expect {
                conn,
                contains,
                matches,
                ..
            } => self.handle_expect(
                conn,
                contains.as_deref(),
                matches.as_deref(),
                line_num,
            ),
            Action::Drain { conn, .. } => self.handle_drain(conn, line_num),
            Action::Disconnect { conn } => {
                self.handle_disconnect(conn, line_num)
            }
            Action::Sleep { .. } => Ok(()), // No-op in synchronous replay.
        }
    }

    fn handle_connect(
        &mut self,
        name: &str,
        line_num: usize,
    ) -> Result<(), String> {
        let (port, buffer) = TestPort::new(name);
        let port_id = dcts::ports::Port::id(&port);

        self.conference
            .handle_new(Box::new(port))
            .map_err(|e| format!("line {line_num}: handle_new failed: {e}"))?;

        self.connections.insert(
            name.to_string(),
            InProcessConnection {
                port_id,
                buffer,
                recv_buf: Vec::new(),
                transcript: String::new(),
            },
        );

        Ok(())
    }

    fn handle_send(
        &mut self,
        name: &str,
        text: &str,
        line_num: usize,
    ) -> Result<(), String> {
        let conn = self.connections.get_mut(name).ok_or_else(|| {
            format!("line {line_num}: no connection named {name:?}")
        })?;
        let port_id = conn.port_id;

        // Record the sent text in the transcript (matching TCP's send_line
        // which records text\r\n — we use \n and strip \r from expected).
        conn.transcript.push_str(text);
        conn.transcript.push('\n');

        // Process input through conference.
        let input = BytesMut::from(text.as_bytes());
        match self.conference.handle_input(port_id, &input) {
            Ok(true) => {
                self.conference_ended = true;
            }
            Ok(false) => {}
            Err(e) => {
                return Err(format!(
                    "line {line_num}: handle_input failed: {e}"
                ));
            }
        }

        Ok(())
    }

    fn handle_send_raw(
        &mut self,
        name: &str,
        bytes: &str,
        line_num: usize,
    ) -> Result<(), String> {
        let conn = self.connections.get_mut(name).ok_or_else(|| {
            format!("line {line_num}: no connection named {name:?}")
        })?;
        let port_id = conn.port_id;

        // Record raw bytes in transcript.
        conn.transcript.push_str(bytes);

        // Process as input (no \r\n appended).
        let input = BytesMut::from(bytes.as_bytes());
        match self.conference.handle_input(port_id, &input) {
            Ok(true) => {
                self.conference_ended = true;
            }
            Ok(false) => {}
            Err(e) => {
                return Err(format!(
                    "line {line_num}: handle_input (send_raw) failed: {e}"
                ));
            }
        }

        Ok(())
    }

    fn handle_send_bytes(
        &mut self,
        _name: &str,
        _hex: &str,
        line_num: usize,
    ) -> Result<(), String> {
        // Only cmd_rb_ab uses send_bytes, and it's SKIP'd.
        Err(format!(
            "line {line_num}: send_bytes not supported in in-process replay"
        ))
    }

    fn handle_expect(
        &mut self,
        name: &str,
        contains: Option<&str>,
        matches: Option<&str>,
        line_num: usize,
    ) -> Result<(), String> {
        if matches.is_some() {
            return Err(format!(
                "line {line_num}: 'matches' (regex) not yet implemented"
            ));
        }

        let conn = self.connections.get_mut(name).ok_or_else(|| {
            format!("line {line_num}: no connection named {name:?}")
        })?;

        // Flush SharedBuffer → recv_buf.
        flush_buffer(conn);

        // Check pattern.
        if let Some(pattern) = contains {
            let buf_str = String::from_utf8_lossy(&conn.recv_buf);
            if !buf_str.contains(pattern) {
                return Err(format!(
                    "line {line_num}: expect on {name:?}: \
                     pattern {pattern:?} not found in buffer:\n{buf_str}"
                ));
            }
        }

        // Append recv_buf to transcript and clear.
        let text = String::from_utf8_lossy(&conn.recv_buf).into_owned();
        conn.transcript.push_str(&text);
        conn.recv_buf.clear();

        Ok(())
    }

    fn handle_drain(
        &mut self,
        name: &str,
        line_num: usize,
    ) -> Result<(), String> {
        let conn = self.connections.get_mut(name).ok_or_else(|| {
            format!("line {line_num}: no connection named {name:?}")
        })?;

        // Flush SharedBuffer → recv_buf.
        flush_buffer(conn);

        // Append recv_buf to transcript and clear.
        let text = String::from_utf8_lossy(&conn.recv_buf).into_owned();
        conn.transcript.push_str(&text);
        conn.recv_buf.clear();

        Ok(())
    }

    fn handle_disconnect(
        &mut self,
        name: &str,
        line_num: usize,
    ) -> Result<(), String> {
        let mut conn = self.connections.remove(name).ok_or_else(|| {
            format!("line {line_num}: no connection named {name:?}")
        })?;

        // Flush any remaining output.
        flush_buffer(&mut conn);
        let text = String::from_utf8_lossy(&conn.recv_buf).into_owned();
        conn.transcript.push_str(&text);

        // Notify conference of disconnect (unless conference already ended).
        if !self.conference_ended {
            let _ = self.conference.handle_disconnected(conn.port_id);
        }

        // Save the finished transcript.
        self.finished_transcripts
            .push((name.to_string(), conn.transcript));

        Ok(())
    }
}

/// Drain the SharedBuffer contents into recv_buf.
fn flush_buffer(conn: &mut InProcessConnection) {
    #[allow(clippy::expect_used)]
    let mut buf = conn.buffer.lock().expect("lock poisoned");
    conn.recv_buf.extend_from_slice(&buf);
    buf.clear();
}

/// Read all `.txt` files in a directory, keyed by file stem.
fn find_transcript_files(
    dir: &Path,
) -> Result<HashMap<String, String>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?;
    let mut transcripts = HashMap::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    format!("failed to read {}: {e}", path.display())
                })?;
                transcripts.insert(stem.to_string(), content);
            }
        }
    }
    Ok(transcripts)
}

/// Run a fixture directory against the in-process Conference and compare
/// transcripts.
pub(crate) fn run_fixture(fixture_dir: &Path) -> Result<(), String> {
    // Load and parse the script.
    let script_path = fixture_dir.join("in.jsonl");
    let content = std::fs::read_to_string(&script_path).map_err(|e| {
        format!("failed to read {}: {e}", script_path.display())
    })?;
    let lines = parse_script(&content)
        .map_err(|e| format!("failed to parse script: {e}"))?;

    // Load normalizer.
    let norm_path = fixture_dir.join("normalize.toml");
    let normalizer = Normalizer::from_path(&norm_path)
        .map_err(|e| format!("failed to load normalizer: {e}"))?;

    // Load expected transcripts.
    let expected = find_transcript_files(fixture_dir)?;
    if expected.is_empty() {
        return Err(format!(
            "no .txt transcript files found in {}",
            fixture_dir.display()
        ));
    }

    // Run script.
    let mut runner = FixtureRunner::new();
    let actual = runner.run(&lines)?;

    // Compare transcripts.
    let expected_keys: std::collections::BTreeSet<_> =
        expected.keys().collect();
    let actual_keys: std::collections::BTreeSet<_> = actual.keys().collect();

    if expected_keys != actual_keys {
        let missing: Vec<_> = expected_keys.difference(&actual_keys).collect();
        let extra: Vec<_> = actual_keys.difference(&expected_keys).collect();
        let mut msg = String::from("connection set mismatch:");
        if !missing.is_empty() {
            msg.push_str(&format!(" missing={missing:?}"));
        }
        if !extra.is_empty() {
            msg.push_str(&format!(" extra={extra:?}"));
        }
        return Err(msg);
    }

    // Diff each connection.
    let mut failures = Vec::new();
    let mut conn_names: Vec<_> = expected.keys().collect();
    conn_names.sort();

    for conn in conn_names {
        // Strip \r from expected (TCP transcripts use \r\n, in-process uses \n).
        let exp_stripped = expected[conn].replace('\r', "");
        let exp_text = normalizer.apply(&exp_stripped);
        let act_text = normalizer.apply(&actual[conn]);

        let result = diff_transcript(conn, &exp_text, &act_text);
        if !result.matches {
            failures.push(result.diff_output);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}
