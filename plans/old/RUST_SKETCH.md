# Xcaliber Mark II — Rust Rewrite Sketch

## Approach

Async Rust with a web frontend. The server serves a static HTML/CSS/JS page
(glass TTY theme) and handles WebSocket connections for the chat protocol.
Single binary, no separate frontend build step.


## Dependencies

- **axum** — HTTP server, WebSocket support, static file serving. Built on
  tokio, from the tokio team. The standard choice for Rust web services.
- **tokio** — Async runtime. Provides tasks, channels, timers, signals.
- **tower-http** — Middleware (static file serving, CORS if needed).
- **serde / serde_json** — Message serialization over WebSocket.
- **tracing** — Structured logging (replaces `log.c`).


## What we get for free

The original C code is roughly half infrastructure, half application logic.
In Rust with tokio + axum, the infrastructure half disappears:

| Original C                          | Rust equivalent                     |
|-------------------------------------|-------------------------------------|
| `net.c` (BSD sockets)              | `tokio::net::TcpListener` / axum   |
| `stream.c` (circular byte buffer)  | `bytes::BytesMut` / `String`       |
| `select()` loop in `con.c`         | tokio task-per-connection           |
| `telnet.c` (protocol negotiation)  | Not needed (WebSocket)              |
| `mem.c` (debug malloc)             | Ownership model                     |
| Signal handling                     | `tokio::signal`                     |
| Output chunking (`user_sendblk`)   | WebSocket frames (message-oriented) |


## Architecture

```
Browser ←—WebSocket—→ axum handler (one tokio task per connection)
                           ↕ channels
                      Conference task (owns all shared state)
```

### The conference task

A single long-lived tokio task that owns the `Conference` struct. It receives
commands from connection tasks via an MPSC channel and sends messages back
through per-user broadcast or MPSC channels.

```rust
struct Conference {
    users: Vec<Option<User>>,
    left: Vec<DisconnectedUser>,
    warning: Option<String>,
    started: Instant,
    duration: Duration,
    master: usize,       // index of user receiving new connections
}

struct User {
    name: String,
    state: UserState,
    flags: UserFlags,
    master: usize,       // index of this user's master
    ignore: BitVec,
    sender: mpsc::Sender<OutgoingMessage>,  // channel back to their WS task
}
```

### Connection tasks

Each WebSocket connection gets its own tokio task:

```rust
async fn handle_ws(ws: WebSocket, conf_tx: mpsc::Sender<Command>) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (user_tx, mut user_rx) = mpsc::channel(32);

    // Register with conference, get assigned user number
    conf_tx.send(Command::Join { reply: user_tx }).await;

    loop {
        tokio::select! {
            // User typed something in the browser
            Some(msg) = ws_rx.next() => {
                conf_tx.send(Command::Input { user, text: msg }).await;
            }
            // Conference has a message for this user
            Some(out) = user_rx.recv() => {
                ws_tx.send(out.to_ws_message()).await;
            }
        }
    }
}
```

### Messages don't interrupt composition

In the original C, this works because messages are only dequeued when `IDLE`.
In Rust, the conference task simply doesn't send messages down the user's
channel while they're composing. Or, if it does send them, the client-side JS
buffers them and displays them only after the user finishes composing.

The second approach (client-side buffering) is arguably cleaner for a web
client — the server sends messages whenever they're ready, and the browser
decides when to show them. This keeps the server stateless about presentation
concerns.

A third option: send messages immediately but render them in a separate
scrollback area from the composition area. The browser can show a split view
(or a subtle indicator that N messages are waiting) so the user knows messages
arrived without their composition being disrupted. This would be an improvement
over the original's behavior while preserving its spirit.


## User state

The 15 C states simplify significantly with a WebSocket client:

```rust
enum UserState {
    Login,              // Entering name
    Idle,               // Waiting for input, messages can be delivered
    Composing {         // Building a message
        recipients: BitVec,
        body: String,
    },
    Out,                // Unavailable, any input returns to Idle
    Line,               // Line mode, commands only
    Dead,               // Disconnected
}
```

States like RECV_ST, READI_ST, EXPL_ST disappear because WebSocket delivery
is non-blocking from the server's perspective — you just send the frame. The
CMD_ST and transient states (DONE_ST, LOGGED_ST, CHGD_ST, WARND_ST) become
function calls rather than states, since command processing is synchronous
within the conference task.


## Command system

The wildcard abbreviation matching (`*t*e*ll` → matches `t`, `te`, `tell`)
is a fun little parser:

```rust
fn matches(pattern: &str, input: &str) -> bool {
    // '*' in pattern means "zero or more optional characters"
    // Characters before the first '*' are required
    // Characters between '*'s are optional but must appear in order
}

struct CommandTable {
    commands: Vec<(&'static str, CommandHandler)>,
}
```

Command handlers take a reference to the conference and the invoking user,
plus the argument string:

```rust
type CommandHandler = fn(&mut Conference, usize, &str) -> CommandResult;
```


## Web client

A single HTML file served by axum, with inline CSS and JS. No build toolchain.

### Glass TTY theme

```css
body {
    background: #0a0a0a;
    color: #33ff33;
    font-family: "IBM Plex Mono", "Courier New", monospace;
    text-shadow: 0 0 5px rgba(51, 255, 51, 0.4);
}
```

### Client-side JS

Minimal WebSocket client:
- Connect to `ws://host/ws`
- Send user input as text frames (one line per frame)
- Receive messages as JSON or plain text frames
- Render into a scrollback div
- Composition area: a text input or small textarea
- Status line / who list: a fixed area updated on each push

### Protocol over WebSocket

Simple text or JSON frames:

```
Client → Server:
  { "cmd": "tell", "args": "0,1; hello" }
  { "cmd": "who" }
  { "line": "another line of my message" }
  { "line": "" }   // blank line = done composing

Server → Client:
  { "type": "message", "from": 2, "name": "Sting", "body": "hello" }
  { "type": "who", "users": [...] }
  { "type": "notify", "text": "Gwenhwyvar has joined" }
  { "type": "warning", "text": "Conference ends in 5 minutes" }
  { "type": "prompt", "text": "Enter your name: " }
```


## Module layout

```
src/
  main.rs           — Startup, axum router, static file serving
  conference.rs     — Conference struct, the conference task, message routing
  user.rs           — User struct, UserState, UserFlags
  command.rs        — Command table, wildcard matching, all command handlers
  command/
    tell.rs         — Tell command (message sending)
    who.rs          — Who/port/status display
    master.rs       — Kill, warn, give, mty, norm
    priv.rs         — xyzzy, xtend, xreset
    filter.rs       — ignore, accept, ra/aa, rn/an, rp, rc/oc
    info.rs         — time, clock, info, left, version, tty
  message.rs        — Message struct and types
  websocket.rs      — WebSocket connection handler task
  protocol.rs       — Client↔server message format (serde types)
  lang.rs           — i18n string tables (could be simple const maps)
  help.rs           — Help text (embedded as const &str or loaded from files)
static/
  index.html        — Glass TTY web client (HTML + inline CSS + JS)
```


## Build and run

```
cargo build --release
./target/release/xcal --port 8080
# Open http://localhost:8080 in a browser
```

Single binary. No runtime dependencies. Cross-compiles easily.
