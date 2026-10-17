# 27 — A corpse must not hold the slot

> **Amends [15](15-views.md) and [24](24-the-tally-server.md).** 15 says a
> view is found by its session id. 24 made the column a **window index**,
> which is a *namespace* — and a dead view still occupies it.

## The bug

Kill the session on a pad whose view is open, then spawn a new one on the
same pad and press it. Nothing happens. The window keeps showing the dead
session's last output. Kill the pane by hand and press again: the new view
appears.

**Measured, tmux 3.6b** — `new-window -d -t r01:4` with index 4 occupied:

```
create window failed: index 4 in use
exit=1
```

**Measured on the live grid**, mid-bug:

```
r01:4 %11 dead=1 slot=1:1:4 sess=01M54NRGWHQX5WW4CCNVMZ6WY7
```

**Measured in `~/.local/state/tally/tallyd.log:2715-2736`**, one pair per
press:

```
tallyd: jump 1:1:4 -> 01M54P0VWK5HT3W3H00ZG79V4D
tallyd: tmux new-window: create window failed: index 4 in use
```

### The four hops

| | |
|---|---|
| 1 | The kill ends the PTY; `remain-on-exit` keeps the pane, so window `r01:4` stays — tagged with the **old** session (`docs/19`). |
| 2 | The new session is a new id. `view_for` (`src/jump.rs`) looks that id up and misses: the corpse carries the other one. |
| 3 | Miss means create. `make_view` → `new_window_args` targets `r01:4` (`src/jump.rs`). |
| 4 | Index in use. `view_pane` returns `Err`, `jump` dies on it, and the grid has no way to show you an error. |

Killing the pane by hand frees the index, which is why the workaround
works and why it looked like a tmux problem rather than ours.

**The identity lookup is right and stays.** What is missing is that the
*address* is also a lookup, and tally has been writing it on every pane
since 24 (`@tally_slot`, `Tmux::tag`) without ever reading it back.

## Goal

A pad press lands, whatever the previous occupant of that slot was doing.

## The design: find by slot, reuse the corpse

`view_pane` gains one fallback, between "found by id" and "create":

```
by id    → alive: use it; dead: respawn in place     (15, unchanged)
by slot  → dead pane at this address: respawn onto the new session   ← new
otherwise→ make_view                                  (24, unchanged)
```

The reclaim path is the **respawn branch that already exists** — the pane
keeps its id across `respawn-pane` (measured, `src/jump.rs`), so it
keeps its place in the row, its tab index, and its window. `tag` then
rewrites `@tally_session`, `@tally_claude`, the title and the tab name, in
that order, exactly as it does for any other view. Nothing new is invented
for this case; a lookup is added and an existing path is reached from it.

### Only the dead — decided

A **live** pane at the slot tagged with a different session is the same
collision (rebind the pad while its old view is attached) and it is
deliberately left failing. A running `anchor attach` is something you are
looking at; taking its window out from under you is a bigger decision than
this bug, and it gets its own doc if it ever bites. The reclaim therefore
tests `dead` as well as the address.

### Why not kill the window

`kill-window`-then-create, or `new-window -k`, both work and both are
shorter. Rejected:

- They destroy the dead line, which is the one thing a corpse is still
  **for** — it carries `claude --resume <uuid>` (`docs/26`). A respawn
  clears it too, but a respawn only happens when you asked for this slot
  to hold something else.
- `-k` does not ask whether the occupant is dead. The decision above has
  to live somewhere, and a flag is not where.
- An empty index between the two calls is a window for a reconcile tick to
  observe a row that is missing a column.

## The listing has to carry the address

`list-panes -F` is parsed by **position**, and the rule in
`list_panes_args` is that the possibly-empty field goes last — an unset
option prints as an empty string, and whitespace-splitting swallows it.
Two possibly-empty fields (`@tally_session`, `@tally_slot`) cannot both be
last.

So the listing moves to an explicit separator, `|`, and the parse to
`split('|')`, which holds position whether or not a field is empty.
Neither a ULID nor a `page:row:col` address can contain one.

## Edge cases

| case | behaviour |
|---|---|
| dead pane at the slot, same session | hop 1 catches it; the new path is never reached |
| dead pane at the slot, different session | **reclaimed** — this bug |
| live pane at the slot, different session | fails as today, by decision above |
| no pane at the slot | `make_view`, as today |
| untagged pane — anything tally did not create | never matches: an empty address is not an address |
| two panes tagged with one address | first wins; cannot arise — a pane never moves windows, and only one pane can hold an index |
| pane created before this shipped | has `@tally_slot` already (24); reclaim works on it |

## Scope

- No change to `src/act.rs`. The kill stays label, terminate, report.
- No change to anchor, the grid, the colours, or the chords.
- No new tmux call on the happy path: the slot comes from the listing
  `view_pane` already takes.

## Validation

- **Unit**: the listing format carries the address and is `|`-separated;
  the parse holds position with an empty session **and** an empty address;
  `view_for` still refuses to match an untagged pane; the new lookup
  matches a dead pane by address, refuses a **live** one at the same
  address, refuses an untagged one, and refuses an empty address.
- **End to end**, on the live grid: pad with a dead view at `1:1:4`, spawn,
  press. The window must show the new session without a hand kill, and
  `list-panes` must show the same `pane_id` with the new `@tally_session`.

### Measured, tmux 3.6b

A corpse tagged `@tally_slot=1:1:4` holding window `r01:4`:

```
%1|@1|1|OLDSESSION|1:1:4
```

Today's path, and the bug verbatim:

```
$ tmux new-window -d -t r01:4 …
create window failed: index 4 in use      (exit 1)
```

The reclaim, on the same corpse:

```
%1|@1|0|NEWSESSION|1:1:4
```

✅ Same `pane_id`, same window, same index. `pane_dead` back to 0, the tag
rewritten. Nothing was created and nothing was destroyed.

✅ The `|` parse holds with **two** empty tags: `%0|@0|0||` reads as pane
`%0`, not as a shifted line.

### Caught by this, not by review

`back()` was parsing the same listing with its own `split_whitespace`
(`src/jump.rs`, pre-fix). Changing the separator would have made every
origin look closed and `◄` silently stop working. Every read of the pane
listing now goes through `row`, which is why there is exactly one place
that knows the format.
