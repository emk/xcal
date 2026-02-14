# Xcaliber Mark II — Testing Strategy

## Core idea

We have a working C implementation that speaks a line-oriented text protocol
over TCP. The Rust rewrite exposes the same TCP interface for its core
conference logic. We drive both servers with identical scripted scenarios and
diff the per-connection transcripts.

The web frontend (WebSocket/LiveView) is a separate layer built on top of the
tested core. Get the TCP behavior right first.


## Three levels of testing

### 1. In-process unit tests (primary)

The conference logic is a pure synchronous state machine — "user N sent
line X" in, output bytes out. No sockets, no async runtime, no timeouts.

A `TestConference` harness wraps `Conference` with a fake `TestPort`
implementation that captures output in memory. This gives a simple API for
writing focused command-level tests:

```rust
#[test]
fn who_shows_master_with_marker() {
    let mut tc = TestConference::new();
    let master = tc.connect("Explorer");
    tc.input(master, "who");
    let out = tc.output(master);
    assert!(out.contains("=>"));
    assert!(out.contains("Explorer"));
}

#[test]
fn bye_sends_terminated() {
    let mut tc = TestConference::new();
    let master = tc.connect("Explorer");
    tc.input(master, "bye");
    assert!(tc.output(master).to_lowercase().contains("terminated"));
}
```

The `TestConference` interface (defined in `conferences/test_harness.rs`,
`pub(crate)` so handler test modules can use it):

- `connect(name) -> UserId` — create a `TestPort`, call `handle_new`,
  send the name to log in, clear the welcome output
- `input(user, line)` — feed a line of input
- `output(user) -> String` — drain and return accumulated output
- `drain(user)` — discard accumulated output

This is where the bulk of the behavioral testing lives. It's fast (thousands
of tests in milliseconds), deterministic (no timing dependencies), and tests
the actual logic without any I/O concerns.


### 2. In-process fixture replay (planned)

We have 33 JSONL fixture scripts already captured from the C server,
covering commands, subconferences, tell, ignore, and most edge cases.
These are authoritative baselines — new fixtures should rarely be needed.

This level parses those existing fixtures and replays them against
`Conference` + `TestPort` directly, comparing output against the saved
`.txt` transcripts. This reuses the same fixtures for both TCP integration
and in-process regression testing.

In this mode, `expect` becomes "assert output contains X" and `drain` is a
no-op (everything is synchronous). Normalization rules from `normalize.toml`
are applied before comparison, same as `xcal-test check`.

This provides confidence that the Rust conference matches the C server's
behavior without needing a running server or TCP connections.


### 3. TCP integration tests via `xcal-test` (secondary)

For end-to-end confidence against a running server. Uses the JSONL script
format with real TCP connections.

The fixture baselines have already been captured from the C server
(`xcal-test run`). Going forward, this level is mainly used to verify
the Rust server against those baselines (`xcal-test check`). New fixtures
are only needed for behaviors not already covered.

See "Tooling" below for details.


## Timeouts

Both the C server and any rewrite are pure CPU-bound state machines with no
disk I/O, database calls, or external service dependencies. A round trip
through the conference logic is microseconds.

For TCP integration tests:
- **`drain` quiet timeout:** 100ms default. If the server hasn't sent
  anything for 100ms, it's done.
- **`expect` timeout:** 5000ms default (generous to avoid flakes on slow
  machines). If the expected output hasn't arrived, something is wrong.
- **No retry loops or backoff.** If it doesn't work quickly, it's a bug.

For in-process tests (levels 1 and 2): no timeouts at all. Everything is
synchronous.


## C server and Rust server are shared resources

The C server (`xcal -l` on port 2456) is a single global conference. Only
one connection can be the master — the first one to connect after a restart.
Any other connections join as regular users and see whatever state the
conference is already in.

The top-level interactive session can arrange exclusive access when needed
— e.g. to capture authoritative fixture recordings with `xcal-test run`.
But **agent teams working in parallel must not assume exclusive access**.
In particular:

- Do not assume you will be the master. Another agent or manual session may
  already be connected.
- Do not assume `who` output contains only users you created.
- Run fixture scripts **one at a time** from the top-level session — they
  are not safe to parallelize against a shared server.
- Prefer in-process unit tests (level 1) for command development. They are
  isolated, fast, and deterministic. Reserve the live C server for
  end-to-end verification from the top-level session.

Similar rules apply to the Rust server, if we run it separately.

## Tooling: `xcalr serve`

The Rust server can be run locally in TCP mode as follows:

```sh
cd xcal_in_the_rust && cargo run -p xcalr -- serve --tcp 127.0.0.1:2457
```

This can be used to run `xcal-test check` against a live Rust server, for final integration test confirmation.

## Tooling: `xcal-test`

`xcal-test` is a Rust CLI tool (using tokio) that drives JSONL scripts
against a live TCP server. It has three subcommands:

### `run` — capture transcripts

```
$ xcal-test run tests/fixtures/hello/in.jsonl
```

Runs the script against a server (default `localhost:2456`), saves
per-connection transcripts to the script's parent directory. Use
`--transcript-dir` to override, or `--stdout` to print instead.

### `check` — diff against saved transcripts

```
$ xcal-test check tests/fixtures/hello/in.jsonl
```

Re-runs the script and diffs live output against previously-saved `.txt`
transcripts. Shows colored diff for mismatches. Exit code 0 if all match.

### `check-all` — run all fixtures

```
$ xcal-test check-all tests/fixtures/
```

Discovers all subdirectories containing `in.jsonl`, runs each sequentially,
prints a summary. Place a `SKIP.md` file in a fixture directory to skip it.


## JSONL script format

Each line is a JSON object. Lines with a `"comment"` key are skipped.
All other lines must have `"action"` and `"conn"` keys (except `sleep`).

| Action | Required fields | Description |
|--------|----------------|-------------|
| `connect` | `conn` | Open a TCP connection. |
| `send` | `conn`, `text` | Send `text` + `\r\n`. |
| `send_raw` | `conn`, `bytes` | Send raw bytes (no `\r\n` appended). |
| `send_bytes` | `conn`, `hex` | Send hex-encoded bytes (e.g. `"ff f3"`). |
| `expect` | `conn`, `contains` | Read until buffer contains substring. Optional `timeout_ms` (default 5000). |
| `drain` | `conn` | Read until quiet. Optional `quiet_ms` (default 100). |
| `disconnect` | `conn` | Close the connection. |
| `sleep` | `ms` | Sleep for `ms` milliseconds (no `conn` needed). |

**Important: `expect` clears the receive buffer** after a successful match.
If the server sends multiple messages in a single TCP batch, the first
`expect` consumes the entire buffer. Expect the *last* distinctive string
in the batch rather than chaining multiple expects.

### Telnet handling

`xcal-test` automatically strips telnet IAC sequences from received data
and responds WONT/DONT to all option negotiations. Transcripts contain
clean text only.

### Normalization

Place a `normalize.toml` in the fixture directory to define regex
replacement rules applied to both expected and actual transcripts before
comparison:

```toml
[[rules]]
pattern = '\d{1,2}:\d{2}:\d{2} [ap]\.m\.'
replace = "HH:MM:SS xm"

[[rules]]
pattern = '\d{1,2}/\d{1,2}/\d{2,4}'
replace = "M/D/YY"
```

Use `--normalize-rules` to specify an alternate path.


## Key scenarios to test

### Basic lifecycle

Connect, login, who, tell, bye — the minimal happy path. Captured as
`tests/fixtures/hello/`.

### Messages don't interrupt composition

Alice starts composing. Bob sends a message mid-compose. Alice should not
see Bob's message until she finishes composing and returns to idle.

### Ignore lists

Alice ignores Bob. Bob's messages should not reach Alice, but Alice's
should still reach Bob.

### Subconferences

Master promotes Alice to submaster, gives Bob to Alice. Bob can talk to
Alice but not Master directly.

### Command abbreviation

Abbreviated commands work: `t` for tell, `w` for who, `by` for bye, etc.

### Tell-all behavior

Tell-all (no recipients), enable/disable, reject-all flag.

### %k message abort

Start composing, type `%k` on a line, send blank line. Message should be
cancelled.

### Edge cases to discover via exploration

- What happens when you `tell` someone who is `out`?
- What happens when you `tell` yourself?
- What happens when you `ignore` someone mid-composition?
- What does `who` show for someone in each state?
- What does the master see vs. a regular user for the same `who`?
- What happens when a submaster disconnects — where do their users go?
- What does `give` do when the target already has subordinates?
- Can you `kill` someone who is composing a message to you?
- What is the exact output format of each command?

These are best discovered by interactive exploration against the C server,
then codified as test scripts.


## Workflow

1. **Implement command handlers** in grouped files under `conferences/cmd/`.
   Each group lives in its own file (e.g. `cmd_listing.rs`, `cmd_query.rs`,
   `cmd_prefs.rs`) with the uniform `fn(conf, who, args) -> CommandResult`
   signature. Stubs return `lang.not_implemented()` until filled in.
2. **Write in-process unit tests** in a `#[cfg(test)]` module co-located
   with each handler file. Import `TestConference` from the test harness.
   This is the primary test suite.
3. **Consult the C server** when exact output format or edge-case behavior
   is unclear — use telnet or `xcal-test run --stdout`.
4. **Verify the Rust server** by running `xcal-test check` against the
   existing fixture baselines (already captured from the C server).
5. **Add in-process fixture replay** to catch regressions without needing
   a running server.
6. **Repeat** until feature-complete.

The C server is the oracle. When in doubt about behavior, ask it. The 33
existing fixtures cover most commands and interactions; new fixtures should
rarely be needed.
