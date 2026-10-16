# 20 — The overview

Settled elsewhere and not reopened here: the grid is an 8×8 window onto a
64-row space, scrolled one row at a time by `▲▼`
([16](16-the-queue.md)); a pad tap is a jump; the right column is seven
`>` launchers and the kill modifier ([18](18-the-launch-column.md)). This
document is about **one button**, `Session` — `CC95`, `Top(5)`, and what the grid
means while it is held.

## The measurement

Every row the journal has ever seen a session in, by absolute row:

```text
row  1  cols 1 2 3
row  2  col  1
row  3  cols 1 3
row  4  col  4
row  6  col  2
row  8  cols 1 2
row 13  col  5
──────────────────
7 rows touched, of 64
```

Two facts, and they point in opposite directions.

**The space is nearly empty.** Fifty-seven rows have never held anything.
The 64-row address space was sized by the hardware — eight pages of eight
— and not by any observed need.

**And it is already discontiguous.** Row 13 sits five rows past the
nearest neighbour. Reaching it from row 1 is five `▼` presses across four
rows of nothing, and there is no way to know it is there: the logo says
*attention you cannot see*, and says nothing about where.

That gap is the whole case. The 64-row space is not a plane you survey,
it is a sparse set of islands you hop between, and the surface has no
gesture that shows the set.

> **Sparsity is the argument for a map, not against it.** A dense 64-row
> overview would be sixty-four lit pads and unreadable. Seven lit pads out
> of sixty-four is instantly legible, and every lit pad is a destination.
> This is the rare case where the thin data is the strong data.

## What it is

**Hold `Session` and the grid becomes the whole address space: one pad per
absolute row, in reading order.**

```text
pad (r, c)  ⟶  absolute row (r - 1) × 8 + c

 ┌────┬────┬────┬────┬────┬────┬────┬────┐
 │  1 │  2 │  3 │  4 │  5 │  6 │  7 │  8 │   ← page 1
 │  9 │ 10 │ 11 │ 12 │ 13 │ 14 │ 15 │ 16 │   ← page 2
 │ …                                      │
 │ 57 │ 58 │ 59 │ 60 │ 61 │ 62 │ 63 │ 64 │   ← page 8
 └────┴────┴────┴────┴────┴────┴────┴────┘
```

Sixty-four rows, sixty-four pads, no compression and no scrolling. The
grid row happens to be the page and the grid column happens to be the row
within it, which is exactly how an address is written — `page:row:col` —
but nothing depends on that reading. The rule is reading order.

**Tap a pad and the window lands there**: that row becomes the top of the
eight-row window, clamped at the bottom of the space.

## What a pad says

One pad stands for eight slots, so seven-eighths of the information is
thrown away. What survives is the worst thing in the row:

| rank | state in the row | the pad |
|---|---|---|
| 1 | something is asking | `Pulse(AMBER)` |
| 2 | something is producing output | `Pulse(GREEN)` |
| 3 | something is alive and quiet | `Static(GREEN)` |
| 4 | something exited non-zero | `Static(RED)` |
| 5 | something is lost | `Static(DARK_RED)` |
| — | nothing | `Off` |

Same colours, same meanings, no second language to learn. *Which column*
is the thing deliberately lost: you are choosing a row to stand in, and
the eight pads of that row answer the rest a half-second later.

Mute needs no rule here. A muted slot never resolves to `NeedsAnswer` at
all ([`signal.rs:263`](../src/signal.rs)) — it falls through to the state
underneath — so it cannot contribute amber to a row, for free and for the
same reason it cannot reach the queue.

## Where you are standing

The window is eight consecutive pads in reading order. It is marked by the
only thing that costs nothing: **a pad inside the window that would be
dark is `Static(DARK_GREY)` instead.**

No new colour, no new meaning — dim grey already says *available, not
urgent* on the arrows and the launchers. A pad inside the window that has
content keeps its true colour, because state outranks position.

> **The limit, stated rather than hidden.** A window whose eight rows are
> all occupied shows no marker at all. That is survivable because it is
> rare — the measured grid has at most three occupied rows in any window —
> and because in that case every pad you are looking at is lit, which is
> its own kind of answer. If it stops being rare, the marker needs a real
> colour and this paragraph is the record of why.

## The gesture

**Momentary. Press to show, release to hide, tap a pad to go.** Releasing
without tapping changes nothing.

This is the hold-then-tap scheme the surface already runs on
([`chord.rs`](../src/chord.rs)), and it is chosen over a latched mode for
one reason: the top row is built on *dark does nothing, lit does exactly
one thing, there is no state to remember* ([16](16-the-queue.md)). A
latched overview is a mode — the grid would mean two different things
depending on something invisible — and it is the only proposal here that
breaks that rule.

It also means `Session` earns its silkscreen. Ableton's Session view is
the zoomed-out one.

`Session` rests `DARK_GREY`, lit always, because there is always an
overview to show. White while held, like every modifier.

> **An earlier draft gave it `BLUE`, and that draft was wrong before the
> ink dried.** It reasoned that blue was the resting colour of the kill
> modifier and that `Session` joining it would make *blue means hold me*
> true. The kill button moved to red the same evening — *the colour of its
> consequence* — which retired blue and took the rule with it.
>
> Dark grey is the better answer anyway. It is what `▲▼` and the `>`
> launchers rest on, and it means the same thing on all of them:
> **available, not urgent**. The top row is five navigation buttons and
> they should look like five navigation buttons; what separates `Session`
> from its neighbours is that it is held rather than pressed, and that is
> a thing you learn from the grid changing under your thumb, not from a
> hue. Blue stays retired.

### What the rest of the surface does while it is held

| | |
|---|---|
| `▲▼◄►` | **stay live.** Scrolling under the map slides the grey marker across it, which is a free demonstration of what the marker means |
| `>` ×7 | **dark and inert.** Row 3 of the grid means *page 3* right now, not *row 3*, and a launcher that fires into the wrong row is the one mistake this column must not make |
| kill | holding it too is ambiguous: **neither fires**, the overview disappears, and the grid reverts — the same disarm the two-modifier rule already produces ([`chord.rs::modifier`](../src/chord.rs)) |

## Interfaces

```rust
// chord.rs
pub enum Gesture {
    …
    /// A pad tapped while the overview is up: put the window here.
    Goto { row: u8, col: u8 },
}
// Chords gains `overview: bool`, fed by Button::Top(5) down/up.
// While it is set: Grid ⟶ Goto, Launch suppressed, Tap/Chord suppressed.
// While it is set *and* a right-column modifier is held: nothing fires.

// model.rs
/// The worst state in absolute row `abs`, or `None` if it is empty.
pub fn row_summary(resolved: &BTreeMap<Slot, Resolved>, abs: u8) -> Option<SlotState>;
pub struct View<'a> { …, pub overview: bool }

// render.rs
/// One overview pad: the row's summary, and whether the window covers it.
pub fn overview(summary: Option<SlotState>, in_window: bool) -> Cell;
```

`frame_from` grows one branch, ahead of both the grid and the held
preview: if `view.overview`, draw sixty-four summaries and nothing else.

`tallyd` handles `Goto` the way it handles `Home` — set `offset`, log the
nav line — with `offset = ((row - 1) * 8 + col - 1).min(slot::MAX_OFFSET)`.

## The instrument

One journal line per press, so the next version of this document argues
from numbers rather than from this one:

```json
{"kind":"nav","via":"overview","to":13,"from":0,"lit":7}
```

`lit` is the count of non-empty rows at that moment. It answers the
question this design is a bet on — **is the space sparse, and does it stay
sparse** — and if `lit` sits at seven for a month, the honest follow-up is
not a better map but a smaller address space.

## Edge cases

| | |
|---|---|
| tap past the end | every pad maps to a real row — 64 pads, 64 rows, no dead pads |
| tap in the bottom page | `offset` clamps at `MAX_OFFSET`, so row 60 lands fifth rather than first. Honest, and the alternative is a window hanging off the end |
| tap the row you are already on | the window moves to put it first. Not a no-op, and not special-cased |
| anchord unreachable | the overview draws the last known rows, like the grid does. The logo is still the only thing that reports it |
| released mid-press | release emits nothing, as everywhere else |
| `Session` held at startup | no press was seen, so `overview` is false until one arrives. Self-correcting at the first release |

## Rejected

**A compacted list — occupied rows only, packed from the top.** Fewer dark
pads and a shorter reach. Rejected on precedent: [13](13-layout-review.md)
killed row-as-urgency because *the pads move while you reach for them*,
and this is that same proposal wearing the sparsity argument as a hat. A
map whose addresses are stable is worth more than a map that is short.

**A latched mode.** See *The gesture*. It is the one option that makes the
grid mean two things.

**A per-page map — eight pads, one per page.** Pages stopped being a thing
you navigate ([16](16-the-queue.md)), the measured usage crosses a page
boundary inside a single region of interest, and eight buttons is less
than the sixty-four the hardware is already holding.

**Brightness instead of hue for the window marker.** It needs a dim
variant of every colour in the palette, and the surface has gone out of
its way to carry meaning in hue because hue does not get tiring
([`render.rs`](../src/render.rs)).

## Amendments this forces

- [16](16-the-queue.md#the-top-row) — the top row is `▲▼◄►⌂`; `Top(5)`
  stops being `Home` and becomes the overview modifier
- [`render::home`](../src/render.rs) is **deleted**. It lit `Top(5)` dim
  when scrolled away from the top; `render::arrow` already says exactly
  that about `▲`, which is now the button that carries it. `Gesture::Home`
  survives, but `chord.rs` stops emitting it — see *Home moves to
  hold-`▲`*
- [`palette.rs`](../src/surface/palette.rs) — the `WHITE` comment still
  cites `crate::render::page`, removed in M8. Stale, found here, fix with
  this change
- [`addr.rs`](../src/surface/addr.rs) — `Top`'s doc says *four that carry
  nothing*; it has been three since `⌂`, and stays three after this

## Home moves to hold-`▲`

`Session` cannot be two things, so `⌂` leaves it. It does **not** get
folded into the overview, because the one jump the data describes — row 13
back to row 1 — is worth a single press.

**`▲` scrolls one row on press. Held past the threshold, it snaps to row
1.** There is no tap-versus-hold ambiguity to resolve, because a tap is
not withheld waiting to find out: the scroll happens immediately and
keeping the finger down does more. Nothing is deferred, nothing is
guessed, and you feel the arrival because `▲` goes dark the moment you
are there.

It fires **once** per press. Autorepeat would arrive at the same row by a
longer road and would need a rate nobody has measured.

> **The clock does not go in `chord.rs`.** That module is pure — presses
> and releases in, gestures out, no `now` parameter — and that purity is
> what lets the four chord rules be tested without a device or a
> timeline. A hold is a *timed* gesture, so the timer lives where the
> clock already is: `tallyd`'s 16 ms loop, which has `elapsed` in hand on
> the same line it calls `feed` ([`tallyd.rs:394`](../src/bin/tallyd.rs)).
>
> The consequence, stated so nobody looks for it: `Gesture::Home` stays in
> the enum and `navigate` keeps handling it, but **`chord.rs` never emits
> it**. It is the one gesture the state machine does not produce.

```rust
// tallyd
/// How long `▲` must be held before it means *all the way up*.
const HOME_HOLD: Duration = Duration::from_millis(500);
// Track the press instant for Top(1); on each tick, if it is still down
// past HOME_HOLD and has not fired, emit Home once and latch until release.
```

500 ms is a guess with a reason — short enough not to feel like waiting,
long enough that a lazy tap cannot reach it — and it is a named constant
because the number is a hardware question, not a design one.

`▼` gets no hold. The bottom of a 64-row space where seven rows are
occupied is not a destination, and the overview reaches the deep rows in
one gesture anyway. If *jump to the deepest occupied row* turns out to be
wanted, this is the symmetric place to put it.

## Validation

- ⬜ holding `Session` with seven occupied rows shows seven lit pads and
  the grey run, and is readable without counting
- ⬜ tapping row 13's pad from row 1 puts the window there in one gesture
- ⬜ releasing without tapping leaves the grid exactly as it was
- ⬜ `Session` + kill held together disarms visibly — both white, grid
  plain
- ⬜ `▲▼` under the overview slides the grey run and nothing else
- ⬜ `>` is dark and does nothing while the overview is up
- ⬜ holding `▲` for 500 ms from row 13 lands on row 1, once, and the
  button goes dark under the finger
- ⬜ 500 ms is not reachable by a lazy tap, and not slow enough to feel
  like waiting
- ⬜ a week of `via:"overview"` lines: is it reached for, and does `lit`
  move
