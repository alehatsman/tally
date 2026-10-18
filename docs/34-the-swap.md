# 34 — Two live views trade places

**Status: shipped.** This reverses one line of
[31](31-the-moved-view.md): *"a live pane at the destination is left
alone"*. It is not left alone any more. The two views **swap**.

[31](31-the-moved-view.md) shipped one case it knowingly left broken and
[#19](https://github.com/alehatsman/tally/issues/19) is the tracker it
eventually got. When a moved session's destination window is held by a
**live** pane, `relocation()` returned `None` — *stay* — and the caller
then tagged the pane where it stood. Two writes that disagree, and the
result is the exact state the whole of `31` exists to prevent: **a pane
whose `@tally_slot` names an address other than its window.**

## Why it was left, and why that stopped holding

`31` deferred it for a good reason and a stale one.

The good reason is [27](27-the-slot-collision.md)'s: **a live
`anchor attach` is something somebody may be reading**, and a relocation
is not the gesture that closes it. That still stands, and nothing here
closes anything.

The stale one is *"it can only be there if an earlier relocation was
skipped or failed"* — a rare, self-inflicted state. That was true when it
was written and is not true now.
[#20](https://github.com/alehatsman/tally/issues/20) produced a
**generator**: with `renumber-windows on`, closing any window pulls every
sibling to its right down one index, so a whole row of views is at the
wrong address **by tmux's doing, with no Move involved**. One refused move
then makes the next one's destination live, which is how a rare state
becomes a self-perpetuating one.

[33](33-the-server-holds-its-indexes.md) removed that generator by pinning
the option off. This removes the thing the generator was feeding.

## The decision

```
destination window holds…
  nothing                       → move-window
  a dead pane                   → move-window -k
  a live pane                   → swap-window        ← this document
destination session missing     → placeholder, move-window -k, kill the placeholder
```

`swap-window -d -s <pane id> -t rNN:col`. One call, atomic, nothing
created and nothing destroyed.

**Measured, tmux 3.6b**, because every other claim in this subsystem that
was not measured turned out to be wrong:

| | |
|---|---|
| a **pane id** as `-s` | ✅ resolves to its window, like `move-window` already relies on |
| across two sessions | ✅ `r01:1` ↔ `r02:3` |
| onto an index holding no window | ❌ `can't find window: 7` — which is the branch this is never used for |
| `-d` | the active window does not change: a relocation must not move anybody |

### What the other view gets

`P` lands at the address it claims. `O` — the live occupant — lands at
`P`'s old window.

**`O` is not thereby correct, and this does not claim it is.** `O` was by
construction a view already out of place, and its own `@tally_slot` is
untouched. What changes is that it is out of place somewhere that is now
*free*, instead of sitting on an address another view needs. The lazy path
repairs it on the next press of its own pad, exactly as before
([#22](https://github.com/alehatsman/tally/issues/22) is still the issue
for finding it without that press).

So the invariant this buys is narrow, and it is worth stating exactly
because the obvious wider version is **false**:

> **The view being relocated always ends at the address it claims.**

It is *not* true that no pad is left wedged. The swap does not reduce how
many views are out of place — it chooses **which** one is, and it always
chooses in favour of the view that was just moved. `O` lands at `P`'s old
window, which is itself an address some pad may want, so a wedge can
travel one address sideways rather than disappear.

That is still strictly better than what it replaces, in two ways that can
be checked:

- the relocation **accomplishes the move**, where before it accomplished
  nothing at all
- it stops lying about it. Before, two views were out of place *and* one
  of them wore an address it did not hold

**Draining the last one is not this document's job.** A view displaced by
a swap is repaired by the lazy path the next time its own pad is pressed,
exactly as any other drift is, and the thing that would repair it without
that press is the sweep in
[#22](https://github.com/alehatsman/tally/issues/22) — still open, and
now the only thing between this and *every view is at its address*.

### What it costs

**A client sitting in `O` follows `O`.** tmux swaps the windows'
positions, and a client attached to that window stays attached to it — so
somebody reading `O` sees its tab move. That is the surprise `31` was
unwilling to spend without a decision, and it is spent here deliberately:
the alternative is a pad that cannot be pressed at all, and `O` was
already at an address no pad claimed.

**Rejected: moving `O` first, recursively, toward its own address.** Same
end state when it terminates, and it does not always terminate — two
views each holding the other's index is a cycle, and the fix would need a
visited set, a depth bound and a decision about what to do when the cycle
closes. A swap *is* the two-element cycle, resolved in one call.

## The second half: an address is not written unless it is held

Independent of the swap, and worth doing even if the swap is one day
replaced.

`view_pane` used to discard the relocation's result —
`let _moved = self.move_view(…)` — and then tag unconditionally. So a
relocation that was refused **or that simply failed** was followed by a
write claiming the address anyway. That is the half of the bug that made
the squatter invisible (`31`), and the swap does not remove it: a
`swap-window` can still fail for a reason nobody predicted, and the next
thing to happen must not be a lie about where the pane is.

Now: **when the relocation did not succeed, `@tally_slot` is not
written**, and the refusal is logged.

- the pane keeps whatever address it last legitimately held
- the drift is visible to `tmux list-panes` and to anything that later
  looks for it, instead of being papered over
- the view is still reachable: `view_for` matches on `@tally_session`,
  which is always written

The rest of `tag` is unchanged. The session id, the conversation,
`remain-on-exit` and the dead-pane format are facts about *the pane*, not
claims about where it is, and a pane that failed to move still needs all
four.

## Scope

- `relocation()` returns a plan that is now an enum: `Move { replace,
  create_row }` or `Swap`. It is still a pure function of the listing, and
  still the whole table in one place.
- `move_view` grows one arm.
- `tag` takes whether the pane is known to hold the address, and skips
  one `set-option` when it does not.
- **No change** to `put_down`'s one write, to anchord, the grid, the
  colours, the chords, or to what a Move means on the surface.

## Validation

- **Unit**, in `src/jump.rs`: `relocation()` returns `Swap` for a live
  occupant and `Move` for every other row of the table; the argument list
  is `swap-window -d -s <pane id> -t rNN:col` and aims at a pane, never a
  bare `rNN:`.
- **Against real tmux**, in `tests/relocate.rs`
  ([#21](https://github.com/alehatsman/tally/issues/21)), under all four
  server configs: two live views trade places, `P` ends at its address,
  `O` ends at `P`'s old window, both are still alive, and the pad `P`
  vacated can then be spawned onto.
- The test that pinned the old behaviour — *"today S1 does not move: #19"*
  — is **rewritten rather than deleted**. It was written to fail the day
  this landed, and this is that day.
- **Not covered by the suite, and said rather than hidden:** the
  `SomewhereElse` branch. Every row of `relocation()`'s table now
  succeeds against real tmux — the swap closed the last one that did
  not — so there is no longer a *known* way to make a relocation fail
  from outside, and a test that forced one would be testing a tmux error
  this document cannot name. The guard stays as defence against the
  unforeseen, which is exactly what it was reached for: the bug it
  prevents was itself unforeseen. Attempted and rejected: relocating to
  row 0, which turns out to succeed and would have been a test asserting
  a failure that does not happen.
- **Not run:** with a client actually attached to `O`. The suite is
  headless, and what a client does on a swap is tmux's documented
  behaviour rather than ours. It is the one claim here taken from the
  manual instead of from a measurement.
