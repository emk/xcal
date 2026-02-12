# Subconferences

Subconferences allow the master to partition a conference into
semi-isolated groups, each managed by a promoted "submaster." This is
the most complex feature in xcal and was added to the C clone after the
initial Christmas 1997 implementation (notes from February 1999 mention
"several additional hours" making subconferences work).

This document covers the data model, commands, visibility and
communication rules, lifecycle, edge cases, known bugs, and differences
from the original DCTS Xcaliber. All behavior was verified against the
running C server using `xcal-test`.


## Data model

There is no explicit "subconference" data structure. Membership is
defined entirely by a single field on each user:

```c
user.mstr    // index of this user's master
```

Plus four bit flags:

| Flag | Hex | Meaning |
|------|-----|---------|
| `MASTER_FLG` | `0x8000` | Conference master (user #0) |
| `SUBMST_FLG` | `0x4000` | Submaster (heads a subconference) |
| `GETNEW_FLG` | `0x2000` | Set at init but never read (vestigial) |
| `TELALL_FLG` | `0x1000` | Tell-alls enabled in this master's subcon |

And one field on the conference:

```c
con.new      // index of master who receives new connections
```

### The tree

Subconference membership forms a tree rooted at the main master:

```
Master #0  (mstr=0, MASTER_FLG)
├── User #1  (mstr=0)
├── Submaster #2  (mstr=0, SUBMST_FLG)
│   ├── User #3  (mstr=2)
│   └── Sub-submaster #4  (mstr=2, SUBMST_FLG)
│       └── User #5  (mstr=4)
└── User #6  (mstr=0)
```

A submaster exists in a **dual role**: they belong to their parent's
subconference (via `mstr`) AND they head their own subconference
(users whose `mstr` points to them). This dual role is the key to
understanding all the visibility and communication rules.


## Commands

### `mt` (MTY) — Make master terminal

Promotes one or more users to submaster status.

**Syntax:** `mt 2` or `mt 2,4,6`

**Rules:**
- Caller must be a master or submaster.
- Can only promote your own direct subordinates (`target.mstr == caller`).
- Cannot promote yourself.
- If the target is already a submaster, silently skips (no error, no
  re-notification).
- Target receives: "You are now a master terminal"

**Note:** `TELALL_FLG` is NOT explicitly set by `mty`. However, users
are initialized with `TELALL` enabled at connection time, so a freshly
promoted submaster will have tell-alls enabled unless they previously
ran `disable`.

### `gi` (GIVE) — Transfer users between subconferences

Moves users from the caller's subconference to another master/submaster.

**Syntax:** `gi 2;3,5,7` or `gi 2;new`

**Rules:**
- Caller must be a master or submaster.
- Recipient must be a master or submaster (else: "Must pass to master").
- Can only give your own direct subordinates (`target.mstr == caller`).
- Cannot give yourself.
- Recipient does NOT need to be in the caller's subconference — you can
  give users to a sibling submaster, upward to a parent, or to any
  master/submaster on the conference.
- Transferred users receive: "You are talking with #N: Name"
- Recipient receives: "You have been passed" followed by a WHO-style
  listing of transferred users.

**Verified behavior** — submaster gives users back to parent:
```
user1 (submaster): gi 0;2     → "Done"
user2:                         → "You are talking with #0: Explorer"
master:                        → "You have been passed\n  [#2] ... Bravo"
```

#### `gi N;new` — Transfer new-connection routing

The `=>` arrow in WHO marks which master receives newly connecting
users (stored in `con.new`, initially #0).

**Syntax:** `gi 1;new`

**Rules:**
- Caller must currently hold `con.new` (be the `=>` master).
- Recipient must be a master or submaster.
- Recipient receives: "You have been passed\n  New users"

**Verified behavior:**
```
master: gi 1;new  → "Done"
master: who       →   [#0] M COM CN Explorer    (no => arrow)
                    =>[#1] M --- CN Alpha       (=> moved to Alpha)
# New user connects:
user2: who        → =>[#1] M --- CN Alpha       (assigned to Alpha's subcon)
                      [#2]   COM CN Bravo
```

After `gi 1;new`, new connections go directly into submaster #1's
subconference. The main master doesn't see join notifications for
them (notifications are scoped to the subconference).

### `no` (NORMALIZE) — Remove submaster status

Demotes a submaster and returns their subordinates to the caller's
subconference.

**Syntax:** `no 2` or `no 2,4` or `no` (no args = normalize ALL your
direct submasters)

**Rules:**
- Caller must be a master or submaster.
- Can only normalize your own direct subordinates.
- Cannot normalize yourself.
- Target receives: "You are no longer a master terminal"
- Subordinates of the target are transferred to the caller via
  `con_give_map`.
- If the target held `con.new`, the new-connection role is also
  transferred.

**Verified behavior:**
```
master: no 1      → "Done"
                    "You have been passed\n  [#2] ... Bravo"
user1:            → "You are no longer a master terminal"
user2:            → "You are talking with #0: Explorer"
master: who       → all 4 users visible again, user1 no longer has M flag
```

### `below` — View subordinate users

Shows only users directly below you in the tree.

**Syntax:** `below` (no arguments)

**Rules:**
- Caller must be a master or submaster.
- Uses `FMT_BELOW` flag: shows users where `user.mstr == caller`, plus
  the caller themselves.

**Verified behavior:**
```
user1 (submaster): below  →   [#1] M COM CN Alpha
                               [#2]   --- CN Bravo
                               [#3]   --- CN Charlie
```

Note: `below` includes the submaster's own line. It does NOT show the
parent conference.

### `en` / `di` (ENABLE / DISABLE) — Control tell-alls

Each master/submaster independently controls whether tell-alls are
permitted in their subconference.

**Syntax:** `en` or `di`

**Rules:**
- Caller must be a master or submaster; regular users get "Command
  error."
- Sets/clears `TELALL_FLG` on the caller.
- The flag is checked on the **master of the sender**, not the sender
  themselves: when user #3 (whose `mstr` is #2) tries a tell-all, the
  code checks `TELALL` on user #2.
- When disabled, tell-all attempts get: "Tell-all not enabled"
- New users are initialized with `TELALL` enabled, so a new submaster
  starts with tell-alls allowed.

**Verified behavior:**
```
user1 (submaster): di
user2 (subordinate): te;msg  → "Tell-all not enabled"
user1: en
user2: te;msg                → "Message sent" (and user1 receives it)
```


## Visibility rules

### WHO

`who` uses `FMT_SUBCON = FMT_SAME | FMT_BELOW`. A user sees:

| Relationship | Visible? |
|---|---|
| Your master (`ix == user.mstr`) | Yes |
| Peers (same `mstr`) | Yes |
| Your subordinates (`ix.mstr == you`) | Yes |
| Users in a different subconference | No |
| Yourself | Always |

This means a **submaster sees both directions**: their parent's
subconference (via `FMT_SAME`: parent + peers) AND their own
subordinates (via `FMT_BELOW`). A regular user sees only sideways
and up.

**Verified — three-level nesting:**
```
Master #0: who  → #0 Explorer, #1 Alpha (submaster)
User #1:   who  → #0 Explorer, #1 Alpha, #2 Bravo (sub-submaster)
User #2:   who  → #1 Alpha, #2 Bravo, #3 Charlie
User #3:   who  → #2 Bravo, #3 Charlie
```

User #3 cannot see #0 or #1 — they are isolated at their level.

### `who N` — View another submaster's subconference

The original DCTS help file (`give.hf`) says: "If you type WHO4, the
users in #4's subcon. will be listed." This feature is **not
implemented** in the C clone. Both `who 1` and `who1` filter to show
only user #1's individual line within your own subconference view.

### PORT

`port` uses the same `FMT_SUBCON` scoping as `who`, plus port info.
Subconference-scoped.

### EVERYTHING / ALL / A

`a` uses `FMT_ALL` — shows **all users on the conference** regardless
of subconference membership, with both status and port info. This is
how a master can see users in nested subconferences they can't reach
with `who`.


## Communication rules

### Tell (targeted)

A user can send a targeted tell to user `ix` only if:
- `ix` is their master (`ix == user.mstr`), OR
- `ix` has the same master (peer: `ix.mstr == user.mstr`), OR
- `ix` is their subordinate (`ix.mstr == user`)

Otherwise: "Invalid argument(s)"

**Verified:** User #3 (in sub-submaster #2's subcon) cannot tell
master #0 or submaster #1 directly.

### Tell-all

A tell-all reaches the same set as targeted tells (master, peers,
subordinates), excluding:
- Yourself
- Users ignoring you
- Users in OUT state
- Users rejecting alls (RA)

The tell-all is additionally gated by the `TELALL` flag on **the
sender's master**. If the master has disabled tell-alls, the sender
gets "Tell-all not enabled."

**Verified — isolation between subconferences:**
- User #2's tell-all reaches user #1 (submaster) and user #3 (peer)
  but NOT master #0.
- Master #0's tell-all reaches user #1 (subordinate) but NOT users
  #2/#3 (in a different subconference).

### Notifications

Join, leave, rename, and kill notifications (`con_notify`) use the same
three-way subconference membership test as tell. A notification about
user `ex` goes to:
- `ex`'s master
- `ex`'s peers
- `ex`'s subordinates

Filtered by RN (reject notification) and OUT state.

**Implication:** If `con.new` points to a submaster, the main master
does NOT see join notifications for new users — they go to the
submaster's subconference only. The main master can still see all
users via `a` (everything).

### Warnings

Warnings (`con_warn`) are **global** — they go to ALL users on the
conference regardless of subconference. They cannot be ignored or
filtered.

### Ignore

`ig` with a user number can only ignore users in your subconference
(peers or subordinates). Notably, you cannot ignore your own master
via targeted ignore — `ix == user.mstr` is not in the allowed set.
However, bare `ig` (no args) ignores everyone including your master,
then accepts yourself back.


## Lifecycle

### Creating a subconference

1. Master runs `mt N` — user N becomes a submaster.
2. Master runs `gi N;3,5,7` — users 3, 5, 7 are transferred to N.
3. Users 3, 5, 7 now have `mstr = N`. They see N and each other.
4. N sees both the parent conference and their subordinates.

### Dissolving a subconference

Three ways:

1. **Explicit normalize:** `no N` — subordinates return to caller's
   subconference.
2. **Submaster disconnects or is killed:** `con_exuser` calls
   `con_normalize` before cleaning up the slot. Subordinates return to
   the submaster's parent.
3. **Submaster gives users back:** `gi 0;3,5,7` — the submaster
   manually transfers users to any other master/submaster.


## Edge cases

### Nested submasters (arbitrarily deep)

The code allows a submaster to promote their own subordinates to
sub-submasters via `mt`. There is no depth limit. The three-way
visibility check (same master / subordinate / superior) works
consistently at every level, creating strict isolation between
non-adjacent levels.

Verified with 4 users at 3 levels:
```
depth 0: Master #0        sees: #0, #1
depth 1: Submaster #1     sees: #0, #1, #2
depth 2: Sub-submaster #2 sees: #1, #2, #3
depth 3: User #3          sees: #2, #3
```

**Security note:** In languages with finite stack space, a
reimplementation should limit nesting to some reasonably large N to
prevent stack overflow and server DoS. Whether the original DCTS
system enforced such a limit is effectively unknowable this long after
shutdown, but it's a reasonable restriction in a reimplementation.

### Kill by submaster

A submaster can only kill their **direct subordinates** (`user.mstr ==
caller`). They cannot kill sub-subordinates (users under their own
submasters). To reach a sub-subordinate, the submaster would need to
normalize the intermediate submaster first.

**Verified:** Submaster #1 can `ki 3` (direct subordinate). The
killed user receives "You are no longer connected to conference."

### Non-master command rejection

Regular users who try master commands (`mt`, `en`, `di`, `gi`, `no`,
`below`) get "Command error" — the generic `cmderr` response, not a
specific permission message.

### `con.new` transfer on normalize

When a submaster holding `con.new` is normalized, `con_normalize` sets
bit 0 in the transfer bitmap as a sentinel. `con_give_map` then sets
`cp->new = USER(cp, who)->mstr` — the recipient's parent. For
single-level nesting (the common case), this correctly returns `new`
to the main master. For deeper nesting, `new` skips to the
grandparent, which may not be the intended behavior.


## Known bugs in the C implementation

### 1. `msg_type` set on wrong variable in `con_give_map`

**File:** `con.c:635`

```c
nf = msg_alloc(strlen(MSG(tlkwith)));
msg_type(nf, M_NOTIFY);              // line 630: correct
...
mp = msg_alloc(strlen(MSG(passed)));
msg_type(nf, M_NOTIFY);              // line 635: BUG — should be mp
msg_write(mp, MSG(passed));
```

The "You have been passed" message `mp` never gets type `M_NOTIFY`.
It retains the default `M_MSG` (0) type, so it's delivered as a
regular message rather than a notification. This is cosmetically
wrong — the message cannot be suppressed by RN — but has no serious
impact since the recipient is a master who was deliberately given
users.

### 2. `GETNEW_FLG` is vestigial

`SETGETNEW` is called once (at master initialization) and the
`GETSNEW` macro exists, but neither `GETSNEW` nor `GETNEW_FLG` is
ever tested anywhere. The actual new-connection routing uses `con.new`
exclusively. The flag is dead code.

### 3. Bit-0 sentinel collision in `con_give_map`

`con_normalize` uses bit 0 in the user bitmap as a sentinel to signal
"transfer the new-user role." `con_give_map` checks `MAPBIT(map, 0)`
and if set, transfers `con.new`. If user #0 were ever explicitly in
the bitmap for a different reason, this would collide. In practice
user #0 is the main master and is never "given" to anyone, so this
is safe but architecturally fragile.

### 4. `who N` subconference view not implemented

The original DCTS help text (`give.hf`) says "If you type WHO4, the
users in #4's subcon. will be listed." The C clone does not implement
this. `who N` filters to show user N's individual line within the
caller's subconference view, which is the same as the standard `who`
argument behavior (filter by user number). This is a **missing feature**,
not a bug per se, but it means there's no way for a master to view a
submaster's subconference from above except via `a` (everything), which
shows all users without subconference grouping.


## Summary of message notifications

| Event | Who sees it | Message |
|-------|------------|---------|
| `mt N` | User N | "You are now a master terminal" |
| `gi M;2,3` | Users 2,3 | "You are talking with #M: Name" |
| `gi M;2,3` | Master M | "You have been passed\n  [#2]...\n  [#3]..." |
| `gi M;new` | Master M | "You have been passed\n  New users" |
| `no N` | User N | "You are no longer a master terminal" |
| `no N` | Caller | "You have been passed\n  [users...]" |
| `no N` | Returned users | "You are talking with #C: Name" |
| Submaster disconnects | Returned users | "You are talking with #P: Name" |
| Submaster disconnects | Subcon members (AN) | "Ex-user at #N: Name" |
| Submaster killed | Subcon members (AN) | "User killed at #N: Name" |
