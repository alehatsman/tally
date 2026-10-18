# 35 — Drift that nothing was looking for

**Status: shipped.** Two halves of
[#22](https://github.com/alehatsman/tally/issues/22), and deliberately
not the third.

[34](34-the-swap.md) closed the case where a relocation is *refused*. This
is the case where nobody asks. A view can be out of its window for
reasons no Move caused — [#20](https://github.com/alehatsman/tally/issues/20)
renumbered whole rows, a swap displaces its occupant by design — and
until now exactly one thing repaired that: **pressing the drifted view's
own pad.** The pad it is sitting on stays wedged until somebody happens
to press a different one.

So drift was repaired by luck and was invisible until something collided
with it. These are the two halves that change: **the collision repairs
itself**, and **the drift gets counted**.

## Half one: nothing holds a window it is not entitled to

`make_view` creates the window a new view belongs in —
`new-window -t rNN:col` — and that is the call that returns
`create window failed: index N in use`. It is the user-visible wedge: the
pad does nothing, every time, for as long as the squatter lives.

Two kinds of squatter, and the first was never recognised at all:

| what holds the index | before | now |
|---|---|---|
| a **corpse**, tagged with *this* address | reclaimed and respawned (`27`) | unchanged |
| a **corpse**, tagged with some *other* address | `index N in use`, forever | `new-window -k` replaces it |
| a **live view**, tagged elsewhere | `index N in use`, forever | parked, then the window is made |
| a pane with **no tally tag** | `index N in use` | unchanged — not ours to move |

The second row is the one that is easy to miss. `dead_view_at` finds a
corpse **by the address it claims**, so a corpse claiming a *different*
address was never matched by it and never replaced — the exact blind spot
`27` named one case of and did not generalise.

### Parked, not relocated

The obvious repair is the one
[#22](https://github.com/alehatsman/tally/issues/22) describes: *resolve
the squatter by its tag and move it to where it belongs.* **This module
cannot do that**, and the reason is a boundary worth keeping. A tag is an
address — `work:2:5` — and page names are **config**.
[`slot.rs`](../src/slot.rs) says it outright:

> addresses know about tmux geometry, and the tmux module knows nothing
> about pages

So `jump` cannot turn a tag into a `Place` without being handed the page
table, and handing it over to fix a collision would be the wrong way for
that dependency to point.

It does not need to. **The squatter does not have to go home; it only has
to get out of the way.** It is moved to the first free window index at or
above `PARK` — a number **above the addressable column range**, so:

- no pad can ever name it, so a parked view cannot wedge anything else
- it is still a live view, still reachable by id, still jumpable
- the lazy path brings it home on its next press, exactly as before
- the sweep below counts it, because parked *is* drifted

> ⚠️ **The same argument applies to [34](34-the-swap.md)'s swap and is not
> made there.** A swap puts the displaced occupant at the window the moved
> view vacated, which is inside the address space and may be a pad
> somebody wants — `34` says so, and calls it a wedge travelling sideways.
> Parking has no such cost. Whether the swap should park the occupant
> instead of trading with it is a real question and is **not decided
> here**: `34` shipped, this is a different call site, and changing a
> shipped decision deserves its own argument rather than a side effect of
> this one.

## Half two: the sweep counts and does not touch

Once, after the first time anchord answers — not at process start, where
`sessions` is empty and there is nothing to audit — every view's address
is compared with the window it is in, and the difference is **logged**.

```
tallyd: sweep: 01M57SZ0… claims 1:1:3, sits at r01:2
tallyd: sweep: 1 of 7 views are not where their address says
```

**Nothing is moved, on purpose.** The acting version is the one that
drains drift without a press, and it is the obvious thing to build; it is
held for the same reason [28](28-the-level-and-the-edge.md) is held on the
week of `attention.jsonl`:

> measuring the architecture as specified is worth more than measuring it
> already patched

There is no number for how often a view drifts. `34` shipped a mechanism
that displaces one on purpose; [#20](https://github.com/alehatsman/tally/issues/20)
was a generator that has since been pinned off; nobody knows what is left.
A sweep that repairs silently would answer the question by erasing it. A
sweep that counts is the first instrument this subsystem has ever had, and
the decision it feeds is whether the acting version is worth its blast
radius — **every `tallyd` restart rearranging panes on a live grid, at the
moment the daemon knows least.**

If the number is zero for a week, the acting sweep is not worth building
and this is the record of why.

### Where it runs, and why not in `jump`

In `tallyd`, because the comparison needs `Slot::place()` and that needs
the page table. `jump` is asked a question it can answer without knowing
what a page is:

> here is a list of sessions and the `Place` each one's view should be in;
> which are somewhere else?

`Tmux::audit` answers it with `relocation()` — the same pure function the
move path plans with, so the sweep and the repair can never disagree about
what *out of place* means.

## Scope

- `Jump` gains `audit`. `Tmux` is still its only implementor.
- `make_view` grows the squatter check. `relocation()`, `move_view`,
  `tag` and `view_pane` are untouched.
- **No change** to the grid, the colours, the chords, what a press does,
  or what a Move means.
- **Not done:** the acting sweep (held, above); resolving a squatter to
  its own address (needs the page table in `jump`, see above). Both stay
  on [#22](https://github.com/alehatsman/tally/issues/22).

## Validation

Against real tmux, in `tests/relocate.rs`, under all four server configs:

- a corpse claiming **another** address no longer wedges the pad
- a live view squatting on a pad is parked above the column range, the
  pad works, and the squatter is still alive
- a parked view still comes home when its own pad is pressed
- `audit` reports a drifted view and is silent about one in place
