# Bounce commands

The bounce system allows the main master (#0) to ban users from
rejoining after being killed or disconnecting. It was implemented in
the original DCTS Xcaliber but **not** in the Mark II C clone. The C
clone registers all four commands in `cmdtab` but routes them to
`cmd_nimp` ("not implemented").

## Original DCTS behavior

From `help/master.hf`, the original help text:

### `bo` (BOunce) — Ban by port

Prevents users on specified ports from rejoining the conference.
Syntax: `bo 2/260, 2/261, 2/262`. The bounce does **not** disconnect
them — it only prevents rejoining. If the user is still on the
conference, `bu` is easier.

Main master only (not submasters).

### `bu` (Bounce User) — Ban by user number

Convenience command: bounces the ports of specified users, so the
master doesn't need to look up port numbers first. Equivalent to
running `port`, noting the port numbers, then `bo`.

### `nb` (NBounce) — Unban

Opposite of `bo`. Unbounces specified ports. `nb` with no arguments
unbounces all ports.

### `lb` (LBounce) — List bans

Lists all bounced ports. Ports bounced via `bu` show as `*hidden*` to
prevent the master from using the ban list to discover port-to-user
mappings — preserving the `rp` (reject ports) privacy contract.

## The `*hidden*` detail

When a user has rejected ports (RP), bouncing them via `bu` must not
leak their port identity back through the `lb` listing. The original
system shows `*hidden*` for BU-bounced ports, so the master can ban an
annoying user without learning where they're physically sitting.

The original DCTS Xcaliber had no privilege escalation mechanism (the
`xyzzy` system and `xwho` are Mark II additions — confirmed by their
absence from all help files and by the Mark II authorial voice in the
`cmd_xyzzy` comments). So on the original system, `*hidden*` was an
absolute protection: there was no way for the master to see an RP
user's port, even after bouncing them.

## Conference-scoped lifetime

Bounces last until the conference terminates. On DCTS, conferences ran
on a countdown timer (`time`, `runtime`), so all bans are inherently
temporary. The use case is escalation beyond `kill`: "this user keeps
coming back after being killed, so ban their terminal for the rest of
this cycle." When the conference expires and a new one starts (possibly
with a different master), the slate is clean.

## Why it wasn't implemented in Mark II

From `README.md` footnote [16]:

> "It's not entirely clear how this would be done; bouncing by TCP port
> is not very useful at all. On the other hand, bouncing by IP address
> is probably too powerful, and in any case breaks the syntax of the
> original Xcaliber. I may implement it purely for the sake of
> completeness, although I suspect it would be useless in practise."

On DCTS, port numbers mapped to physical wall jacks — deterministic
and stable. On TCP, the source port is ephemeral and changes every
connection. IP addresses are the closest equivalent, but they're a
coarser identifier (multiple users behind NAT) and a more powerful
weapon (blocks an entire household or campus subnet).

## Design notes for a web reimplementation

In a web context, "port" no longer maps to a physical location, but
the need to ban a persistent nuisance for the remainder of a
conference session is still real. Some considerations:

**IP bans (conference-scoped)** are the most direct translation. They
preserve the original semantics: banned for this conference cycle,
clean slate on the next one. The main downside is NAT — multiple
legitimate users may share an IP, especially on campus or corporate
networks (exactly the environment most likely to host a conference).

**Session token / cookie bans** would be more precise than IP — they
target the specific browser session rather than everyone behind the
same NAT. Easy to circumvent (clear cookies, open incognito), but the
original port-based bans were also circumventable by physically moving
to a different terminal. The bar just needs to be high enough to deter
casual reconnection.

**A combined approach** — ban by session token primarily, with IP as
an optional escalation — mirrors the original's `bu` vs `bo`
distinction. `bu` (ban user) targets the session; `bo` (ban
address/origin) targets the network identity and is the heavier tool.

**The `*hidden*` / RP privacy contract** still applies: if a user has
opted to hide their identity information, the ban mechanism shouldn't
leak it through the ban list. In a web context, this means `lb` might
show a session hash or just `*hidden*` rather than an IP address for
users who were bounced via `bu`.

**The `nb` / `lb` commands** translate directly — list active bans,
remove specific ones or clear all. The conference-scoped lifetime
means there's no persistent ban database to manage.
