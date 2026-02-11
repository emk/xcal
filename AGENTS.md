# `xcal` retrocomputing chat system

Xcaliber Mark II is a multi-user conference (chat) server written in C over Christmas 1997, reimplementing a beloved 1977 GMAP assembly program from Dartmouth's DCTS time-sharing system. We are building Rust tooling to explore and test the C server's protocol as groundwork for an eventual Rust rewrite.

## Useful files

- `README.md`: Overview and history.
- `docs/`: Newly-generated documentation.
    - `ORIGINAL_CODE_OVERVIEW.md`: What this program is, history, etc.
    - `COMMANDS.md`: Short overview of available commands.
- `plans/`: Implementation plans.
    - `RUST_EXPLORATION_TOOL.md`: Design docs for `xcal-test` protocol exploration tool. 
- `help/*.hf`: Original help for all commands and topics.

### Essential C files

- `cmd.c`: Command dispatch and 40+ command handlers. The largest file.
- `con.c`: Conference lifecycle: the `select()` main loop, message delivery, user join/leave, subconference management.
- `user.c`: Per-user state machine, input polling, message queue operations.
- `xcal.c`: `main()`: argument parsing, signal handling, outer conference loop.

### Essential Rust files

- `xcal_in_the_rust/`: Rust workspace — tooling, maybe someday a port.
    - `crates/`: Rust library crates.
        - `xcal_test_tools/`: Data structures for JSONL protocol testing language, plus support code. Does not include networking support.
    - `tools/`: Rust binary crates.
        - `xcal-test/`: CLI tool that runs JSONL scripts against a live `xcal` server and captures transcripts.
    - `tests/fixtures/`: Sample input and output data for interacting with `xcal`. Captured from original `xcal` using `xcal-test`.
        - `{name}/in.jsonl`: Input JSONL test script (supports multiple named connections).
        - `{name}/{conn}.txt`: Per-connection output transcripts.

## Running `xcal`

This will normally be done by the user, but the command is `xcal -l`. This will start a server that automatically restarts after a conference exists. The server will run on port 2345, using TCP socket with very minimal telnet negotation support (it refuses everything).

## Testing the protocol with `xcal-test`

`xcal-test` runs JSONL scripts against a live `xcal` server, managing multiple named TCP connections and capturing per-connection transcripts. It defaults to port 2456 and `localhost`.

### Running a script

```sh
cd xcal_in_the_rust
cargo run -p xcal-test -- run \
    --script tests/fixtures/hello/in.jsonl \
    --host localhost --port 2456 \
    --save-transcripts tests/fixtures/hello
```

Without `--save-transcripts`, transcripts print to stdout.

### JSONL script format

Each line is a JSON object. Lines with a `"comment"` key are skipped. All other lines must have `"action"` and `"conn"` (the named connection) keys.

| Action | Required fields | Description |
|--------|----------------|-------------|
| `connect` | `conn` | Open a TCP connection to the server. |
| `send` | `conn`, `text` | Send `text` + `\r\n` to the connection. |
| `send_raw` | `conn`, `bytes` | Send raw bytes (no `\r\n` appended). |
| `expect` | `conn`, plus `contains` or `matches` | Read until buffer contains substring (or matches regex). Optional `timeout_ms` (default 5000). |
| `drain` | `conn` | Read until quiet. Optional `quiet_ms` (default 100). |
| `disconnect` | `conn` | Close the connection. |
| `sleep` | `ms` | Sleep for `ms` milliseconds (no `conn` needed). |

### Example script

```jsonl
{"comment": "Connect as master, run who, disconnect"}
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

### Multi-connection scripts

Scripts can manage multiple named connections to simulate multiple users. The first connection to `xcal` becomes the master; subsequent connections are regular users.

```jsonl
{"conn": "master", "action": "connect"}
{"conn": "master", "action": "expect", "contains": "master terminal"}
{"conn": "master", "action": "send", "text": "MasterUser"}
{"conn": "user1", "action": "connect"}
{"conn": "user1", "action": "expect", "contains": "enter your name"}
{"conn": "user1", "action": "send", "text": "RegularUser"}
```

Each connection gets its own transcript file when using `--save-transcripts`.

### Telnet handling

`xcal-test` automatically strips telnet IAC sequences from received data and responds WONT/DONT to all option negotiations. Transcripts contain clean text only.

### Tracing

Set `RUST_LOG` for debug output:

```sh
RUST_LOG=xcal_test=debug cargo run -p xcal-test -- run --script ...
RUST_LOG=xcal_test=trace cargo run -p xcal-test -- run --script ...  # full I/O
```
