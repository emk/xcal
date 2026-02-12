# Xcaliber Mark II — Help System (`explain`)

The online help system serves `.hf` (help file) text in response to the
`explain` and `help` commands. All lookup goes through a single code path
in `xhelp.c`, driven by a table-of-contents file that maps topic patterns
to filenames.

## Source files

| File | Role |
|------|------|
| `help/*.hf` | Plain-text help files, one per topic group |
| `help/exptoc.txt` | Human-editable TOC: maps topic patterns to `.hf` filenames |
| `help/mktoc` | Perl script that compiles `exptoc.txt` → `exptoc.x` (binary TOC) |
| `help/exptoc.x` | Compiled binary TOC used at runtime by the C server |
| `xhelp.c` | C help lookup: opens binary TOC, scans entries, returns file handle |
| `cmd.c:cmd_exp()` | The `explain` command handler — calls `help_lookup()`, delivers text |
| `cmd.c:cmd_help()` | The `help` command — just calls `cmd_exp()` with the `helptop` lang string (normally `"help"`) |

## How `explain <topic>` works

1. `cmd_exp()` strips leading whitespace from the argument, then calls
   `help_lookup(TOCFILE, topic, &fp)`.
2. `help_lookup()` opens `exptoc.x` from the help directory
   (`$XCALHELPDIR` or the compiled-in default), verifies the `XM2` header,
   and scans entries sequentially.
3. For each entry, it tests the user's input against every topic pattern
   using `cmd_comp()`. On the first match, it opens the corresponding
   `.hf` file and returns `HELP_OK` with an open `FILE*`.
4. `cmd_exp()` reads the file, converting `\n` → `\r\n` and `\t` → the
   tab-expansion string, wraps it in a message, and delivers it to the user.
5. On `HELP_NF` (no match), it sends the `cantexp` lang string
   ("Can't explain that"). On any other error (bad TOC, missing `.hf`),
   it sends `fnfmsg` ("Help-file not available").

The `help` command is just sugar: `cmd_help()` calls
`cmd_exp(cp, who, flag, MSG(helptop))`, where `helptop` is a lang string
set to `"help"` in `en_us.lf`. This resolves via the TOC entry
`general.hf:he*lp,gen*eral`.

## The TOC format (`exptoc.txt`)

Each line maps a filename to one or more topic patterns:

```
filename.hf:pattern1,pattern2,...
```

Comment lines starting with `#` and blank lines are skipped.

### Topic patterns and `cmd_comp()`

Patterns use `*` to mark **abbreviation points** — positions where the
user's input is allowed to stop. The matching algorithm (`cmd_comp()` in
`cmd.c`) is case-insensitive and works as follows:

```
cmd_comp(user_input, pattern):
    walk both strings left to right:
      - if chars match (case-insensitive): advance both, clear ok flag
      - if pattern char is '*': set ok flag, advance pattern only
      - otherwise: no match
    at end of user_input:
      - if pattern is also exhausted: match
      - if ok flag is set (or next pattern char is '*'): match
      - otherwise: no match
```

Examples for `t*e*ll`:
- `t` — matches (stops at first `*`, ok flag set)
- `te` — matches (stops at second `*`)
- `tel` — matches
- `tell` — matches (both strings exhausted)
- `tells` — no match (user input longer than pattern)
- `tx` — no match (`x` ≠ `e`)

### Special TOC entries

Most entries follow the obvious pattern (`who.hf:w*ho`, `tell.hf:t*e*ll`),
but a few are notable:

| Entry | Purpose |
|-------|---------|
| `eambig.hf:e` | Bare `e` is ambiguous (`explain`? `enable`? `everything`?). Maps to a disambiguation message. |
| `everything.hf:ev*ery*thing,a` | Bare `a` resolves here (not ambiguous, since `accept` requires `ac*`). |
| `general.hf:he*lp,gen*eral` | The `help` command target — two aliases for the same file. |
| `mark.hf:mark* brown` | Easter egg topic with a space in the pattern. |
| `usnu.hf:us*er*s,nu*m*ber` | Two commands share one help file. |

### Orphan files

`unavail.hf` exists on disk but has no `exptoc.txt` entry, making it
unreachable through normal lookup. It may have been intended as a
placeholder for topics awaiting writeup.

## Binary TOC format (`exptoc.x`)

The binary format compiled by `mktoc` is designed for easy sequential
scanning in C. It is **not needed** for a Rust reimplementation — parse
`exptoc.txt` directly instead.

For reference:

```
Offset  Content
------  -------
0       'X' 'M' '2'            magic header
3+      entry*                  zero or more entries
        0xFF 0xFF               end-of-file marker

Entry:
        len:u8  filename:[u8; len]
        count:u8
        (len:u8  topic:[u8; len]) * count
        0xFF                    entry terminator
```

Topic strings are stored lowercased with `*` markers intact, ready for
`cmd_comp()` matching.

## Reimplementation notes

- Embed `exptoc.txt` and all `.hf` files into the binary (e.g.
  `rust-embed` or `include_str!`).
- Parse `exptoc.txt` at startup to build a lookup structure. Two options:
  1. **Expand at init**: for each pattern, generate all valid prefixes at
     `*` boundaries and store in a `HashMap<String, &str>`. Fast lookup,
     slightly more memory.
  2. **Match at query time**: store patterns as-is and run `cmd_comp()`
     logic on each query. Faithful to the original, trivially correct.
- The `cmd_comp()` algorithm is ~15 lines of Rust. It's reused for
  command dispatch too, so it belongs in a shared module.
- The `.hf` file content needs `\n` → `\r\n` and tab expansion on output,
  same as the C version. (Or store pre-processed if the output format is
  known at build time.)
