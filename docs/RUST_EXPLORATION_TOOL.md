# Plan: Rust Exploration Tool (Tracer Bullet)

## Context

We're building tooling to explore and test the original Xcaliber Mark II C
server, as groundwork for an eventual Rust rewrite. The first deliverable is a
Rust CLI tool that reads a JSONL script, executes it against a TCP server (the
C implementation), and saves per-connection transcripts. This also establishes
the Rust workspace that will later house the rewrite itself.

## Workspace structure

```
xcal_rust/
  Cargo.toml                          # workspace root
  crates/
    xcal_test_tools/
      Cargo.toml
      src/lib.rs                      # serde structs for JSONL script format
                                      # + shared test infrastructure
  tools/
    xcal-test/
      Cargo.toml
      src/main.rs                     # CLI: run JSONL scripts, save transcripts
      src/telnet.rs                   # minimal telnet byte stripping
      src/runner.rs                   # script execution engine
      src/connection.rs               # manages one TCP connection + recv buffer
  tests/
    fixtures/
      hello_in.jsonl                  # first tracer bullet script
```

Created under `/home/emk/w/src/xcal/xcal_rust/`.


## Crate: `xcal_test_tools` (library)

**Path:** `crates/xcal_test_tools/`

Serde structs representing the JSONL script format from our testing strategy
doc. This is a separate library crate so it can be shared between the CLI tool
(`xcal-test`) and the eventual in-process test harness for the conference
rewrite.

Contains only the data types and any shared test helpers — no TCP/IP, no
async runtime.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Connect { conn: String },
    Send { conn: String, text: String },
    SendRaw { conn: String, bytes: String },
    Expect {
        conn: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        contains: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        matches: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    Drain {
        conn: String,
        #[serde(default = "default_quiet_ms")]
        quiet_ms: u64,
    },
    Disconnect { conn: String },
    Sleep { ms: u64 },
}

// Comments are a separate variant since they lack "action"
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScriptLine {
    Comment { comment: String },
    Action(Action),
}

fn default_quiet_ms() -> u64 { 100 }
```

**Dependencies:** `serde`, `serde_json`


## Crate: `xcal-test` (binary)

**Path:** `tools/xcal-test/`

All TCP/IP and telnet protocol handling lives here, not in the shared library.

### CLI interface (using `clap`)

```
xcal-test run --script <path.jsonl> --host <host> -p/--port <port> \
              [--save-transcripts <dir>]
```

Default port is **2456** (matching the C server's `DEFAULT_PORT`). Default
host is `localhost`.

`xcal-test` is purely a client — it never starts or manages server processes.
The server must already be running.

For the tracer bullet, only `run` mode. Interactive `explore` mode comes later.

### Modules

**`main.rs`** — Argument parsing, loads script, calls runner, writes output.

**`runner.rs`** — The script execution engine.
- Owns a `HashMap<String, Connection>` of named connections.
- Reads `ScriptLine`s, dispatches each action:
  - `Connect` → open TCP connection, store in map
  - `Send` → write `text + \r\n` to connection's socket
  - `SendRaw` → write raw bytes
  - `Expect` → loop: read with timeout, check accumulated buffer for match
  - `Drain` → loop: read with short timeout, stop when quiet
  - `Disconnect` → close socket
  - `Sleep` → `tokio::time::sleep`
  - `Comment` → skip
- After all actions, return per-connection transcripts.

**`connection.rs`** — Wraps a single TCP connection.
- `tokio::net::TcpStream`
- Accumulated receive buffer (`Vec<u8>`)
- Transcript buffer (`String`) — everything received, after telnet stripping
- Methods:
  - `connect(host, port) -> Connection`
  - `send(bytes)`
  - `recv_until_quiet(quiet_ms) -> bytes received`
  - `recv_until_match(pattern, timeout_ms) -> Result<(), timeout>`
  - `transcript() -> &str`
  - `close()`

**`telnet.rs`** — Minimal telnet handling.
- Strip IAC sequences from received bytes before appending to transcript.
- When we see `IAC WILL <opt>` → send `IAC DONT <opt>`
- When we see `IAC DO <opt>` → send `IAC WONT <opt>`
- Everything else (IAC NOP, IAC BRK, subneg, etc.) → strip silently
- This is ~50 lines. We're just being a polite telnet client, not implementing
  the protocol.

Constants needed (from `telnet.h`):
```
IAC  = 0xFF
WILL = 0xFB
WONT = 0xFC
DO   = 0xFD
DONT = 0xFE
```

**Dependencies:** `tokio` (net, time, io, macros, rt-multi-thread), `clap`,
`xcal_test_tools`, `miette`, `tracing`, `tracing-subscriber`

### Tracing

Set up `tracing` with `tracing-subscriber` (with `fmt` and `EnvFilter`).
Use `#[instrument]` on everything that directly or indirectly does async net
operations:

```rust
#[instrument(level = "debug", skip_all, fields(conn = %name))]
async fn handle_connect(&mut self, name: &str) -> Result<()> { ... }

#[instrument(level = "trace", skip_all, fields(conn = %name, text))]
async fn handle_send(&mut self, name: &str, text: &str) -> Result<()> { ... }

#[instrument(level = "debug", skip_all, fields(conn = %name))]
async fn handle_expect(&mut self, name: &str, ...) -> Result<()> { ... }
```

Default `RUST_LOG` (set if env var is absent):
`xcal_test=info,xcal_test_tools=info,warn`

Override with `RUST_LOG=xcal_test=debug` or `RUST_LOG=xcal_test=trace` for
full I/O visibility.


## Tracer bullet fixture

**`tests/fixtures/hello_in.jsonl`:**

```jsonl
{"comment": "Tracer bullet: connect as master, run who, say hi to ourselves, disconnect"}
{"conn": "master", "action": "connect"}
{"conn": "master", "action": "expect", "contains": "master terminal"}
{"conn": "master", "action": "send", "text": "Explorer"}
{"conn": "master", "action": "expect", "contains": "Enter command"}
{"conn": "master", "action": "send", "text": "who"}
{"conn": "master", "action": "expect", "contains": "Explorer"}
{"conn": "master", "action": "send", "text": "bye"}
{"conn": "master", "action": "drain", "quiet_ms": 100}
{"conn": "master", "action": "disconnect"}
```

The first connection is the master (gets `mwelc`: "Hello.  You are the master
terminal.\r\nPlease enter your name. "). After login, gets the intro message.
`who` shows our own status line. `bye` ends the conference.

Running:

```
cargo run -p xcal-test -- run \
    --script tests/fixtures/hello_in.jsonl \
    --host localhost --port 2456 \
    --save-transcripts tests/fixtures/hello
```

Saves `tests/fixtures/hello/master.txt` with the complete transcript.


## Implementation order

1. **Workspace Cargo.toml** — define workspace members
2. **`xcal_test_tools`** — serde types, unit tests for round-trip serialization
3. **`xcal-test/telnet.rs`** — IAC stripping + response, unit tested with
   byte sequences
4. **`xcal-test/connection.rs`** — TCP wrapper with telnet filtering and
   transcript accumulation
5. **`xcal-test/runner.rs`** — script execution engine
6. **`xcal-test/main.rs`** — CLI with clap, wire it all together
7. **`tests/fixtures/hello_in.jsonl`** — the tracer bullet fixture
8. **Test against C server** — compile and run the C server on port 2456,
   execute the fixture, verify transcript looks right


## Verification

1. `cargo build --workspace` succeeds with no warnings
2. `cargo test --workspace` passes (serde round-trip, telnet stripping)
3. Ensure C server is already running on port 2456 (`./xcal -p 2456`)
4. Run tracer bullet:
   ```
   cargo run -p xcal-test -- run \
       --script tests/fixtures/hello_in.jsonl \
       --host localhost --port 2456 \
       --save-transcripts tests/fixtures/hello
   ```
5. `cat tests/fixtures/hello/master.txt` shows the expected conversation:
   welcome → name prompt → intro → who output → goodbye
