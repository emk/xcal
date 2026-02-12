# who_subcon — `WHO N` subconference view (not implemented)

## What we're testing

The original DCTS help text (`give.hf`, line 12) says:

> If you type WHO4, the users in #4's subcon. will be listed.

This would let a master view a submaster's subconference from above
without using `a` (everything). We tested whether the C clone
implements this.

## What actually happens

The C clone treats `who N` as a simple user-number filter on the
caller's existing subconference view. It does not switch to showing
user N's subconference membership.

| Command (from master) | Expected per `give.hf` | Actual |
|---|---|---|
| `who` | Explorer, Alpha | Explorer, Alpha |
| `who 1` | Alpha, Bravo, Charlie (Alpha's subcon) | Alpha only |
| `who1` | Same | Alpha only |
| `who 2` | Bravo (in Alpha's subcon) | "Invalid argument(s)" |

`who 2` fails because Bravo is in Alpha's subconference and not
visible to the master via normal `FMT_SUBCON` scoping.

## Why

In `cmd.c`, `cmd_who` passes `FMT_SUBCON` (= `FMT_SAME | FMT_BELOW`)
to `cmd_who_raw`. When arguments are provided, the code checks whether
each specified user is visible under the caller's own subconference
rules. It never reinterprets the argument as "show me this submaster's
group." The `who N` feature described in `give.hf` was either a DCTS
original that was never ported, or an aspirational description.
