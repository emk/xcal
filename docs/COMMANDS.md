# Xcaliber Mark II — Command Reference

Cross-reference of the C implementation (`cmd.c` cmdtab) with the online
help files (`help/*.hf`). Commands are dispatched via `con_command()` using
the `cmdtab[]` array, where `*` in a pattern marks an optional-from-here
abbreviation point (e.g., `w*ho` matches `w` or `wh` or `who`).

### Status legend

| Icon | Meaning |
|------|---------|
| 📄 | Transcript fixture captured (`tests/fixtures/`) |
| ✅ | Implemented in Rust |

## User commands

These are available to all users.

### Communication

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| 📄 | `*t*e*ll` | `t` | `cmd_tell` | Send message to user(s). Bare number works too. `te` = tell-all. Enters BUILD state. Semi-colon sends inline: `t 3;hello`. |
| 📄 | `im*p` | `im` | `cmd_im` | Show users building messages to you (impending). Empty line = `im`. |

### Information

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| 📄 | `w*ho` | `w` | `cmd_who` | Show users in your (sub)conference with state/status flags. |
| 📄 | `p*ort` | `p` | `cmd_port` | Show users with port/location info. Blocked if you are RP. |
| 📄 | `ev*erything` | `ev` | `cmd_all` | Combined who + port listing. Alias: `a`. |
| 📄 | `a` | `a` | `cmd_all` | Alias for everything. |
| 📄 | `al*l` | `al` | `cmd_all` | Also alias for everything. |
| 📄 | `tt*y` | `tt` | `cmd_tty` | Show your own WHO line. |
| 📄 | `le*ft` | `le` | `cmd_left` | List recent departed users with timestamps. K = killed. |
| 📄 | `us*ers` | `us` | `cmd_usrs` | Show count of users on conference. |
| 📄 | `nu*mber` | `nu` | `cmd_usrs` | Alias for users. |
| 📄 | `in*f*o` | `in` | `cmd_info` | Conference info: uptime, clock, time left, users, version, memory. |
| 📄 | `c*l*ock` | `c` | `cmd_clock` | Current time of day. |
| 📄 | `ti*me` | `ti` | `cmd_time` | Uptime, current time, and time remaining. |
| 📄 | `ru*n*time` | `ru` | `cmd_ru` | CPU runtime used vs. allowed. |
| 📄 | `ve*r*s*ion` | `ve` | `cmd_vers` | Show server version string. |
| 📄 | `dw` | `dw` | `cmd_dw` | Display current warning message. |

### State toggles

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| 📄 | `ra*ll` | `ra` | `cmd_ra` | Reject alls — stop receiving tell-all messages. |
| 📄 | `aa*ll` | `aa` | `cmd_aa` | Accept alls — resume receiving tell-all messages. |
| 📄 | `ig*nore` | `ig` | `cmd_ig` | Ignore user(s). `ig` alone = ignore all. Takes user number list. |
| 📄 | `ac*cept` | `ac` | `cmd_ac` | Accept user(s) — undo ignore. `ac` alone = accept all. |
| | `an` | `an` | `cmd_an` | Accept notification (join/leave/rename/kill events). |
| | `rn` | `rn` | `cmd_rn` | Reject notification. |
| | `rp` | `rp` | `cmd_rp` | Reject ports — hide your location. Permanent and irreversible. |
| | `rc` | `rc` | `cmd_rc` | Reject controls — filter control characters from messages. |
| | `oc` | `oc` | `cmd_oc` | Okay controls — allow control characters again. |
| | `rb` | `rb` | `cmd_rb` | Reject breaks — ignore break key. |
| | `ab` | `ab` | `cmd_ab` | Accept breaks — break key disconnects again. |

### Session

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| 📄 | `id` | `id` | `cmd_id` | Change name. `id NewName` or `id` alone prompts (CHG state). |
| 📄 | `ou*t` | `ou` | `cmd_out` | Go "out" — no messages until you press return (OUT state). |
| 📄 | `l*in*e` | `l` | `cmd_lin` | Line mode — hold messages, prompt for one command (LINE state). |
| 📄 | `by*e` | `by` | `cmd_bye` | Disconnect. Master bye = end conference. |
| 📄 | `st*op` | `st` | `cmd_bye` | Alias for bye. |

### Help

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| 📄 | `he*lp` | `he` | `cmd_help` | Show general help (same as `explain general`). |
| 📄 | `e*x*p*l*ain` | `e` | `cmd_exp` | Explain a topic from the help files. |


## Master commands

Available to the conference master (#0) and submasters created via `mty`.

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| 📄 | `ki*ll` | `ki` | `cmd_kill` | Disconnect specified user(s). Takes user number list. |
| | `mt*y` | `mt` | `cmd_mty` | Make user(s) into submasters. Takes user number list. |
| | `no*r*m` | `no` | `cmd_norm` | Normalize — remove submaster status, return their users to main. |
| | `gi*ve` | `gi` | `cmd_give` | Transfer users to a submaster. Format: `gi 4;2,5,7`. Also `gi 4;new`. |
| | `en*able` | `en` | `cmd_en` | Enable tell-alls in your subconference. |
| | `di*s*able` | `di` | `cmd_dis` | Disable tell-alls in your subconference. |
| 📄 | `wa*rn` | `wa` | `cmd_warn` | Set conference warning. `wa text` or `wa` alone prompts (WARN state). |
| | `be*l*ow` | `be` | `cmd_below` | Show users in subconferences below you. |


## Extended commands (Xcaliber II additions)

Privileged commands require `xyzzy name;password` first.

| Status | Pattern | Min | Handler | Description |
|--------|---------|-----|---------|-------------|
| | `xw*h*o` | `xw` | `cmd_xwho` | Extended who: all users + full IP/port, even if RP. Master/priv only. |
| | `xdl` | `xdl` | `cmd_xdl` | Display current language. Master/priv only. |
| | `xlang` | `xlang` | `cmd_xlang` | Change language environment. Master/priv only. |
| | `xyzzy` | `xyzzy` | `cmd_xyzzy` | Enable/disable privileges. `xyzzy name;password` or `xyzzy` to toggle off. |
| | `xtend` | `xtend` | `cmd_xtend` | Extend conference lifetime by N seconds. Priv only. |
| | `xreset` | `xreset` | `cmd_xreset` | Force-terminate conference. Priv only. |


## Unimplemented commands

These exist in cmdtab but map to `cmd_nimp` (prints "not implemented").

| Pattern | Min | Original purpose |
|---------|-----|-----------------|
| `bo` | `bo` | Bounce — ban ports from rejoining. |
| `nb*o` | `nb` | Unbounce ports. |
| `lb*o` | `lb` | List bounced ports. |
| `bu` | `bu` | Bounce user — ban by user number. |


## User states

From the WHO display and `cmd.c` state constants:

| Code | Constant | Meaning |
|------|----------|---------|
| `E/N` | `LOGIN_ST` | Entering name (just connected). |
| `C/N` | `CHG_ST` | Changing name (`id` with no argument). |
| `B-n` | `BUILD_ST` | Building message to user #n. |
| `B/A` | `BUILD_ST` | Building tell-all message. |
| `B/P` | `BUILD_ST` | Building message to multiple users. |
| `B/D` | `BUILD_ST` | Building to disconnected user. |
| `R/M` | `RECV_ST` | Reading a received message. |
| `COM` | `CMD_ST` | Executing a command. |
| `R/I` | `READI_ST` | Reading instructions (help). |
| `EXP` | `EXPL_ST` | Reading an explain topic. |
| `LIN` | `LINE_ST` | Line mode (messages held). |
| `OUT` | `OUT_ST` | Out — unreachable. |
| `---` | `IDLE_ST` | Idle — waiting for input. |


## WHO status flags

From `cmd_who_entry()` in `cmd.c`:

```
=>[#0] M COM R  CN  Username
^^     ^  ^  ^^ ^^
||     |  |  || |+-- N = accepting notification (AN)
||     |  |  || +--- C = rejecting controls (RC)
||     |  |  |+----- P = rejecting ports (RP)
||     |  |  +------ I/X/B = ignore relationship (I=you ignore them,
||     |  |                    X=they ignore you, B=mutual)
||     |  +--------- R = rejecting alls (RA)
||     +------------ state code (see above)
|+------------------ M = master or submaster
+------------------- => = this user receives new connections
```


## Command dispatch notes

- Command matching: `cmd_match()` uses `cmd_comp()` which does
  case-insensitive matching with `*` as "skip optional chars" markers.
- Empty input (bare return) dispatches as `im` (impending).
- After dispatch, if the handler didn't change state, user returns to
  `IDLE_ST`.
- Arguments are parsed by `cmd_argmap()`: comma-separated user numbers,
  with optional `;` separator for message body or subcommand args.


## Connection lifecycle

1. New TCP connection → `LOGIN_ST` → server sends welcome message
2. First connection becomes master; subsequent connections are regular users
3. User enters name → `IDLE_ST` → receives intro message
4. Commands processed in `IDLE_ST` (and some other states)
5. `bye`/`stop` → master: ends conference; others: disconnected
6. Break key → disconnects (unless `RB` set or in `BUILD_ST` for master)


## Transcript testing notes

Commands that need **multiple connections** to test properly:
- `tell` — need sender and receiver
- `ig`/`ac` — need two users to observe ignore behavior
- `who`/`port`/`everything` — more interesting with multiple users
- `left` — need someone to depart first, then check the list
- `users`/`number` — more interesting to verify count changes as users join/leave
- `mty`/`give`/`normalize`/`below` — subconference management
- `kill` — master disconnects another user
- `enable`/`disable` — affects tell-all behavior for subcon members
- `warn` — warning goes to all users

Commands that need **multiple connections** to verify effect:
- `id` — calls `con_change` which sends `namenot` to other users
- `ra`/`aa` — can set the flag solo, but need a tell-all sender to verify blocking
- `rc`/`oc` — need a sender with control chars to verify filtering
- `rn`/`an` — need another user to join/leave to see notifications appear or not
- `rp` — affects what others see in `port`; permanent, so test last
- `warn`/`dw` — `warn` sends to all users; `dw` only meaningful after a warning is set

Commands testable with a **single connection** (master):
- `tty`, `info`, `clock`, `time`, `runtime`, `version`
- `out`, `line`
- `help`, `explain`
- `bye`

Commands needing **special strategy**:
- `rb`/`ab` — break key handling; could test by sending telnet BRK
- `tell` with `BUILD_ST`: send command, then type message, then blank line
- `out`: user enters OUT state, needs bare return to resume
- `line`: holds messages, runs one command, then returns to idle
- `warn` with no args: enters WARN state, needs text + return
- `id` with no args: enters CHG state, needs new name
- `give`: complex syntax `gi N;list` — needs submaster setup first
- `xyzzy`: needs valid priv file configured on server
