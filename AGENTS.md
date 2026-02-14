# `xcal` retrocomputing chat system

Xcaliber Mark II is a multi-user conference (chat) server written in C over Christmas 1997, reimplementing a beloved 1977 GMAP assembly program from Dartmouth's DCTS time-sharing system. We are building Rust tooling to explore and test the C server's protocol as groundwork for an eventual Rust rewrite.

## Useful files

- `README.md`: Overview and history, all the way back to the original Xcaliber in 1977.
- `docs/`: Newly-generated documentation.
    - `DCTS_XCAL_BACKGROUND.md`: How the original XCaliber worked. Includes a timeline and relevant OS details about DCTS.
    - `ORIGINAL_CODE_OVERVIEW.md`: What the XCaliber Mark II program is, history, etc.
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

This will normally be done by the user, but the command is `xcal -l`. This will start a server that automatically restarts after a conference exists. The server will run on port 2456, using TCP socket with very minimal telnet negotation support (it refuses everything).

## Testing the protocol with `xcal-test`

`xcal-test` runs JSONL scripts against a live `xcal` server, managing multiple named TCP connections and capturing per-connection transcripts. It defaults to port 2456 and `localhost`.

### Running a script

```sh
cd xcal_in_the_rust
cargo run -p xcal-test -- run tests/fixtures/hello/in.jsonl
```

This saves per-connection transcripts to the script's parent directory (i.e. `tests/fixtures/hello/master.txt`, etc.). To override the output directory:

```sh
cargo run -p xcal-test -- run tests/fixtures/hello/in.jsonl --transcript-dir /tmp/output
```

To print transcripts to stdout instead of saving:

```sh
cargo run -p xcal-test -- run tests/fixtures/hello/in.jsonl --stdout
```

### Checking transcripts

Re-run a script and diff the live output against previously-saved transcripts:

```sh
cd xcal_in_the_rust
cargo run -p xcal-test -- check tests/fixtures/hello/in.jsonl
```

This reads the expected `.txt` files from the script's parent directory, runs the script against the server, and shows a colored diff for any mismatches. Exit code is 0 if all transcripts match, 1 otherwise.

### Normalization

Some transcript content (timestamps, dates) varies between runs. Place a `normalize.toml` in the fixture directory to define regex replacement rules applied to both expected and actual transcripts before comparison:

```toml
[[rules]]
pattern = '\d{1,2}:\d{2}:\d{2} [ap]\.m\.'
replace = "HH:MM:SS xm"

[[rules]]
pattern = '\d{1,2}/\d{1,2}/\d{2,4}'
replace = "M/D/YY"
```

The normalizer is loaded automatically by `check` from `normalize.toml` in the transcript directory. Use `--normalize-rules` to specify an alternate path.

### Checking all fixtures

Run all fixtures in a directory and report aggregate results:

```sh
cd xcal_in_the_rust
cargo run -p xcal-test -- check-all tests/fixtures/
```

This discovers all subdirectories containing `in.jsonl`, runs each one sequentially, and prints a summary. Exit code is 0 if all pass, 1 if any fail.

To skip a fixture, place a `SKIP.md` file in its directory. Skipped fixtures are reported but don't count as failures.

### JSONL script format

Each line is a JSON object. Lines with a `"comment"` key are skipped. All other lines must have `"action"` and `"conn"` (the named connection) keys.

| Action | Required fields | Description |
|--------|----------------|-------------|
| `connect` | `conn` | Open a TCP connection to the server. |
| `send` | `conn`, `text` | Send `text` + `\r\n` to the connection. |
| `send_raw` | `conn`, `bytes` | Send raw bytes (no `\r\n` appended). |
| `send_bytes` | `conn`, `hex` | Send hex-encoded bytes (e.g. `"ff f3"` for telnet IAC BRK). |
| `expect` | `conn`, plus `contains` or `matches` | Read until buffer contains substring (or matches regex). Optional `timeout_ms` (default 5000). |
| `drain` | `conn` | Read until quiet. Optional `quiet_ms` (default 100). |
| `disconnect` | `conn` | Close the connection. |
| `sleep` | `ms` | Sleep for `ms` milliseconds (no `conn` needed). |

**Important: `expect` clears the receive buffer** after a successful
match. If the server sends multiple messages to the same connection in
a single TCP batch (e.g. "Done" followed by "You have been passed"),
the first `expect` will consume and discard the entire buffer —
including text that hasn't been matched yet. To handle this, expect
the *last* distinctive string in the batch rather than chaining
multiple expects on the same connection. When in doubt, use a single
`expect` for the latest-arriving content and let earlier messages pass
through implicitly.

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
{"conn": "master", "action": "send", "text": "Explorer"}
{"conn": "user1", "action": "connect"}
{"conn": "user1", "action": "expect", "contains": "enter your name"}
{"conn": "user1", "action": "send", "text": "Wanderer"}
```

Each connection gets its own transcript file when saving transcripts.

### Telnet handling

`xcal-test` automatically strips telnet IAC sequences from received data and responds WONT/DONT to all option negotiations. Transcripts contain clean text only.

### Tracing

Set `RUST_LOG` for debug output:

```sh
RUST_LOG=xcal_test=debug cargo run -p xcal-test -- run ...
RUST_LOG=xcal_test=trace cargo run -p xcal-test -- run ...  # full I/O
```

## Claude Code sandbox

Within the project root directory, you have broad privileges to work inside the sandbox without user confirmation. To prevent tasks being halted for user permission, you will often want to `cd` to the root of the project, and to prefer your built-in search and read tools. Trying to construct absolute paths or paths using `~/` may trigger unnecessary approval prompts.

- BAD: `~/w/src/xcal/foo`
- GOOD: cd to project root, use `foo`

## A note about DCTS I/O

Many DCTS terminal applications appear to have used line-oriented I/O modes, with single-line editing done on the terminal. This allowed entire lines to be sent as a single network message, making more efficient use of the mainframe CPU. This assumption seems to be largely present in Xcaliber Mark II.

As part of this, when the user was actively editing text, then the `xcal` server would refrain from sending any messages. This doesn't happen _immediately_; it happens after "tell 1\r\n". So "tell 1; message\r\n" doesn't automatically pause output.

Around 93-94, there was still a Mac extension "KSP" that implemented the "Kiewit Stream Protocol" over AppleTalk. Combined with a special Mac terminal emulator, this could be used to access the tiny handful of publicly remaining DCTS applications (mostly course registration). However, reconstructing the finer details of things like the Kiewit Stream Protocol (or any earlier DTSS stream protocols), is probably not for the faint of heart at this late date. The moral of the story: Do not assume Unix-era terminal assumptions apply to this project!

Interesting historical context:

- [DCTS era](https://www.cs.cornell.edu/wya/AcademicComputing/text/netcases.html)
- [Dartmouth Mac era](https://www.cs.cornell.edu/wya/AcademicComputing/text/macdartmouth.html)
- [Historical information on DTSS/DCTS](https://dtss.dartmouth.edu/) (There's an emulator, but it's for a much earlier version of DTSS.)
- [An outside intruder dialing into Xcaliber in the 80s](https://dtss.dartmouth.edu/kiewit-dtss.php) (Preserved on the DTSS history site itself! An interesting historic crossover with BBS "files" culture.)
