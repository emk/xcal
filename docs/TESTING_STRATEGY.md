# Xcaliber Mark II — Testing Strategy

## Core idea

We have a working C implementation that speaks a line-oriented text protocol
over TCP. Any new implementation (Rust, Elixir, etc.) should expose the same
TCP interface for its core conference logic. We can then drive both servers
with identical scripted scenarios and diff the per-connection transcripts.

The web frontend (WebSocket/LiveView) is a separate layer built on top of the
tested core. Get the TCP behavior right first.


## Two levels of testing

### 1. In-process unit tests (primary)

The conference logic should be a library — a pure state machine that takes
"user N sent line X" and produces output for each connection. No sockets, no
async runtime, no timeouts.

```rust
#[test]
fn messages_dont_interrupt_composition() {
    let mut conf = Conference::new();
    let alice = conf.connect();
    conf.login(alice, "Alice");
    let bob = conf.connect();
    conf.login(bob, "Bob");

    conf.input(alice, "tell 1");        // start composing
    conf.input(alice, "first line");
    conf.input(bob, "tell 0; surprise!");  // bob sends mid-compose
    conf.input(alice, "second line");
    conf.input(alice, "");               // blank line = done

    let alice_out = conf.drain_output(alice);
    // Alice sees her delivery confirmation BEFORE bob's message
    assert!(alice_out.find("surprise!").unwrap() >
            alice_out.find("first line").unwrap());
}
```

This is where the bulk of the behavioral testing lives. It's fast (thousands
of tests in milliseconds), deterministic (no timing dependencies), and tests
the actual logic without any I/O concerns.

The conference library exposes a simple interface:

- `connect() -> UserId` — simulate a new connection
- `login(user, name)` — set user's name
- `input(user, line)` — feed a line of input
- `drain_output(user) -> String` — collect all pending output for a user
- `disconnect(user)` — simulate connection close

This is essentially a synchronous wrapper around the conference state machine
that processes input immediately and buffers output. The TCP server and
WebSocket layer are thin adapters over this same interface.

### 2. TCP integration tests (secondary)

For end-to-end confidence and for verifying against the original C server.
Uses the JSONL script format described below. These tests involve real
sockets but should still be fast — see "Timeouts" below.


## Timeouts

Both the C server and any rewrite are pure CPU-bound state machines with no
disk I/O, database calls, or external service dependencies. A round trip
through the conference logic is microseconds.

For TCP integration tests:
- **`drain` quiet timeout:** 50–100ms is generous. If the server hasn't sent
  anything for 100ms, it's done.
- **`expect` timeout:** 500ms is more than enough. If the expected output
  hasn't arrived in half a second, something is wrong.
- **No retry loops or backoff.** If it doesn't work quickly, it's a bug.

For in-process unit tests: no timeouts at all. Everything is synchronous.

This means the full test suite (both levels) should run in seconds, not
minutes.


## Tooling: two modes, one format

A single tool, written in Python (asyncio), that operates in two modes:

### 1. Interactive exploration mode

A CLI client for manually driving multiple named connections against a running
server. Think of it as a multi-session telnet client.

```
$ xcal-test explore --host localhost --port 2456

xcal> :connect alice
[alice] connected
xcal> :connect bob
[bob] connected
xcal> :switch alice
alice> Alice
[alice] < Welcome to Xcaliber...
alice> :switch bob
bob> Bob
[bob] < Alice has joined...
bob> tell 0; hello from bob
alice> :switch alice
[alice] < [from who #1: Bob]
[alice] < hello from bob
alice> :quit
```

Commands prefixed with `:` are client meta-commands (`:connect`, `:switch`,
`:quit`, `:sleep`, `:drain`, etc.). Everything else is sent to the current
connection as a line.

With `--record session.jsonl`, the tool writes a JSONL transcript of
everything that happened — what was sent, what was received, and the timing.
This can be edited into a test case.


### 2. Test runner mode

Reads a JSONL script, executes it against a server, captures per-connection
transcripts, and either saves them (for creating expected output) or compares
them against saved expected output.

```
$ xcal-test run --script tests/tell_basic.jsonl \
                --host localhost --port 2456 \
                --save-transcripts tests/tell_basic/

$ xcal-test run --script tests/tell_basic.jsonl \
                --host localhost --port 2456 \
                --expect-transcripts tests/tell_basic/
```

The first invocation runs against the C server and saves the transcripts as
the expected baseline. The second runs against the new implementation and
diffs.


## JSONL script format

Each line is a JSON object. The format is implementation-independent — it
describes abstract actions on named connections against a TCP server.

### Actions

#### connect

Open a new TCP connection and assign it a name.

```json
{"conn": "alice", "action": "connect"}
```

The host and port come from command-line arguments, not the script, so the
same script runs against any server.

#### send

Send a line of text (appends `\r\n` automatically).

```json
{"conn": "alice", "action": "send", "text": "Alice"}
{"conn": "alice", "action": "send", "text": "tell 1; hello"}
```

#### send_raw

Send exact bytes without appending `\r\n`. For testing partial input, telnet
sequences, or other edge cases.

```json
{"conn": "alice", "action": "send_raw", "bytes": "partial input with no newline"}
```

#### expect

Wait until the connection's received output contains the specified text.
Fails if not seen within timeout (default 500ms). This is a synchronization
gate, not a transcript assertion — it ensures the server has finished
processing before the script continues.

```json
{"conn": "alice", "action": "expect", "contains": "has joined"}
```

Can also match with a regex:

```json
{"conn": "alice", "action": "expect", "matches": "\\[#\\d+\\].*Bob"}
```

#### drain

Read all available output until the server goes quiet (no data for N ms).
This is the implicit behavior between actions, but an explicit `drain` can
be useful after actions that produce unpredictable amounts of output.

```json
{"conn": "alice", "action": "drain", "quiet_ms": 100}
```

#### disconnect

Close the connection (server sees EOF).

```json
{"conn": "alice", "action": "disconnect"}
```

#### sleep

Wait for a fixed duration. Use sparingly — prefer `expect` for
synchronization. Useful for testing timing-dependent behavior (conference
warnings, etc.).

```json
{"action": "sleep", "ms": 500}
```

#### comment

Ignored by the runner. For documenting test intent.

```json
{"comment": "Alice composes a message, Bob sends one mid-composition, Alice finishes. Bob's message should be delivered only after Alice goes idle."}
```

### Transcript handling

The test runner captures a separate transcript per connection — the complete
sequence of bytes received from the server. These are saved as plain text
files named by connection (e.g., `alice.txt`, `bob.txt`).


## Normalization

Before comparing transcripts, apply normalization rules to handle expected
differences between implementations or between runs:

- **Timestamps:** Replace `\d{2}:\d{2}:\d{2}` with `HH:MM:SS`
- **Dates:** Replace date patterns with `YYYY-MM-DD`
- **Duration/time remaining:** Replace `\d+ seconds` with `N seconds`
- **Telnet sequences:** Strip IAC bytes (0xFF and following command bytes)
- **Trailing whitespace:** Normalize line endings to `\n`, strip trailing spaces
- **Version strings:** Replace version numbers

Normalization rules are specified on the command line or in a config file, not
baked into scripts, so different tests can use different rules.


## Key scenarios to test

### Basic lifecycle

```jsonl
{"comment": "Basic connect, login, who, tell, bye"}
{"conn": "master", "action": "connect"}
{"conn": "master", "action": "expect", "contains": "name"}
{"conn": "master", "action": "send", "text": "Master"}
{"conn": "master", "action": "expect", "contains": "Welcome"}
{"conn": "user1", "action": "connect"}
{"conn": "user1", "action": "expect", "contains": "name"}
{"conn": "user1", "action": "send", "text": "Alice"}
{"conn": "user1", "action": "expect", "contains": "Welcome"}
{"conn": "master", "action": "expect", "contains": "Alice has joined"}
{"conn": "user1", "action": "send", "text": "who"}
{"conn": "user1", "action": "expect", "contains": "[#0]"}
{"conn": "user1", "action": "send", "text": "tell 0; hello master"}
{"conn": "master", "action": "expect", "contains": "hello master"}
{"conn": "user1", "action": "send", "text": "bye"}
{"conn": "user1", "action": "expect", "contains": "Goodbye"}
{"conn": "master", "action": "expect", "contains": "Alice has left"}
{"conn": "master", "action": "send", "text": "bye"}
{"conn": "master", "action": "disconnect"}
```

### Messages don't interrupt composition

```jsonl
{"comment": "Alice starts composing. Bob sends a message mid-compose."}
{"comment": "Alice should not see Bob's message until she finishes."}
{"conn": "alice", "action": "connect"}
{"conn": "alice", "action": "send", "text": "Alice"}
{"conn": "alice", "action": "expect", "contains": "Welcome"}
{"conn": "bob", "action": "connect"}
{"conn": "bob", "action": "send", "text": "Bob"}
{"conn": "bob", "action": "expect", "contains": "Welcome"}
{"conn": "alice", "action": "expect", "contains": "Bob has joined"}
{"comment": "Alice begins composing a tell to bob"}
{"conn": "alice", "action": "send", "text": "tell 1"}
{"conn": "alice", "action": "expect", "contains": "Enter message"}
{"conn": "alice", "action": "send", "text": "first line"}
{"comment": "While alice is composing, bob sends her a message"}
{"conn": "bob", "action": "send", "text": "tell 0; surprise!"}
{"conn": "bob", "action": "drain", "quiet_ms": 100}
{"comment": "Alice continues composing and finishes (blank line)"}
{"conn": "alice", "action": "send", "text": "second line"}
{"conn": "alice", "action": "send", "text": ""}
{"comment": "NOW alice should see bob's message, after her own is delivered"}
{"conn": "alice", "action": "expect", "contains": "surprise!"}
{"conn": "bob", "action": "expect", "contains": "first line"}
```

### Ignore lists

```jsonl
{"comment": "Alice ignores Bob. Bob's messages should not reach Alice."}
{"comment": "But Alice's messages should still reach Bob."}
```

### Subconferences

```jsonl
{"comment": "Master promotes Alice to submaster, gives Bob to Alice."}
{"comment": "Bob can now talk to Alice but not Master directly."}
```

### Command abbreviation

```jsonl
{"comment": "Test that abbreviated commands work: 't' for tell, 'w' for who"}
{"conn": "alice", "action": "send", "text": "w"}
{"conn": "alice", "action": "expect", "contains": "[#0]"}
```

### Tell-all behavior

```jsonl
{"comment": "Test tell-all (no recipients), enable/disable, reject-all flag"}
```

### %k message abort

```jsonl
{"comment": "Start composing, type %k on last line, send blank line."}
{"comment": "Message should be cancelled, recipient should see nothing."}
```

### Edge cases to discover via exploration

- What happens when you `tell` someone who is `out`?
- What happens when you `tell` yourself?
- What happens when you `ignore` someone mid-composition?
- What does `who` show for someone in each of the 15 states?
- What does the master see vs. a regular user for the same `who`?
- What happens when a submaster disconnects — where do their users go?
- What does `give` do when the target already has subordinates?
- Can you `kill` someone who is composing a message to you?
- What is the exact output format of each command? (Spacing, punctuation, etc.)

These are best discovered by interactive exploration against the C server,
then codified as test scripts.


## Removing telnet from the C server

The C server's telnet handling is minimal — it mostly rejects all option
negotiations and handles break signals. For cleaner testing, we could strip
the telnet protocol code from the C server (or add a flag to disable it),
so both servers speak identical plain TCP. The relevant code is in
`telnet.c` and `user.c:user_handle_telnet()`.

Alternatively, the test client can simply strip telnet IAC sequences from
received data during normalization. This is simpler and avoids modifying
the original code.


## Workflow

1. **Design the conference as a library** with a synchronous, I/O-free
   interface (`connect`, `login`, `input`, `drain_output`, `disconnect`).
2. **Write in-process unit tests** against this interface as you build each
   feature. This is the primary test suite and where most behavioral
   coverage lives.
3. **Build the TCP exploration tool** (Python, asyncio).
4. **Explore interactively** against the C server. Discover exact behaviors
   and output formats.
5. **Record and codify** exploration sessions into JSONL test scripts.
6. **Capture baselines** by running scripts against C server, saving
   transcripts.
7. **Add the TCP listener** as a thin adapter over the conference library.
   Run JSONL scripts against it, diff against baselines.
8. **Repeat** until feature-complete. Both test suites grow with the
   implementation.

The C server is the oracle. When in doubt about behavior, ask it.
