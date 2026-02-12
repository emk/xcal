# Xcaliber — Rust i18n Plan

> **Status:** Draft. Not finalized.

Embed all localizable strings and help files into a single static binary
using Project Fluent for message formatting and `rust-embed` for help
content.

**IMPORTANT:** Prioritize matching the original output over modern localization. If it says "1 user(s)" in the original, it should stay that way.

## Background

The C server has two parallel systems:

1. **Lang files** (`lang/*.lf`) — ~70 keyed message strings with
   `%d`/`%s` printf placeholders, compiled by a Perl script (`mklang`)
   into a position-indexed binary blob (`.lx`). Translations exist for
   `en_us`, `fr`, `de`, and `ga`.
2. **Help files** (`help/*.hf`) — plain-text reference pages, indexed by
   a binary TOC (`exptoc.x`) compiled from `exptoc.txt`. English only.
   See `docs/EXPLAIN.md` for the full help system writeup.

These serve different purposes and don't need to share an i18n framework.

## Proposed approach

### Message strings → Fluent (`.ftl`)

Convert the lang file keys to [Project Fluent](https://projectfluent.org/)
format. The mapping is mostly mechanical:

```
# lang/en_us.lf (current)
speak=<CR>Speak!<CR>
newuser=<CR>New user at #%d: %s<CR>
numusrs=<CR>%d user(s)<CR>
timewrn=<CR><BEL><BEL>You have about %d %s left<CR>
secsing=second
secplur=seconds
```

```ftl
# locales/en-US/messages.ftl (proposed)
speak = Speak!
new-user = New user at #{ $number }: { $name }
num-users = { $count } { $count ->
    [one] user
   *[other] users
  }
time-warning = You have about { $count } { $count ->
    [one] { $unit }
   *[other] { $units }
  } left
```

Benefits over the `.lf` format:
- **Proper pluralization** — replaces the `secsing`/`secplur` hack with
  CLDR-aware plural rules (important for languages with >2 plural forms).
- **Named arguments** — `{ $name }` instead of positional `%s`.
- **Translator-friendly** — `.ftl` is an established format with editor
  support and tooling.

The `<CR>`, `<SP>`, `<BEL>` escapes and line-ending concerns move to the
transport layer (the telnet/websocket output code), not the message
definitions.

### Help files → embedded at compile time

Help content stays as plain text. No Fluent, no translation framework.

```rust
#[derive(RustEmbed)]
#[folder = "help/"]
#[include = "*.hf"]
#[include = "exptoc.txt"]
struct HelpFiles;
```

At startup, parse `exptoc.txt` to build a topic → filename index, then
serve help content from the embedded data. See `docs/EXPLAIN.md` for
details on the TOC format and matching algorithm.

### Crate choices

| Crate | Role | Notes |
|-------|------|-------|
| [`fluent`](https://crates.io/crates/fluent) | Message formatting | 0.16.x, pre-1.0 but actively maintained. Used by the Rust compiler. |
| [`fluent-langneg`](https://crates.io/crates/fluent-langneg) | Locale negotiation | Picks the best available locale from a user's preferences. |
| [`rust-embed`](https://crates.io/crates/rust-embed) | File embedding | Embeds file contents at compile time. Single binary, no filesystem access needed. |
| [`unic-langid`](https://crates.io/crates/unic-langid) | Language identifiers | Parsing/matching of locale tags like `en-US`, `fr`. |

`i18n-embed` is another option that wraps `fluent` + `rust-embed`
together, but it adds complexity we probably don't need. Using `fluent`
directly is straightforward enough.

### Proposed file layout

```
locales/
  en-US/
    messages.ftl        # converted from lang/en_us.lf
  fr/
    messages.ftl        # converted from lang/fr.lf
  de/
    messages.ftl
  ga/
    messages.ftl
help/
  *.hf                  # unchanged
  exptoc.txt            # unchanged
```

### Runtime flow

```
startup:
  1. Embed all locales/**/*.ftl and help/**  at compile time
  2. Parse exptoc.txt → build help topic index (HashMap)
  3. Load FluentBundle for default locale (en-US)

per-user:
  4. On language change command, load/switch FluentBundle for that user
  5. Format messages via bundle.format_pattern()
  6. Help lookup: cmd_comp() match against topic index, return embedded .hf text
```

### Migration path

1. Convert `en_us.lf` → `en-US/messages.ftl` (mechanical, one-time).
2. Wire up `fluent` with embedded `.ftl` files.
3. Convert remaining `.lf` files.
4. Delete `mklang`, `mktoc`, and the binary `.lx`/`.x` files from the
   Rust build. (Keep in the repo for the C server.)

### What we're not doing

- **Translating help files.** The `.hf` content is English-only reference
  text written in 1997. Translating it is out of scope.
- **Using ICU4X.** We don't need locale-aware number/date formatting
  beyond what Fluent's built-in `NUMBER()` and `DATETIME()` functions
  provide.
- **Runtime locale file loading.** Everything is compiled in. A single
  binary with no external file dependencies.
