# DCTS Port-Based I/O Architecture in xcalr

## What DCTS "Ports" Are Conceptually and Historically

In the original Dartmouth College Time-Sharing (DCTS) system (1964-1999), "ports" were physical terminal connections to the mainframe. Each port represented a one-to-one mapping between port numbers and physical locations on campus:

- **Physical Architecture**: Terminals plugged into wall jacks connected to nodes on the Kiewit Network. Around 100 nodes (New England Digital minicomputers) each supported 56 asynchronous terminal ports.
- **Location Identity**: Port numbers revealed physical locations — knowing someone's port meant knowing which terminal they were at. Xcaliber exposed this with fantasy names like "Avalon," "Mordor," "Istanbul" for terminal clusters.
- **Line-Oriented I/O**: DCTS used line-oriented input — users composed complete lines on their terminals before sending them as a single message to the mainframe. This was far more efficient than character-at-a-time models and meant applications could assume whole-line input.
- **Remote Access**: The system also supported remote clusters via HDLC leased lines and dial-up/Telenet access, extending the port abstraction beyond physically wired terminals.

The port abstraction was transport-agnostic even in the 1970s — hardwired terminals, dial-up modems, and remote clusters all appeared as "ports" to applications like Xcaliber.

---

## How the Rust `dcts` Crate Models Ports

The `xcal_rust/crates/dcts/src/ports/mod.rs` module provides a modern, transport-agnostic port abstraction inspired by the historical DCTS design:

### Core Types

1. **`PortId`** (`ports/mod.rs:14-28`): Unique UUID-based identifier for each connection, replacing physical port numbers.

2. **`Port` trait** (`ports/mod.rs:57-115`): The core abstraction over connection identity and outbound I/O.
   - **Identity methods** (must never block):
     - `id()` → unique PortId
     - `public_name()` → fantasy name shown to users (e.g., "Avalon")
     - `admin_name()` → detailed name for privileged users (e.g., "192.168.1.1:4532")
     - `port_address()` → DCTS-style formatted address: `"%8s %d/%04d"` (fantasy name, port bits, IP bits)
   - **Output method**: `try_send(data: Bytes) -> Result<(), Bytes>` — non-blocking enqueue for outbound data. Returns `Err(data)` if queue is full. Application sends `\n` for line breaks; port converts to transport format (e.g., `\r\n` for TCP).
   - **Lifecycle method**: `shutdown()` — non-blocking signal to close the connection.

3. **`PortMessage` enum** (`ports/mod.rs:141-170`): Events from transport to application:
   - `New { port }` — new connection arrived
   - `InputLine { port_id, input }` — complete line received (terminators stripped)
   - `SendComplete { port_id }` — last chunk finished writing
   - `Disconnected { port_id }` — connection closed

4. **`PortMessageSink` trait** (`ports/mod.rs:128-134`): Async channel for transport tasks to send `PortMessage`s to the application. Supports backpressure.

### Key Design Decisions

- **Asymmetric I/O Model**: Input is line-oriented (honoring DCTS heritage), output is byte-oriented (application controls formatting).
- **Non-blocking Constraint**: All `Port` methods must never block. Implementations use channels/buffers drained by async tasks.
- **Small Queue Sizes**: The `Port` implementation uses tiny queues (capacity 1 for TCP) because buffering/flow control lives in the application layer's `User` struct.
- **Single Disconnect Path**: Disconnections are reported only via `PortMessage::Disconnected` through the sink, never through `try_send` return values.

---

## How `app.rs` Uses Ports to Manage Connections

The `xcal_rust/tools/xcalr/src/app.rs` file implements the top-level application that owns the event loop and conference lifecycle:

### Architecture

1. **`XcaliberApp` struct** (`app.rs:17-32`):
   - Owns an mpsc channel (`sender`/`receiver`) for all application messages
   - Holds an `Option<Conference>` — conferences are created on demand and destroyed when empty
   - Outlives individual conferences (like C's `-l` mode)

2. **`ConferenceMessageSink`** (`app.rs:102-122`): Bridges the port layer to the app:
   - Implements `PortMessageSink`
   - Wraps each `PortMessage` in `ConferenceMessage::Port` before sending to the app channel

3. **Event Loop** (`app.rs:45-54`): Runs forever, receiving messages and dispatching them.

4. **Port Message Handling** (`app.rs:59-97`):
   - `PortMessage::New` → Get or create conference, call `conference.handle_new(port)`
   - `PortMessage::InputLine` → Dispatch to `conference.handle_input()`, check if master typed `bye` (terminates conference)
   - `PortMessage::SendComplete` → Call `conference.handle_send_complete()` to flush next block
   - `PortMessage::Disconnected` → Call `conference.handle_disconnected()`, destroy conference if empty

### Key Patterns

- **Lazy Conference Creation**: The conference is created when the first port connects, not at app startup.
- **Conference Termination**: Two ways to end a conference:
  - Master types `bye` → `handle_input()` returns `true`
  - Last user disconnects → `handle_disconnected()` returns `true`
- **Port Message Sink Factory**: `app.port_message_sink()` creates boxed sinks that the TCP listener can clone and share across connection tasks.

---

## How `conferences/mod.rs` Uses Ports for Message Delivery and User Management

The `xcal_rust/tools/xcalr/src/conferences/mod.rs` file implements the core conference domain:

### User-Port Relationship

Each `User` (from `xcalr/src/users.rs:196-223`) owns:
- A `Box<dyn Port>` for transport I/O (`users.rs:208`)
- An `output: BytesMut` buffer for data waiting to send (`users.rs:212`)
- A `queue: VecDeque<Arc<Message>>` for queued messages (`users.rs:216`)

The `Conference` tracks:
- `port_to_user: HashMap<PortId, UserId>` — map port IDs to user slots (`conferences/mod.rs:67`)
- `users: Vec<Option<User>>` — sparse user array (`conferences/mod.rs:69`)

### Connection Lifecycle

1. **New Connection** (`conferences/mod.rs:128-177`):
   - Find or allocate user slot
   - Assign role (first user → Master, others → Normal)
   - Create `User::new(id, port, role)` — stores the port
   - Send welcome message to `user.output` buffer
   - Call `flush_user_output()` to send via `port.try_send()`
   - Queue existing conference warning for new user
   - Insert into `port_to_user` map

2. **Input Handling** (`conferences/mod.rs:179-343`):
   - Look up user by port ID
   - Dispatch based on `user.state`:
     - `Login` → capture name, transition to `Idle`, send intro + "talking with" message
     - `Idle`/`Line` → dispatch command
     - `Composing` → accumulate lines, deliver on blank line (unless `%k` abort)
     - `Out` → any input returns to `Idle`
   - After state-specific handling, deliver queued messages and flush output

3. **Disconnection** (`conferences/mod.rs:396-442`):
   - Look up user by port ID
   - Call `auto_normalize()` to reparent subordinates if user was a submaster
   - Send exit notification **before** clearing user slot (so subconference scoping still works)
   - Record in `left` list
   - Remove from `port_to_user` map
   - Set user slot to `None`
   - Return `true` if conference is now empty

### Message Delivery Architecture

**Output Helpers** (`conferences/mod.rs:444-478`):
- `append_output()` — write text to user's `output` buffer
- `flush_output()` → `flush_user_output()` → `port.try_send(output.split())`
  - If `try_send()` returns `Err(data)`, re-append data to buffer (backpressure)

**Queuing and Delivery** (`conferences/mod.rs:552-669`):
- `deliver_composed()` (lines 552-640): Takes a user's composed message, builds recipient set, queues to each recipient's `VecDeque<Arc<Message>>`, then calls `deliver_queued()` for idle recipients.
- `deliver_queued()` (lines 642-669): **Only delivers if `user.state == Idle`**. Pops messages from queue, appends to `output` buffer (with control char filtering if `reject_controls`), then flushes.

**Key Insight**: Messages are queued immediately but only delivered when the user is `Idle`. This preserves the original Xcaliber design where incoming messages don't interrupt composition.

**Send Completion** (`conferences/mod.rs:389-393`): When the transport finishes writing a chunk (`PortMessage::SendComplete`), the conference calls `flush_output()` again to send the next block if more data is buffered.

### Subconference Scoping

**Notification Delivery** (`conferences/mod.rs:490-548`):
- `send_notification()` builds a recipient set using the 3-way visibility rule:
  - Same master (siblings + self)
  - User's master is viewer (subordinates)
  - User **is** viewer's master
- Skips users in `Login`, `Out`, or with `reject_notifications`
- Queues notification to eligible recipients, then delivers to idle ones

**Auto-Normalization** (`conferences/mod.rs:784-863`):
- When a submaster disconnects, `auto_normalize()` reparents their subordinates back to the parent master
- Sends "talking with {parent_name}" to each reparented subordinate
- Sends "you have been passed" + WHO listing to the parent master

---

## Interesting Architectural Decisions and Constraints

1. **Separation of Concerns**: The port layer is purely transport (networking, buffering, wire format). The conference layer owns all domain logic (state machines, subconference rules, message routing).

2. **Async/Sync Boundary**: Transport tasks are async (tokio). The application/conference layer is synchronous — all domain logic runs in a single-threaded event loop. Ports bridge the two with non-blocking `try_send()`.

3. **Backpressure Design**: Small port queues (capacity 1) force backpressure into the application's `User.output` buffer. The application controls chunking (via `send_block_size`). The transport signals `SendComplete` to request the next chunk.

4. **Line-Oriented Input, Byte-Oriented Output**: Honors DCTS heritage — applications receive complete lines, but send raw bytes with `\n` line breaks (port converts to `\r\n`).

5. **Fantasy Names**: TCP connections get whimsical location names (e.g., "Avalon", "Mordor") derived from IP address hash, echoing the original DCTS port naming system where physical locations had playful cluster names.

6. **Port Address Format**: `"%8s %d/%04d"` — 8-char fantasy name, low 3 bits of TCP port, low 16 bits of IP mod 10000. Mimics the original DCTS display format.

7. **Single Disconnect Path**: Disconnections are reported only via `PortMessage::Disconnected`, never via `try_send()` return values. The `TcpPort` uses `Arc<AtomicBool>` to ensure reader and writer tasks send exactly one disconnect notification.

8. **Conference Lifecycle**: Conferences are ephemeral — created when the first user connects, destroyed when the last disconnects. The `XcaliberApp` outlives conferences and creates fresh ones on demand.

9. **State Machine Discipline**: The `User` state machine controls message delivery. Only `Idle` state triggers `deliver_queued()`. This preserves the original Xcaliber's "compose in peace" design where incoming messages don't interrupt composition.

10. **Zero-Copy Message Sharing**: Messages are wrapped in `Arc<Message>` and queued to multiple recipients without copying. The body is built once and shared.
