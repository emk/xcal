# Xcaliber Mark II — Original Code Overview

## Background

Xcaliber Mark II is a multi-user conference (chat) program written in ANSI C
over Christmas 1997 by Michael J. Fromberger. It's a reimplementation of the
original Xcaliber, a GMAP assembly program written by David D. Wright ('78) and
Michael S. Morton ('80) for Dartmouth College's DCTS time-sharing system
(1977–1982).

The original Xcaliber was a hugely popular multi-terminal conference system —
essentially a proto-IRC predating most chat programs. Mark II recreates its
essential features in ~4,500 lines of portable ANSI C with no external
dependencies beyond POSIX.


## Source files

### Core logic

| File          | ~Lines | Purpose |
|---------------|--------|---------|
| `cmd.c`       | 1614   | Command dispatch and 40+ command handlers. The largest file. |
| `con.c`       | 890    | Conference lifecycle: the `select()` main loop, message delivery, user join/leave, subconference management |
| `user.c`      | 351    | Per-user state machine, input polling, message queue operations |
| `xcal.c`      | 206    | `main()`: argument parsing, signal handling, outer conference loop |

### Data structures

| File          | ~Lines | Purpose |
|---------------|--------|---------|
| `stream.c`    | 300    | Circular byte buffer ADT (`stream`) with put/get/printf-like operations |
| `message.c`   | 150    | Message struct: sender, recipient bitmap, type, data stream |

### Infrastructure

| File          | ~Lines | Purpose |
|---------------|--------|---------|
| `net.c`       | 89     | Thin BSD socket wrappers: listen, accept, read, write |
| `telnet.c`    | 120    | RFC 854 telnet negotiation (mostly rejects all options) |
| `lang.c`      | 200    | i18n string tables (English, French, German, Scottish Gaelic) |
| `xhelp.c`     | 100    | Help topic lookup from compiled TOC files |
| `log.c`       | 50     | Connection/disconnection event logging |
| `port.c`      | 50     | Port-number-to-location-name mapping |
| `val.c`       | 50     | Privilege password validation |
| `mem.c`       | 20     | Debug malloc/free wrappers (compiled only with `DEBUG2`) |

### Headers

14 header files (`con.h`, `user.h`, `message.h`, `stream.h`, `cmd.h`, `net.h`,
`telnet.h`, `lang.h`, `xhelp.h`, `port.h`, `log.h`, `val.h`, `mem.h`,
`default.h`) totaling ~600 lines.

### Data files

- `help/` — 46 `.hf` help topic files + compiled binary TOC (`exptoc.x`)
- `lang/` — 4 language source files (`.lf`) + build script (`mklang`)


## Build system

Simple Makefile. GCC with `-Wall -Werror -O2 -funsigned-char`. No external
libraries. Single executable `xcal`.


## Architecture

### Main loop (`con.c: con_run()`)

Single-threaded, `select()`-based cooperative concurrency:

```
while (!done) {
    1. For each IDLE user with queued messages → start transmitting one
    2. select() on all user sockets (read) + receiving users (write)
       - If messages are queued: non-blocking poll (timeout = 0)
       - Otherwise: block until next warning time
    3. Accept new connections on listener socket
    4. For each user with readable input → user_poll()
    5. Handle state transitions:
       - DONE_ST    → con_deliver()   (post composed message)
       - CMD_ST     → con_command()   (parse and execute)
       - LOGGED_ST  → con_login()     (broadcast join notification)
       - CHGD_ST    → con_change()    (broadcast name change)
       - WARND_ST   → con_newwarn()   (post system warning)
       - RECV_ST    → user_sendblk()  (send next chunk to socket)
    6. Check conference timer, post warnings as needed
}
```

### Key design principle: messages never interrupt composition

This is the defining UX feature, called out in the README:

> "Unlike IRC, you can compose messages without being interrupted by other
> messages."

**How it works:**

1. Messages are always enqueued regardless of recipient state
   (`con_post_one()` → `user_enqueue()`).
2. Messages are only *dequeued and transmitted* when the user is `IDLE_ST`
   (the `ISIDLE()` check at `con.c:175`).
3. While composing (BUILD_ST), the queue silently grows.
4. Messages are delivered one at a time, whole — `user_transmit()` dequeues
   one message, sends it in 512-byte blocks, returns to IDLE only when done.
5. Input is always read from the socket (the fd is always in the read set),
   so **type-ahead works** — keystrokes buffer silently during RECV_ST and
   get processed when the user returns to IDLE.


### User state machine

15 states, stored in the low bits of `user.bits`:

```
IDLE_ST (0)   — Waiting for input. Messages can be delivered.
BUILD_ST (1)  — Composing a message. Lines appended. Blank line → DONE_ST.
DONE_ST (2)   — Message ready for delivery (transient).
RECV_ST (3)   — Receiving a message (being written to socket in chunks).
CMD_ST (4)    — Processing a command (transient).
LOGIN_ST (5)  — Entering name at initial connection.
LOGGED_ST (6) — Just logged in (transient, triggers join notification).
CHG_ST (7)    — Entering new name for id change.
CHGD_ST (8)   — Name change complete (transient).
OUT_ST (9)    — "Out" — rejecting messages, any input returns to IDLE.
WARN_ST (10)  — Master composing a warning.
WARND_ST (11) — Warning composition complete (transient).
READI_ST (12) — Reading help text (treated as RECEIVING for output).
EXPL_ST (13)  — Reading explanation (treated as RECEIVING for output).
LINE_ST (14)  — "Line mode" — no messages, input treated as commands.
DEAD_ST (15)  — Disconnected, awaiting cleanup.
```

The `RECEIVING()` macro groups RECV_ST, EXPL_ST, and READI_ST — all states
where the server is writing output to the user's socket.


### User flags

Stored in the high bits of `user.bits`:

| Flag       | Meaning |
|------------|---------|
| MASTER     | Conference master (first to connect) |
| SUBMST     | Submaster (controls a subconference) |
| GETNEW     | Master who receives new connections |
| TELALL     | Tell-alls enabled for this master's subconference |
| REJALL     | User rejects all tell-all messages |
| REJPRT     | User hides port/location info from others (reciprocal) |
| REJBRK     | User ignores telnet break signals |
| REJNOT     | User rejects join/leave notifications |
| REJCTL     | Strip control characters from output |
| WASKLT     | User was killed (vs. natural disconnect) |
| PRVBIT     | User has administrative privileges |


### Conference structure

```c
typedef struct {
    user **who;      // Array of user pointers (indexed by user number)
    int    size;     // Max users
    int    num;      // Currently connected
    int    max;      // Historical peak
    user **left;     // Disconnected users (for the `left` command)
    int    ear;      // Listener socket
    mptr   warn;     // Current conference-wide warning
    time_t up, til;  // Start time, end time
    int    trblk;    // Transmission block size (default 512 bytes)
    int    new;      // Index of master who receives new connections
    bolist bo[];     // Bounce (ban) list
} con;
```


### Subconference hierarchy

Users form a tree rooted at the conference master:

```
Master #0
├── User #1
├── User #2
├── Submaster #3
│   ├── User #4
│   └── User #5
└── Submaster #6
    └── User #7
```

Each user has a `mstr` field pointing to their master. A user can communicate
with:
- Their master
- Their siblings (same `mstr`)
- Their subordinates (users whose `mstr` is them)

Commands like `who` and `tell` are scoped to this subconference. The master or
submaster can transfer users between subconferences with `give`, promote with
`mty`, and demote with `norm`.


### Message structure

```c
typedef struct msg {
    int    from;       // Sender's user index
    stream data;       // Message content
    byte   rcpt[5];    // Recipient bitmap (up to 40 users)
    byte   type;       // M_MSG, M_TOALL, M_WARN, M_NOTIFY, M_HELP, M_EXPL
    int    ref;        // Reference count
} message;
```


### Message delivery path

```
1. User types "tell 0,1; Hello"
2. cmd_tell() parses recipient list into bitmap
3. Validates recipients (same subconference, not ignoring sender)
4. Creates message with header "[from who #2: username]"
5. con_post() iterates all users, con_post_one() checks:
   - Is user a recipient? (bitmap)
   - Is user connected? (sd >= 0)
   - Is user ignoring sender? (igmap)
   - Is user "out"?
6. If valid → user_enqueue() adds to their message queue
7. Main loop: when user is IDLE → user_transmit() dequeues one message
8. user_sendblk() writes 512-byte chunks to socket
9. When output stream empty → IDLE_ST → next message dequeued
```


### Command system

40+ commands with wildcard-abbreviated matching:
- `*t*e*ll` matches `t`, `te`, `tel`, `tell`
- `w*ho` matches `w`, `wh`, `who`
- `c*l*ock` matches `c`, `cl`, `clo`, `cloc`, `clock`

Commands are matched case-insensitively against a table of `{pattern, handler}`
pairs in `cmd.c`.

**User commands:** `tell`, `who`, `port`, `id`, `clock`, `time`, `left`,
`help`, `explain`, `info`, `bye`, `tty`, `im`, `users`, `version`, `runtime`

**Message filtering:** `ignore`/`accept`, `ra`/`aa` (reject/accept all),
`rn`/`an` (reject/accept notifications), `rp` (reject ports), `rc`/`oc`
(reject/accept control chars), `rb` (reject breaks), `out`, `line`

**Master commands:** `warn`, `kill`, `enable`/`disable` (tell-alls),
`mty`/`norm` (promote/demote submaster), `give` (transfer users)

**Privileged commands:** `xyzzy` (gain/lose privileges), `xtend` (extend
conference), `xreset` (force termination), `xlang`/`xdl` (language)


### Status line display

The `who` command shows one line per user:

```
         +++-------------------------------------------what user is doing
         ||| +++++-------------------------------------status indicators
         vvv |||||                  vvvvvvvvvvvvvvvv---"con name"
=>[#0] M COM    CN  Asylum   0/6533 Sting, masterful
  [#2]   B/A R  CN  Moscow   1/2013 Gwenhwyvar
  [#3]   ---   PCN  Rejecting ports
  [#4]   ---    CN  St Croix 0/3133 echo
  [#7]   B-3 R  CN  Reality  0/2130 Heps, l/t Mingus
```

Components: master indicator (`=>`), user number (`[#N]`), master/privilege
code, 3-char activity state (`B/A`, `COM`, `---`, `R/M`, etc.), flag
indicators (R=reject-all, P=reject-ports, C=reject-controls, N=notifications),
port name and number, user's chosen name.


### Special behaviors

- **`%k` abort:** If the last text before the blank line ending a message is
  `%k`, the message is cancelled instead of sent.
- **`rp` at login:** Typing `rp` instead of a name at the login prompt sets
  reject-ports before entering your name.
- **Warnings (`M_WARN`):** Cannot be ignored. Delivered to all users regardless
  of ignore lists or out status.
- **Conference timer:** Configurable duration (default 45 min). Warnings posted
  at intervals as time runs out. Master can extend with `xtend`.
- **Loop mode (`-l`):** Server reuses listener socket and restarts the
  conference after it ends, without TIME_WAIT delays.
- **SIGINT handling:** First interrupt closes the listener (stops new
  connections). Second interrupt forces shutdown. SIGPIPE is ignored so broken
  connections return errors instead of crashing the server.
