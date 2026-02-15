# Xcaliber — Web Server Design

> **Status:** Draft. Not finalized.

A thin web server wrapping `xcalr` with WebSocket sessions, a lobby for
discovering and joining conferences, and a JSON protocol for browser
communication. See [`docs/UX_IMPROVEMENTS.md`](../docs/UX_IMPROVEMENTS.md)
for the motivating UX goals.

## Design principles

- **Changes at the edges.** The conference domain logic (`Conference`,
  `User`, `Port` trait) should not need to know about WebSockets, sessions,
  or lobbies. The web layer is a new transport, like TCP.
- **Preserve the synchronous feel.** Xcaliber is a synchronous, ephemeral
  space. The lobby helps people find each other; it doesn't turn the system
  into a message board.
- **Keep it thin.** The server serves static assets and manages WebSocket
  sessions. All conference logic lives in `XcaliberApp` and below.

## Architecture overview

```
Browser ↔ WebSocket ↔ Session ↔ WebSessionPort ↔ XcaliberApp ↔ Conference
                        ↑                              ↓
                   SessionManager ←── AppListener ──────┘
```

**axum** serves static assets and handles WebSocket upgrades. Each
WebSocket connection is associated with a **Session** (created or resumed).
Sessions are managed by a **SessionManager** that tracks all active
sessions and subscribes to conference lifecycle events via an
**AppListener**.

When a user joins a conference, their session creates a **WebSessionPort**
(a thin `Port` trait implementation) and registers it with `XcaliberApp`
through the normal port sink. When they leave or the conference ends, the
port is torn down but the session persists in the lobby.

## Sessions

A session represents a browser user's presence in the system. It has a
larger lifecycle than any single WebSocket connection or conference
participation.

### Session lifecycle

1. Browser connects WebSocket, sends `hello` (with or without token).
2. Server creates or resumes a session, responds with token + lobby state.
3. Session sits in **lobby** state until the user links a conference.
4. User links → session creates a `WebSessionPort`, registers with app →
   session enters **conference** state.
5. Conference ends or user leaves → port torn down → session returns to
   **lobby**.
6. WebSocket drops → session stays alive for a timeout period (a few
   minutes), then dies. If a `WebSessionPort` exists, it sends
   `PortMessage::Disconnected` through the sink on final timeout.
7. WebSocket reconnects within timeout → browser sends same token →
   reattaches to existing session (in whatever state it was in).

### Session tokens

Tokens are stored in the browser's **`sessionStorage`**, which:

- Survives page reloads (the key requirement).
- Is per-tab — each tab gets its own session, enabling multiple independent
  conference users from the same browser.
- Is inaccessible to other origins (same-origin policy).
- Is not sent automatically with requests (no CSRF surface).

The token is transmitted as the first WebSocket message, not in cookies or
URL parameters.

### Multiple tabs

Each tab is an independent session. This is a feature: it lets users
experiment with multiple handles, which is more dignified than summoning an
LLM to keep you company (see UX Improvements, Issue 5).

A user accidentally opening a second tab simply lands in the lobby with a
fresh session. No harm done.

## The `WebSessionPort`

A thin `Port` trait implementation. The session owns it and mediates
between the port interface and the current WebSocket connection.

- **`try_send()`** forwards data to the session, which wraps it in JSON
  and pushes to the WebSocket (if one is currently attached). If no
  WebSocket is connected, data may be dropped — brief disconnections during
  a page reload are acceptable losses.
- **Identity methods** (`id()`, `public_name()`, etc.) are stable for the
  session's lifetime, even across WebSocket reconnects.
- **`shutdown()`** signals the session to close.

The port does not handle WebSocket lifecycle, reconnection, or the JSON
protocol. That's the session's job.

## SessionManager and AppListener

The **SessionManager** is a central registry of all active sessions. It
serves two purposes:

1. **Reconnection:** When a `hello` arrives with a token, the session
   manager looks up the existing session and reattaches the new WebSocket.
2. **Lobby state distribution:** The session manager subscribes to
   conference lifecycle events via `AppListener` and distributes lobby
   state updates to all sessions currently in the lobby.

The **AppListener** is a trait registered with `XcaliberApp` via something
like `app.add_listener(listener)`. It receives basic lifecycle events:
conference created, conference ended, user count changes — enough for the
lobby to know what's happening without coupling to conference internals.

## JSON protocol

Bidirectional JSON over WebSocket. All messages have an `action` (client →
server) or `event` (server → client) key.

### Client → Server

| Message | Description |
|---------|-------------|
| `{"action": "hello"}` | New session. |
| `{"action": "hello", "token": "..."}` | Resume existing session. |
| `{"action": "link", "keyword": "XYZ"}` | Join (or create) a conference. The keyword is decorative — a homage to the original, where "linking" XYZ made you master for the next 45 minutes. For now, only one conference exists at a time; the keyword is flavor. |
| `{"action": "send", "text": "..."}` | Send a line of input. Only valid during conference state. Maps to `PortMessage::InputLine`. |
| `{"action": "leave"}` | Leave conference, return to lobby. Tears down the `WebSessionPort`. |

### Server → Client

| Message | Description |
|---------|-------------|
| `{"event": "session", "token": "..."}` | Session established or resumed. Always the first response to `hello`. |
| `{"event": "lobby", "waiting": N, "conference": "XYZ"}` | Full lobby state snapshot. `conference` is `null` if none is active. Sent on any lobby-relevant state change. |
| `{"event": "joined"}` | Confirmation that `link` succeeded; session is now in conference state. |
| `{"event": "output", "text": "..."}` | Conference output. Raw text with `\n` line breaks (no `\r\n` — the `WebSessionPort` skips TCP's CRLF conversion). May contain control characters like BEL; the browser UI decides how to render them. |
| `{"event": "ended"}` | Conference has ended (master typed `bye`, timeout, etc.). Session returns to lobby state. |

The protocol is deliberately simple: full-state lobby snapshots instead of
incremental deltas, no explicit error messages for v1, no backpressure
signaling to the browser.

## Security

- **Origin checking** on WebSocket upgrade requests.
- **No cookies** for session tokens — `sessionStorage` eliminates CSRF.
- **No authentication** for now. The original required being on the
  Dartmouth system, but as our historical documents show, that didn't stop
  BBS phreaks from breaking in. In the spirit of democratized time-sharing
  and scrambles for moderator powers, it's a public thing.
- **XSS mitigation:** The Rust server and a small JS client minimize
  attack surface. The few places user strings are inserted into HTML will
  be handled with extreme care.

## Static assets

The web server serves static assets (HTML, CSS, JS) for the lobby and
conference UI. Details of the frontend are out of scope for this document —
the focus here is the server-side architecture and the protocol contract
that the frontend will consume.

## Testing strategy

Tests exercise the full session layer in-process without a browser.

**Setup:** Each test starts axum on `127.0.0.1:0` (OS-assigned port)
backed by a real `XcaliberApp`. A Rust test client (`tokio-tungstenite`)
connects via WebSocket and speaks the JSON protocol.

**Test client:** A helper struct (e.g. `WsClient`) wrapping a WebSocket
connection with convenience methods for sending actions and asserting on
expected events. Multiple clients per test simulate multiple browser tabs /
users.

**Key scenarios to cover:**

- **Happy path:** hello → lobby → link → joined → send/output → leave →
  lobby.
- **Multi-user:** Two clients link the same conference, exchange messages.
- **Conference end:** Master sends `bye`, all clients receive `ended`,
  return to lobby.
- **Reconnect in lobby:** Client disconnects, reconnects with same token,
  gets lobby state.
- **Reconnect in conference:** Client disconnects mid-conference,
  reconnects, session still in conference state, output resumes.
- **Session timeout:** Client disconnects, no reconnect within timeout,
  session and port are cleaned up.
- **Multiple independent tabs:** Two clients with no shared token, both
  link, both become separate conference users.
- **Lobby updates:** Client in lobby sees `lobby` events as others join /
  conferences start and end.

This sits one thin layer below full E2E browser tests. It covers the
server, session manager, JSON protocol, port integration, and conference
domain logic. Browser-level testing (DOM, actual UI) comes later when the
frontend is built.
