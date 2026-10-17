# 29 — What the loop holds

**Status: spec, then built.** It changes no behaviour, no frame byte and no
dependency. It is a structural review ([the four findings](#the-four))
written down before it was applied, because three of the four touch the one
file this project has already been bitten in twice.

Every module that decides something here is pure, and
[09](09-testing.md#the-shape-the-suite-ended-up-in) is right that this is a
consequence of where the boundaries were drawn rather than of discipline.
The boundaries were drawn everywhere except one place.

## The shape of the problem

`tallyd::main` is 425 lines holding **23 `let mut`** in a single scope. The
tell is not the length — it is the helper signatures:

```text
goto(row, col, &mut offset, &grid, &mut journal_out)
navigate(gesture, &mut offset, head, queued, &pages, &tmux, &mut journal_out)
beat(device, elapsed, bpm, &mut clock_started, &mut next_clock)
art_press(&mut reel, &mut repaint, button, down)
corner_event(&mut corner, &mut chords, &mut hand, button, down, elapsed)
```

Five functions taking between four and seven of `main`'s locals back by
`&mut`. That is a struct, passed one field at a time. Each of those
extractions has a comment saying it was lifted out because `main` was at
the cognitive-complexity cap — which is true, and is the symptom being
treated rather than the cause.

**The cost is not aesthetic.** `tallyd.rs` has 7 tests against `model`'s 50,
because the loop body is not callable. And this project has now paid twice
for bugs of exactly one shape — *two correct modules, run in the wrong
order* ([09](09-testing.md#3-the-gestures--pure-and-the-reason-they-are),
[22](22-the-mode-button.md), [21](21-the-arts.md)) — which is the shape a
425-line scope of loose mutable state is built to produce and unable to
test.

## The four

1. The loop's state has no names.
2. The journal is one thing, held as two.
3. `queue` writes its one predicate three times.
4. `model` and `render` are a cycle.

Each is addressed below. None of them is a rewrite; the total is
subtractive.

## 1. Four names, not one struct

The temptation is a `Daemon` with 23 fields, and it is wrong: it would make
every helper take `&mut self` and say nothing about which half of the state
it touches. The right granularity is already written in `main`'s own
comments, which group the locals into four clusters and explain each
grouping in prose. The fix is to let the type system hold what the comments
hold.

| name | holds | the invariant it carries |
|---|---|---|
| `Link` | `device`, `last_try`, `last_reply`, `repaint` | a device is **present or absent**, and absence is a normal state ([08](08-lifecycle.md#who-starts-it)); the next frame after any (re)connect is **total** |
| `Beat` | `bpm`, `next`, `started` | pulses are sent when due and **dropped when missed**, never caught up ([17](17-the-tempo.md)) |
| `Waiting` | `head`, `queued` | both are **last tick's**, because they are what the finger was looking at when it landed |
| `Grip` | `corner`, `hand`, `home` | the timed and multi-press gestures, which live here precisely because `chord.rs` is pure and has no `now` ([20](20-the-overview.md)) |

`Grip` takes two methods and not one, and the split is the thing to notice:
`aim` handles `▲` **before** the art is offered the press, `feel` handles
`CC19` **after**. That ordering is not tidiness — it is what the art
swallows and what it does not. A held `▲` scrolls the window, the window is
not what an art is covering, and the grid underneath is where you want to
land when you leave it. Collapsing the two into one method moves `▲` to the
far side of the art and silently changes what the surface does, which is how
this refactor nearly shipped a behaviour change under a no-behaviour-change
claim. It was caught by reading `art_press`, not by a test.

`Link` is the one that earns the most. The rules for dropping a device are
currently four separate edits to three locals, spread across 60 lines of
`main`: a failed write drops it, silence past `DEVICE_SILENCE` drops it, a
reconnect sets `repaint`, and both drop paths must also clear `chords` or
the first tap after a replug is a kill. That last one is a comment today.
It becomes a line in one method.

**`repaint` goes inside `Link` and not beside it.** It is not a rendering
preference; it is the answer to "has this device seen a frame from this
process", and the only correct place to set it is wherever the device
arrives or departs. Keeping it as a loose `bool` is what makes it settable
from five places, one of which is the art.

### What stays a local

`offset`, `sessions`, `reachable`, `grid`, `purgeable`, `chords`, `journal`,
`reel`, `art_wake`, `config`/`pages`, `tmux`, `anchor`, `watchers`,
`started`, `last_reconcile`. These are either owned-once-and-read or already
have a type that names them. Moving them into a struct would be motion, not
structure.

## 2. The journal is one thing, held as two

`main` holds a `Journal` (the resumed map of who is asking) and a
`journal_out: Option<std::fs::File>` (the append handle), and threads the
second through `goto` and `navigate` as a parameter. The rule that **a write
error retires the sink** is then implemented twice: once in `log_nav`, once
inline in the reconcile block.

A rule written twice is a rule that will be written a third time, and the
third one will forget the `flush`. The flush is not incidental — the journal
is deliberately unbuffered, because the transitions it exists to count are
exactly the ones a `SIGKILL` would strand in a buffer.

So `Journal` owns its sink:

```text
Journal::open(path, pages) -> Journal     resume the map, open the handle
  .tick(resolved, pages, now)             this tick's transitions, to the file
  .jumped_next(now, at, waited, queued)
  .jumped_back(now, moved)
  .jumped_to(now, to, from, lit)
```

`record(resolved, pages, now, out)` survives beside `tick`, over
`impl Write`, because that is what the 18 transition tests assert against. It
is not a second path: both call one private `transitions`, and `tick` is that
against the file this journal owns. `tick` destructures `self` so the map and
the sink are borrowed apart — the map is what the lines are computed from,
the sink is where they land, and nothing needs both at once.

Each write is best-effort and self-retiring: on error it reports once and
closes the sink, and the daemon keeps running — the grid lighting up matters
more than the log, which is the existing policy and is not being revisited.

**The line builders stay free functions over `impl Write`.** That is what 18
tests assert against, and they must keep asserting against a string rather
than a file. The methods are a thin layer over them; the sink is the only
new thing.

## 3. One predicate, written three times

`queue::head`, `queue::depth` and `queue::oldest` each spell out the same
eligibility test:

```text
not being looked at  &&  state is NeedsAnswer  &&  not muted
```

`head` and `depth` are character-identical. The module's own documentation
argues the opposite of what the code does:

> **one predicate, read by the renderer and by the press.** Two would
> eventually disagree, and a button that lights up and then does nothing is
> one you press twice.

There are three. They agree today by coincidence of having been written in
one sitting.

Worse, `oldest` deliberately **omits** the `viewing` half — the tempo counts
a request you are sitting in front of and have not answered, where `►` must
not offer you where you already are ([17](17-the-tempo.md)). That is a real
and argued difference, and today it is visible only by diffing three
function bodies. One shared iterator makes the difference explicit at the
two call sites that want it:

```text
answering(waiting, pads)              -> impl Iterator   the predicate, once
answering(..).filter(not in viewing)  -> head, depth
answering(..)                         -> oldest
```

## 4. The vocabulary belongs below both

`render` imports `model::{Consequence, Effect}`; `model` imports
`render::{self, Health}`. Both are pure, so nothing breaks — but neither is
below the other, and it is the only cycle in the crate.

It is also why `act` depends on the 1,899-line `model` to reach a
four-variant enum.

`Pad`, `Effect`, `Consequence` and `effect()` are the shared vocabulary of
three modules: `model` builds the pads, `render` colours them, `act` carries
them out. They move to `effect.rs`, which depends on `chord`, `signal` and
`slot` and on nothing else. The result is a DAG:

```text
surface  <-  slot, chord, signal
             effect  <-  chord, signal, slot
             render  <-  effect, chord, signal, surface
             model   <-  effect, render, anchor, config, signal, slot, surface
             act     <-  effect, model, anchor, chord, config, slot
```

`Pads` stays in `model`. It borrows the resolved grid and the config, which
is the layer it belongs to, and it remains **the one place a `Pad` is
built** — the property that keeps the preview and the press from drifting,
and the one this change must not weaken.

`Health` stays in `render` and `model` keeps importing it. That edge is not
a cycle once the other direction is gone, and it is the right direction:
composition depends on rendering.

## What this change deliberately did not do

> ✅ **The last three bullets were done afterwards, in a second pass**, and
> [the section after them](#the-second-pass) records what that cost. They
> stay written as they were, because the reason for deferring them was
> good and is the reason they were a separate diff: a refactor that claims
> no behaviour change has to be readable as one.

- **No behaviour change.** Not one pad changes colour, not one byte changes
  on the wire, not one journal line changes shape. The validation below is
  built entirely around proving that claim rather than asserting it.
- **No new dependency.** The budget argued in `Cargo.toml` stands.
- **No `main` that is only short.** Splitting the loop body into
  `tick(&mut self)` for its own sake would move the 425 lines rather than
  structure them, and would buy no test that cannot be bought today.
- **`art.rs` is not touched.** 2,084 lines is size, not complexity: a leaf
  with zero coupling and 30 tests. A directory is free whenever it is next
  edited, and it is not being edited now.
- **The three address grammars stay three.** `slot::parse` is canonical,
  `journal::parse_address` is a documented superset and argues for itself,
  and `cli::address` re-walks the text to build an error message a human
  reads. That third one could delegate; it is not in this change's scope and
  it is not where a bug lives.
- **`anchor::Frame` and `jump::View` keep their names**, colliding with
  `surface::Frame` and `model::View`. Renaming them is cheap and is
  unrelated to the four findings; doing it here would make the diff that
  must be reviewed for behaviour change bigger for a reason that is not
  behaviour.

## The second pass

Three of the deferred items and one stale sentence, taken together once
the four findings had landed and been merged. Each is small; the point of
listing them is that none of them is a behaviour change either, and the
same evidence applies.

**`cli::address` asks `slot::parse` first.** The three-part form is that
function's grammar, so what a human may type and what a `tl.slot` label
may carry can no longer drift. The hand-rolled walk survives below it and
runs only for an address that grammar rejected — and only to say *which*
part was wrong, which is the whole reason `cli` had its own copy. The
two-part `row:col` form is this command's own convenience and never
becomes a label. One new test asserts the agreement in both directions.

**`anchor::Frame` is `anchor::Packet`; `jump::View` is `jump::Pane`.** In
a crate whose main currency is a `Frame` of 81 cells and whose window onto
the address space is a `View`, both names were taken. `watch.rs` imported
one of each. anchor's wire vocabulary still says *frame* where it is
quoting anchor's protocol — a frame header is a frame header — and only
the values this crate holds were renamed. `Pane`'s own field went from
`pane` to `id`, because a `Pane` with a `pane` field reads worse than the
`View` it replaced.

**`art.rs` is a directory.** One file per piece, plus `rng`, `colour` and
`sprite` for what they share; `mod.rs` keeps `Piece`, `Reel`, the
dispatcher and the suite, which is mostly about the reel rather than about
any one piece. Each piece's entry point is now `paint`, named by its
module instead of repeating it.

The split paid for itself once, immediately: `SEED` lived in `life` and is
read by all four stateful boards, which a 2,084-line file had no way to
show. It now lives in `rng`, next to the generator it seeds. That is the
argument for splitting a file that was never *wrong* — not that the code
improves, but that the couplings become addressable.

**And one sentence in [08](08-lifecycle.md) that was never true.** It
described the signal handling as a `tokio::signal` stream raced against
the reconcile tick. There is no async runtime here and the dependency
budget never bought one; the handler is a relaxed atomic store that the
16 ms loop reads. The design the sentence described was sound, which is
why nobody caught it — it is the ordinary way a document goes wrong, by
being written next to the code instead of from it.

## Validation

The whole point is that this is provably inert, so the evidence is the
suite plus two things it cannot see. It covers both passes: the renames
and the `art/` split are exactly as inert as the four findings were, and
are held to the same bar.

0. **Counts, for both passes.** 362 tests before; 370 after the four
   findings; **371** after the second pass. Every addition is a test for
   something new, never a rewrite of one that stopped passing.
1. **The existing suite, unchanged in count and in content.** No test is
   rewritten to match a new shape. Tests move with the code they test —
   `effect`'s tests follow it out of `model` — and are otherwise identical.
   A test that has to change to keep passing is a behaviour change, and is
   the signal to stop.
2. **New tests for what the new types claim**, and only that:
   - `Link` drops the device on silence and on a failed write, and both
     paths set `repaint` and clear the chord state.
   - `Beat` sends a due pulse, drops a missed one, and restarts from
     `clock_start` after the device comes back.
   - `Journal`'s sink retires itself on a write error and the daemon
     survives it.
   - `answering` agrees with the three bodies it replaces, on a table that
     includes the muted and the being-looked-at cases.
3. **`cargo clippy` stays clean** under the fleet lint block, including
   `cognitive_complexity` — which `main` is currently at the cap of, and
   which is the one lint that will object if this change moves lines instead
   of structuring them.
4. **The hardware checklist in
   [09](09-testing.md#5-the-whole-thing--against-a-real-device-by-hand)**,
   run end to end. Four of its items are the drop/replug paths that `Link`
   now owns, and they are the ones no unit test reaches. **Press `►` before
   testing a pad.**

> ⚠️ **A green suite proves this did not change behaviour the suite can
> see.** The frame is bytes and the bytes are tested; the *order the modules
> run in* is the half that has twice been wrong here and is still the half
> no unit test holds. That is the argument for `Link` and `Grip` existing at
> all — they turn two of those orderings into methods with one caller — and
> it is not an argument that they are sufficient.
