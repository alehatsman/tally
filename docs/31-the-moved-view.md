# 31 — A moved session takes its view with it

> **Amends [22](22-the-mode-button.md) and settles the case
> [27](27-the-slot-collision.md) parked.** 22 says a move touches no pane.
> 27 left a *live* pane at a slot "to get its own doc if it ever bites".
> Tracked as [#15](https://github.com/alehatsman/tally/issues/15).

## The bug

Move a session from row 3 to row 2. Press it: the agent is still in tmux
session `r03`, window 1. Then spawn on the pad it left and press that:
nothing happens.

One cause. Move is **one label write** (`Context::put_down`,
`src/act.rs`) and a view's physical address is a row **session** plus a
window **index** (`row_session`, `new_window_args`, `src/jump.rs`;
[24](24-the-tally-server.md)). Nothing on the Move path relocates the pane.

| | what happens |
|---|---|
| 1 | `put_down` writes `tl.slot`. The pane stays in `r03:1`, tagged `@tally_slot=1:3:1`. |
| 2 | Press the moved pad. `view_pane` finds the pane **by id**, re-tags it `1:2:1` and renames its window — and returns it. It never asks where the pane *is*. The view is re-labelled, not relocated. |
| 3 | Spawn on the vacated pad and press it. By id: miss. By address (`dead_view_at`): the squatter is alive, and its tag now says `1:2:1`. Miss. |
| 4 | `make_view` → `new-window -t r03:1` → `create window failed: index 1 in use`. `jump` returns the `Err` and the grid cannot show it. |

After hop 2 the index `r03:1` is held by a pane **no address claims**: 27's
collision tagged a *different session*; this one is tagged a *different
slot*. Neither of 27's two tests matches it, which is why 27's fix did not
reach it.

### Measured

`~/.local/state/tally/tallyd.log:19186-19191`, the owner's own daemon:

```
tallyd: launch 1:4:1 -> 01M57RW5Y6CS04K70291262S90 in .../bright-frontend
tallyd: Move 1:4:1 picked up 1:4:1
tallyd: Move 1:3:1 moved 1:4:1 -> 1:3:1
tallyd: jump 1:3:1 -> 01M57RW5Y6CS04K70291262S90
tallyd: tmux new-window: create window failed: index 1 in use
```

Three presses, three failures. `grep -c "in use"` on that log is 77: 27's
corpses are most of them, this is the rest. The squatter was `01M55R3E…`,
moved from `1:3:1` to `1:2:1` at line 19183 and jumped to at 19185.

Line 19183 is also the *other* shape of this. `1:2:1` had been Stopped at
19180, so `r02:1` held a **corpse** — and the jump that followed succeeded
only because nothing compared the pane's window with the place. A fix that
relocates must not turn that into a new failure.

**tmux 3.6b, scratch socket** (`-L`, not tally's):

```
move-window -s r03:1 -t r02:1      r02 absent    can't find session: r02
break-pane  -s %0 -t r02:1         r02 absent    can't find session: r02
move-window -s r03:1 -t r02:0      index held    index in use: 0
move-window -s %5 -t r02:5         already there same index: 5
move-window -s r03:1 -t r02:5      r03 had one window   → r03 is destroyed
move-window -k -s %0 -t r02:1      dead pane there, sole window of r02
                                    → exit 0, %0 is r02:1, r02 survives
show-options -g detach-on-destroy  on
```

A control-mode client attached to `r03` when its only window moved out:

```
default                                        clients after:   (none)
set-option -t r03 detach-on-destroy off first  clients after:   client:r02
```

Placeholder, for a destination session that does not exist yet
(`new-session -d -s r02 -P -F '#{window_id}'`, then `move-window -k`, then
`kill-window` on the printed id): the pane lands at `r02:1` with
`base-index` 0 **and** 1; at 1 the `-k` has already replaced the
placeholder and the `kill-window` answers `can't find window`, which is
ignored.

## Goal

Moving a session moves its view. A pad you vacate is a pad you can use.

## The design

### Where: tallyd, not `act.rs`

`put_down` stays exactly what its doc comment says: **one label write**.
`act.rs` knows nothing about tmux — "jumping is tmux's business and not
anchord's" — and that is a property the tests can see, so it is kept.

What changes is the caller. `Done::Moved` gains the session it moved, and
`tallyd` follows the write with one call, the way it already follows a tap
with `tmux.jump`:

```
Chord Move → put_down → one tl.slot write → Done::Moved { session, from, to }
                                            └→ tallyd: tmux.relocate(session, at, place)
```

22's sentence — "no tmux change, no pane is touched" — is true of the
*gesture* and false of the *daemon*, and now says so.

### Also lazily: `view_pane`, by id

The by-id branch gains the same check, between "respawn if dead" and `tag`.
Both paths call **one function**, `relocate`. The eager call is what makes
the vacated pad usable *before* anyone presses the moved one. The lazy one
is the repair for everything the eager call cannot see:

- a `tallyd` that was down, or restarted, when the move was made
- a relocation that failed (below)
- a move made before this shipped — the state the owner's log is full of

A happy-path jump pays nothing: the pane's current session and window index
come from the listing `view_pane` already takes (`list-panes -F` gains
`#{session_name}` and `#{window_index}`, both never empty, so the `|`
position rule of [27](27-the-slot-collision.md) is untouched).

### `relocate`

```
at the address already          → nothing. tmux refuses it anyway ("same index")
destination window holds…
  nothing                       → move-window
  a dead pane                   → move-window -k
  a live pane                   → nothing. 27's decision stands
destination session missing     → new-session -d (placeholder), move-window -k,
                                  kill the placeholder by id
always, before the move         → set-option -t <source> detach-on-destroy off
no view exists for the session  → nothing. Not an error
```

Then, on the eager path, `tag` — which rewrites `@tally_slot`, the pane
title and the window name to the new address. That is the half of the
bug that made the squatter invisible.

**Why `-k` is allowed here and was refused in 27.** 27 rejected
`new-window -k` because it "does not ask whether the occupant is dead; a
flag is not where the decision lives". Here the decision lives in
`relocate`, from the listing, and `-k` is only ever passed after it has
been made. And the argument that carried 27 carries this: the corpse is
only replaced because *you asked this slot to hold something else*. The
pad it sat on was given away — Move's legal target is a pad with **no
live session** ([22](22-the-mode-button.md#move-is-the-missing-verb)) — so
whatever the corpse viewed is either ended or alive at another address,
where the next press makes it a fresh view. It costs the corpse's
scrollback and its `claude --resume` line ([26](26-the-dead-pane.md)); the
conversation id is still in anchord.

`-k` is atomic, which `kill-window` followed by `move-window` is not: when
the corpse is the only window of its row, killing it destroys the row
session and the move then fails. Measured above.

**Why a live pane at the destination is left alone.** The same reason as
27: a live `anchor attach` is something somebody may be reading. It can
only be there if an earlier relocation was skipped or failed. The view
stays where it is, reachable by id, and nothing is created or destroyed.
A swap of two live squatters (`swap-window`) would resolve it and is a
bigger decision than this bug; **not done**.

**Why `detach-on-destroy off` on the source.** Moving a row's last window
out destroys the row session — tmux does it, nothing here kills anything —
and with the default a client sitting in it is **detached**. In `jump`
that client is listed *after* `view_pane`, so it would be gone, the list
would be empty, and `ensure_terminal` would open a second terminal. Set on
the *session*, not the server, and only on the one about to be left. If it
survives (other columns remain) the option stays: a row session can only
ever lose a client to a neighbour instead of to a detach. Best-effort, like
every other `set-option` here.

**Why a placeholder for a missing row.** `move-window` and `break-pane`
both need the target session to exist; tmux has no verb that adopts a pane
into a session it creates. `make_view`'s trick — create the session around
the first view — cannot apply, because the view already exists and a
second `anchor attach` is a different pane with a different id. So the
row is created the plain way, which is what `make_view`'s doc complains
about (a stray shell), for the length of one `move-window`, and the stray
is killed by id. The shell never gets a prompt anyone sees.

### Failure policy: best-effort, both paths

The move was already written. Failing it for a tmux reason would have no
way to show you; refusing the *next jump* would take away a view that is
perfectly reachable by id. So `relocate` is best-effort in the lazy path,
for `tag`'s reason ("a view that reads badly, not a view that cannot be
found") — and the eager path **logs** its error to `tallyd.log` and
carries on. The cost is stated: a failed relocation leaves the vacated pad
wedged until the moved session is next pressed, which repairs it.

## Edge cases

| case | behaviour |
|---|---|
| moved, never pressed, tallyd up | relocated at the move; the vacated pad is free at once |
| moved, never pressed, tallyd down or relocation failed | **still wedged** until the moved session is pressed (lazy path) |
| moved before any view existed | `view_for` misses → nothing. Not an error |
| moved within its row | `move-window`; the source session is the destination |
| moved across rows, destination row exists | `move-window`; source destroyed if it was the last window |
| moved across rows, destination row absent | placeholder, then as above |
| destination holds a corpse | replaced (`-k`) |
| destination holds a live pane | left; the moved view is re-labelled where it stands, as today |
| view is itself dead (detached) | still moved — a corpse keeps its place in the grid; the next press respawns it in place |
| a client is on the moved view | follows it; the source is set not to detach it |
| two moves, one after the other | each is its own relocate; a chain `A→B`, `B→C` is two single moves |
| the pad was moved *onto* another page | `Place` is the absolute row; page is not a tmux concept |
| squatter already drifted (old state) | repaired by the lazy path when its session is pressed; **not** found by address — a pane whose tag names an address other than its window is not searched for |

## Scope

- No change to `put_down`'s one write, to anchord, the grid, colours or
  chords.
- `Done::Moved` gains a field; its `Display` is unchanged.
- `Jump` gains one method. `Tmux` is its only implementor.
- **Not done:** `swap-window` for two live squatters; finding a drifted
  squatter by its own tag; a startup sweep that relocates every view to
  its label. Each is a bigger decision than this bug.

## Validation

- **Unit**, extending the `list-panes` fixtures in `src/jump.rs`: the
  listing carries `session_name` and `window_index`, `|`-separated; the
  parse holds position; `relocate`'s plan is a pure function of the
  listing — at address, empty destination, dead occupant (`-k`), live
  occupant (refused), absent session (placeholder);
  the argument lists put `-k` only where the plan says so and always aim
  at the **pane**, never a bare `rNN:`.
- **End to end**, on a scratch socket (`TALLY_TMUX_SOCKET`), never
  tally's, driving the real `Tmux` against real tmux 3.6b. **Run, passing,
  and not committed** — it needs a fake `anchor` on `PATH` and a server
  with `remain-on-exit`, which is a harness this repo does not have and
  this bug does not justify. Seen:

  ```
  relocate, r02 absent, r03 keeps a neighbour   S1 r03:1 → r02:1, slot 1:2:1
  then a view for 1:3:1                         lands at r03:1      (symptom 2)
  relocate, r06's only window                   r06 gone, r07:1 there
  relocate onto a corpse at r02:2               replaced; corpse gone
  relocate onto a live pane at r02:3            both stay, no error
  press by id with a new place (lazy)           r05:1 → r05:4
  relocate a session with no view / at place    Ok, nothing
  ```

  The live-occupant row shows the edge the table above admits: the moved
  view is re-labelled where it stands (`r04:1`, tag `1:2:3`) and so is
  again a pane no address claims.
- **Not run when this was written:** against the owner's live `tallyd`.
  It has since been; see below.

## Measured on the live grid

2026-10-18, the owner's `tallyd` at `f0b4046` (`~/.local/bin/.tallyd.commit`;
the process started 17:49:27, a minute after the binary was written). Line
numbers are of `~/.local/state/tally/tallyd.log` as it stood that day; the
log has no timestamps and is not rotated by us, so the *text* is the
citation and the numbers are a convenience.

```
19472  Launchpad connected                  the last restart in the log (matched to
                                       the 17:49:27 process by being the last;
                                       there are no timestamps to say so)
19542  Move 1:2:1 moved 1:3:1 -> 1:2:1      cross-row
19555  Move 1:3:1 moved 1:3:2 -> 1:3:1      within-row
19587  Move 1:3:4 moved 1:3:3 -> 1:3:4      within-row
```

**Seen.** Three moves after the restart, **no `index N in use` caused by a
move**. (There were two that were not; see below.) After them
the three moved sessions' views sit at the window their label names
(`r02:1`, `r03:1`, `r03:4`, by `list-panes`). That is the bug's own
symptom, gone, for the two shapes that ran: a move within a row and a move
to another row. A spawn onto a vacated pad was **not** tried by this
check.

**Not seen, and the numbers that would flatter this:**

- **"77 → 0" is not the result.** 77 is `grep -c "in use"` over the whole
  log, and most of it is [27](27-the-slot-collision.md)'s corpses, fixed
  before this existed. The last one is at line 19221, which is *before the
  previous restart* (19277), so the quiet that follows it is not
  evidence about this change either. The comparison that means something
  is the three moves above, and before the fix one move in 19183 was
  followed by three collisions at 19188-19196.
- **The placeholder branch did not run.** The cross-row move went to `r02`,
  which tmux had held since before the restart
  (`list-sessions`: `r02` created at 1792336048). It is the destination-row
  **exists** shape. The path this document calls the riskiest, a missing
  row, a placeholder session, `move-window -k`, a `kill-window` on the
  placeholder, has run only on a scratch socket.
- **Corpse reclaim at a destination was not exercised on purpose.** `r02:1`
  may have held the corpse of the session Stopped at line 19373, or it may
  not: nothing recorded which, and the window it would have held is gone.
- **The blocked-relocation case ran, uninvited, and lost.** See next.

**What the run found that this document did not predict.** Within the hour
one view was at the wrong window with the right label: pane `%31`, tag
`1:1:3`, at `r01:2`. Not a Move: the corpse at `r01:2` was closed (who did it
is not identified; `tally purge` is the only code that closes a view), and
the owner's `~/.tmux.conf`
sets `renumber-windows on`, which this document and
[24](24-the-tally-server.md) both assume away (*"indexes do not renumber
when a sibling is killed"*). The siblings shifted down, so the lazy path
met the one row of the `relocate` table that is left broken, **a live pane
at the destination**: `1:1:3` was pressed while `1:1:4`'s view held index 3,
was refused, and was re-tagged where it stood. A press of `1:1:4`
afterwards moved *it* out of the way, and 3 was free again. The same two
presses in the other order succeed. That is the edge case the table above
admits, observed rather than constructed, and with a cause (a config option)
the spec did not know to look for.

**And it cost a press, twice.** Pad `1:1:2` held a live session with no
view (a detached spawn opens none), and `r01:2` was held by `%31`. Pressing
`1:1:2` therefore found no view by id, no corpse by address, and ran
`new-window -t r01:2`:

```
19630  tmux new-window: create window failed: index 2 in use     jump 1:1:2
19632  tmux new-window: create window failed: index 2 in use     jump 1:1:2
```

Both are after all three moves, and both are #17's symptom from a cause
that is not a Move. The owner then pressed `1:1:3`, and with 3 free `%31`
moved there; the next press of `1:1:2` opened its view at `r01:2` (`%36`).
So the repair this document specifies worked, by the route it specifies
(*the moved session is pressed*), and the wedge it admits was real until
then. Read from the log and `list-panes` afterwards; the presses were the
owner's. **The gate closed for the thing #17 fixed, on a grid that was still
colliding for another reason in the same hour.**
Tracked: [#19](https://github.com/alehatsman/tally/issues/19) the blocked
case, [#20](https://github.com/alehatsman/tally/issues/20) the renumbering,
[#22](https://github.com/alehatsman/tally/issues/22) finding drift that is
already there, [#21](https://github.com/alehatsman/tally/issues/21) the
end-to-end harness, which would have seen the renumbering and which a
fixture listing cannot.
