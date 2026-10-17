# 16 — The top row is a queue, not a map

Why the top row is `▲▼◄►` and not eight page buttons, and why `►` is the
most used of them. The grid it sits above is flat and the reasons are in
[12](12-the-layout.md); tracked as
[#8](https://github.com/alehatsman/tally/issues/8).

The obvious question to build a layout around is *where is the blocked
agent*. The first hour of real data says that is not the question.

## The measurement

`attention.jsonl` started recording real `declared` transitions on
2026-10-16. One hour, 54 transitions, five slots, time-weighted:

```text
 amber   time      share
  0      14.2m    23.4%
  1       9.4m    15.5%   <- the only time the grid picks for you
  2      12.7m    20.8%
  3       7.1m    11.8%
  4       6.7m    11.1%
  5      10.6m    17.4%   <- everything you have, waiting at once
```

**61% of the time there is a choice to make. 15% of the time there is
not.** The intuition a grouping layout rests on is that you need to know
*which project* is blocked before you jump — and that the honest answer
will turn out to be *"I press the amber one and go"*. It is not. There is
usually no *the* amber one.

And the choice is not close. When two or more pads are amber, the gap
between the longest wait and the shortest is **median 386 s, max 1382 s**.
Ordering is not a tiebreak between similar things; it is the entire
decision.

⚠️ **One hour, and an unrepresentative one** — five agents running while
one of them was being worked with. A quiet afternoon inverts the table.
One hour does not settle it; a week does, and
[§7](#the-instrument) is what makes that week measure the right thing.

## Why grouping is not the instrument

Triage is the bottleneck — that much is right. Arranging the grid is the
wrong tool for it, for four reasons, in increasing order of how hard they
are to repair ([12](12-the-layout.md#closed-and-why-they-stay-closed)
carries the same list as the closed-alternatives record).

**It answers "where" when the question is "which first".** With five pads
amber, knowing that row 4 is the work repo does not rank them. Grouping is
a *locating* aid, and at five lit pads out of sixty-four there is no
locating problem. There is an ordering problem, 61% of the time, and a row
orders nothing.

**Any scheme that relocates a pad is already closed.** The rule is *the
pads move while you reach for them* — you cannot build muscle memory
against a target that relocates, and a mis-press is a kill. It closes row
= urgency band outright, and it closes rows-as-projects too the moment the
rows are **derived rather than declared**: that relocates pads on a slower
clock, which is the same defect running at a speed that hides it.

**The grouping already exists, by hand, and costs nothing.** The five slots
that asked for attention this hour were `1:1:1`, `1:2:1`, `1:3:1`, `1:8:1`,
`1:8:2` — one project per row, lane 1 durable, lane 2 the overflow. That is
one project per row, as a convention over declared addresses. It is
stable because you chose it. What a grouping scheme adds on top is taking
the addresses away from you and letting the daemon assign them, which is
the one part [03](03-model.md#a-slot-is-not-a-session) says you cannot
have: *a grid you cannot learn is a grid you look at instead of feel.*

**It invents a capacity cliff.** Eight rows of seven lanes, for a workload
of five live sessions across four projects, two at most in any one. A row
that can *fill* needs signalling to say so — a limit flat addressing does
not have. The diagnosis grouping starts from is that 512 slots is capacity
for a workload of about twenty; a lane limit is that same error rotated
ninety degrees.

## The queue

```text
queue = slots whose state is NeedsAnswer
        minus muted
        minus the slot this client is already looking at
        ordered by the moment they started asking, oldest first
```

**Nothing is stored for this.** `Journal::asking` already maps a slot to
the moment it began asking, and it holds the *original* timestamp across a
tallyd restart — which was the fix for
[#7](https://github.com/alehatsman/tally/issues/7) and turns out to have
built the queue by accident. No new label, no new file, no new state in the
daemon.

**Mute is the starvation hatch.** A prompt you are never going to answer
sits at the head of the queue forever. Muting it is already the gesture for
*stop this slot raising attention*
([05](05-interaction.md#gestures)), and it already dies with the session it
was set on, which is right: it was that agent you stopped believing.

**The queue is per client.** Which slot you are "already looking at"
differs between two attached terminals, so the head does too — the same
per-client rule that [15](15-views.md#the-return-trip) applies to origin,
for the same reason.

## The top row

The silkscreen stops being decoration. `◄ ►` are the hardest pair on this
device to spend well — every job proposed for them has been thin — and
under a queue they become the most literal buttons on it: go to the thing
that is waiting, come back from it.

```text
  ▲      ▼      ◄      ►   │ ▣     ·    ·    ·  │  ◈
 row    row    back   next │ map    unassigned  │ health
  up    down               │
```

⚠️ **`Top(5)` was amended by [20](20-the-overview.md).** It is the
`Session` button either way, but it is now *held* rather than pressed:
holding it turns the grid into a map of the whole 64-row space. `⌂` moved
to **hold-`▲`** — the press still scrolls one row, and keeping the finger
down past 500 ms goes all the way up. The row below is the table as this
document wrote it; the `⌂` line is the one that changed.

**`▲▼` scroll by one row, not by one page.** The grid is an eight-row
window onto a sixty-four-row space, and it slides. A page boundary is not
a thing you can feel; a row is — you watch the layout move under your
finger instead of being cut to a different screen, and adjacent projects
stay adjacent across what used to be a hard edge.

Pages did not go away. They are still how an address is written, still
what `slots.toml` declares, and still what `tl.slot` carries. They stopped
being a thing you *jump between*, which is the only part that was costing
anything.

| button | dark when | lit | press |
|---|---|---|---|
| `▲` `▼` | there is no row that way | 1 dark grey, static | scroll one row |
| `◄` back | no origin recorded | 3 white, static | return to where you were standing |
| `►` next | the queue is empty | **9 amber, pulsing** | jump to the longest-waiting |
| ~~`⌂` home~~ | ~~already at the top~~ | ~~1 dark grey, static~~ | **now hold-`▲`** |
| `▣` map | never — it is always available | 1 dark grey, static | **hold**: the grid becomes all 64 rows |
| 3 others | always | — | nothing |

**`⌂` was the `Session` button, doing what its silkscreen says** — *return
to the overview*. It is the one move `▲▼` make tedious: sixteen presses
back from the bottom of the row space, for the place you are most often
going.

It turned out the silkscreen was promising more than a scroll. `Session`
now shows the overview *literally* — the whole address space at once —
and going to the top, which is a special case of going anywhere, moved to
holding the arrow that already goes that way
([20](20-the-overview.md#home-moves-to-hold-)). Being always lit makes it
the one exception to *dark does nothing*, and the exception is the point:
there is always a map to show.

It is a tap, not a hold: like `◄` and `►` it commits nothing, it only
moves you.

Every button here is **self-describing**: dark does nothing, lit does
exactly one thing, and there is no state to remember. The arrows report
*movement*, not *occupancy* — you navigate to an empty page on purpose, to
spawn into it, so a dark `▼` has to mean "there is no page below this one"
and nothing else. `next` reuses the
reserved amber and means what amber means everywhere else on the surface —
*look at me* — which is why it does not need a fifth colour.

**`next` crosses pages**, and scrolls the grid the smallest amount that
brings the pad it landed on into view — minimal, not centred, because `►`
is already moving your terminal and the grid lurching further than it must
is a second thing changing under you for no reason. That is what makes
losing the per-page amber indicator affordable: it told you *which page*
wants you so you could go there, and now you do not navigate to attention,
attention navigates to you. The logo still says *some page does*
([05](05-interaction.md#the-logo-is-the-health-light)).

### What is given up, stated plainly

[05](05-interaction.md#the-two-rows-that-are-not-slots) spends four
paragraphs on a rule learned the hard way: **exactly one page button is
bright, and it is always the one you are on.** It was paid for in real
confusion — *"I am confused which one is selected, the white one or the
turquoise one"* — and this layout deletes it. There is no current-page
readout any more.

That is defensible on this evidence and not on much else: **pages used,
70 : 2.** Page 2 has been visited twice, ever. A rule about telling eight
pages apart is a rule about a thing that is not happening. If pages become
load-bearing later, this is the first decision to revisit, and the reason
it was made is written here so it can be.

### What a navigation row costs the alternatives

**`►` cannot also be *commit the safe default*.** Approve-without-reading
has no button here. Narrowing it to the one answer that is safe to give
unread does not rescue it: it is still a button that answers an amber light
rather than a command ([12](12-the-layout.md#closed-and-why-they-stay-closed)).
Spending `►` on navigation retires the idea without needing a further
argument, because **navigation commits nothing.**

**There is no focus mode.** `▲▼` scroll, `◄` is back, `►` is next; none of
them is conditional on anything, so there is no mode to enter and nothing
to escape from.

**A tap on a pad keeps its one meaning**: jump. A second tap is not a
return and not a confirm — `◄` is the return gesture, on its own button,
visible, with an affordance.

## Back, and where you were

One rule, shared with [15](15-views.md#the-return-trip) and unchanged by
this document:

```text
Origin: client name -> pane id
recorded  when a client is about to be switched
          AND no trip is already open for it
          AND it is not already standing on the destination
one deep, first jump wins, cleared by `◄`
```

So `next → next → next → back` lands where you started, not three agents
deep. **The queue is a trip; home is one press.** Chained jumps were the
hardest open question [#3](https://github.com/alehatsman/tally/issues/3)
carried, and the queue answers it by construction:
there is no reason to walk back through agents when the only place you
want is the one you started from.

⚠️ **The first version of this rule said "and its current pane is not a
view pane", and it was wrong.** It assumed the place you jump *from* is
nvim. It is not: every pane this grid is used from is an anchor session,
so every candidate origin was a view, nothing was ever recorded, and `◄`
was dark forever.

Measured within an hour of shipping, by the `kind:"nav"` lines added for
exactly this purpose — two `{"via":"back","ok":false}` in a row, which is
what pressing a dead button twice looks like in a log. The instrument
caught the thing it was built to catch, on its first day, about its own
author's spec.

What that rule was protecting — walking back through agents one at a time
— is held by *first jump wins* instead, which does not care what kind of
pane you started in.

If the origin pane is gone, `◄` goes dark and the press does nothing.
Landing somewhere arbitrary is worse than not moving.

## The instrument

Navigation is recorded, because whether any of this works is a question
about what you actually press.

```json
{"ms":…,"kind":"nav","via":"next","slot":"1:2:1","waited_ms":219722,"queued":4}
{"ms":…,"kind":"nav","via":"back","ok":true}
```

`kind` absent means `"state"`, so every line written before today still
parses and the restart readback skips anything that is not a state line.
One reader change, no migration.

A week of this settles the grouping question with data rather than
opinion:

- **how many jumps came from `next` versus from a pad.** If you keep
  hand-picking, the queue is wrong and grouping deserves its second look.
- **how deep the queue was when you pressed.** If `queued` is almost always
  1, the 61% was an artefact of one unusual hour and the case for a flat
  grid is weaker than this document assumes.
- **how long the thing you chose had been waiting.** If you consistently
  answer something younger than the head, the ordering is wrong — recency
  or project may beat age, and that is the experiment worth running next.

This is the gate rather than a perceptual test of whether you can count to
the fourth row without looking, because it measures the decision rather
than a proxy for it. Counting rows only matters if rows are the answer, and
the first thing to find out is whether they are the question.

## What this unblocks

[15](15-views.md#what-is-cleared-to-build) held two of M7's three steps
behind #8. This clears both:

| held on | because | now |
|---|---|---|
| step 3, the return trip | the second tap on a pad was already spoken for | `◄` is the gesture; no tap is overloaded |
| step 2, adoption | `page:row:col` might be replaced one-way | addressing is unchanged and declared |

Neither was blocked on grouping being a good idea. Both were blocked on not
knowing, which is what an open layout question costs everything downstream
of it.

## Edge cases

| situation | behaviour |
|---|---|
| nothing asking | `►` dark, press does nothing |
| one asking, you are looking at it | `►` dark — the queue excludes your current view |
| one asking, you are elsewhere | `►` amber, press jumps |
| the head is muted | excluded; `►` reflects the queue you can act on |
| the head is on another page | jump, and the grid follows to that page |
| two clients, different current views | different heads, both correct, one press each |
| you answer the head | the hook clears `tl.attention`, it leaves the queue, the next press gets the new head |
| you never answer the head | it stays at the head; mute is the way out |
| `◄` with no origin | dark, press does nothing |
| `◄` after three `next`es | back to the non-view origin |
| tallyd restarts | `asking` is rebuilt from the journal with original timestamps (#7), so the queue survives; origins do not, and `◄` goes dark until you jump again |
| anchord unreachable | no session list, so no queue; `►` dark, logo dark red ([05](05-interaction.md#the-logo-is-the-health-light)) |

## Code shape

```rust
// src/queue.rs — pure, like src/chord.rs: no tmux, no socket.
pub fn head(
    asking:  &BTreeMap<Slot, u64>,
    pads:    &Pads,
    current: Option<Slot>,
) -> Option<Slot>
```

`Gesture` gains `Back` and `Next`. Neither is a chord: they commit nothing,
so there is nothing for a hold to protect.

`Jump` gains `back()`, already specified in
[15](15-views.md#the-rust-seam).

**One rename is forced.** `Button::Page(u8)` (`src/surface/addr.rs`)
becomes `Button::Top(u8)`: four of the eight no longer have anything to do
with pages, and `Page(5)` meaning *a dark unassigned button* is a name that
lies. The CC mapping is untouched — `Top(1)` is still `CC91`.

## Validation

**Unit-testable:**

- the head is the longest-waiting, not the most recent
- a muted slot is never the head
- the slot you are viewing is never the head, and two clients get two heads
- a view *is* a valid origin — the regression above, held by a test
- a pad and an address round-trip through the window at every offset
- scrolling crosses a page boundary with nothing special happening
- `►` scrolls the least it can to show where it landed
- ~~`⌂` is dark exactly at offset 0~~ — amended by [20](20-the-overview.md):
  `Session` is always lit, and `⌂` is hold-`▲`, which is dark at offset 0
  for the same reason the scroll is
- an empty queue is `None`, not an error
- `►` is dark exactly when `head` is `None` — one predicate, read by the
  renderer and by the press, the rule `src/act.rs` already holds for chords
- ties break by slot order, so the head is stable across ticks
- a `kind: "nav"` line is skipped by the restart readback
- a pre-`kind` line still parses

**Only the hardware says:**

- [ ] `►` amber on the top row reads as *there is work* and not as *this
      page wants you* — it is the same amber in a new place
- [ ] the round trip is actually ten seconds: `►`, answer, `◄`
- [ ] losing the current-page readout is not missed
- [ ] a week of `kind: "nav"` lines, read against the three questions in
      [§ The instrument](#the-instrument)
