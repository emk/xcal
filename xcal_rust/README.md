# `xcal_rust`: Rust tooling and server for `xcal`

- Eric Kidd (2026)
- Based on code by Michael J Fromberger (1997)

This is a recreation of Xcaliber, a multiuser chat program that thrived from 1977 until the arrival of BlitzMail at Dartmouth. The original ran on DCTS, the legendary Dartmouth College Time-Sharing System from the the 60s.

See [`xcal`](../README.md) for more information about the original DCTS Xcaliber (1977) and the Unix C port Xcaliber Mark II (1997). This is Xcaliber Rust (2026), built in 2.5–3 days with a mix of human planning, extensive captured transcripts of Xcaliber Mark II, and a _whole_ lot of Claude Opus 4.6. This is not "vibe" code, in that a human has seen most of it (and expressed some pretty strong design opinions around the core logic). But almost none of it was written by a human.

The times, they do keep changing.

## Quick start: running the Rust server

Start the server on the default port (2456):

```sh
cd xcal_rust
cargo run -p xcalr -- serve --tcp
```

Connect with netcat:

```sh
nc localhost 2456
```

You'll see the master terminal welcome. Type your name and press Enter to join the conference.

## What's here

- `tools/xcalr/` — The Rust Xcaliber server.
- `tools/xcal-test/` — CLI tool that runs JSONL scripts against a live server and captures transcripts. This is pretty substantial part of how this works, a recording and validation tool that made it possible to test behavior in detail and recreate it faithfully.
- `crates/xcal_test_tools/` — Data structures for the JSONL protocol testing language.
- `crates/dcts/` — A basic DCTS "OS" layer (mostly I/O ports).
- `tests/fixtures/` — Sample input/output data captured from the original C `xcal` using `xcal-test`.
