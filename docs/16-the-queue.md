# 16 — The top row is a queue, not a map

A counter-proposal to [12](12-layout-proposal.md) and
[13](13-layout-review.md), tracked as
[#8](https://github.com/alehatsman/tally/issues/8).

Both of those documents answer the question *where is the blocked agent*.
The first hour of real data says that is not the question.

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
not.** `13` §10 asks whether you need to know which project is blocked
before you jump, and guesses the honest answer will be *"I press the amber
one and go"*. It is not. There is usually no *the* amber one.

And the choice is not close. When two or more pads are amber, the gap
between the longest wait and the shortest is **median 386 s, max 1382 s**.
Ordering is not a tiebreak between similar things; it is the entire
decision.

⚠️ **One hour, and an unrepresentative one** — five agents running while
one of them was being worked with. A quiet afternoon inverts the table.
This does not replace the week `13` §10 asks for; it re-aims it, and
[§7](#the-instrument) makes the week measure the right thing.

## Why this kills row = project

`12` and `13` are right that triage is the bottleneck. Rows are the wrong
instrument for it, for four reasons, in increasing order of how hard they
are to repair.

**It answers "where" when the question is "which first".** With five pads
amber, knowing that row 4 is the work repo does not rank them. Grouping is
a *locating* aid, and at five lit pads out of sixty-four there is no
locating problem. There is an ordering problem, 61% of the time, and a row
orders nothing.

**`13` already rejected it, in its own words, two sections later.**
*Searched and rejected* kills row = urgency band with *"the pads move while
you reach for them. You cannot build muscle memory against a target that
relocates, and a mis-press is a kill."* §4.1 then decides rows are
**derived, not declared**, which relocates pads on a slower clock. The
consequence was filed under *Still open* — *"derived row order when nothing
is pinned … whether that is good enough"* — rather than recognised as the
rejection the same document had already made.

**The grouping already exists, by hand, and costs nothing.** The five slots
that asked for attention this hour were `1:1:1`, `1:2:1`, `1:3:1`, `1:8:1`,
`1:8:2` — one project per row, lane 1 durable, lane 2 the overflow. That is
`13`'s layout, running today, as a convention over declared addresses. It
is stable because you chose it. What the proposal adds on top is taking the
addresses away from you and letting the daemon assign them, which is the
one part [03](03-model.md#a-slot-is-not-a-session) says you cannot have:
*a grid you cannot learn is a grid you look at instead of feel.*

**It invents a capacity cliff.** Eight rows of seven lanes, for a workload
of five live sessions across four projects, two at most in any one. §4.3
has to add signalling *because a row can fill* — a limit flat addressing
does not have. `12` opens by diagnosing "capacity for a workload of about
twenty" as the error it is correcting, and this is that error rotated
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

The silkscreen stops being decoration. `13`'s own scorecard calls `◄ ►`
**"thin — the weakest pair here"**, mapping them to *leave focus* and
*commit the safe default*. Under a queue they become the most literal
buttons on the device.

```text
  ▲      ▼      ◄      ►   │  ·    ·    ·    ·  │  ◈
 page   page   back   next │      unassigned    │ health
  up    down               │
```

| button | dark when | lit | press |
|---|---|---|---|
| `▲` `▼` | no page that way holds anything | 1 dark grey, static | change page |
| `◄` back | no origin recorded | 3 white, static | return to where you were standing |
| `►` next | the queue is empty | **9 amber, pulsing** | jump to the longest-waiting |
| 4 others | always | — | nothing |

Every button here is **self-describing**: dark does nothing, lit does
exactly one thing, and there is no state to remember. `next` reuses the
reserved amber and means what amber means everywhere else on the surface —
*look at me* — which is why it does not need a fifth colour.

**`next` crosses pages**, and switches the visible page to wherever it
landed so the grid stays coherent with where you are. That is what makes
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

### What this cuts from `13`

**§5's `►` = commit the safe default.** Approve-without-reading loses its
button. `13` argued itself into narrowing that gesture to the one answer
safe to give without reading, having opened by calling the idea *"the most
dangerously framed"* — and the narrowing is still a button that answers an
amber light rather than a command. Spending `►` on navigation retires it
without a further argument.

**Focus mode goes with it.** `▲▼` are pages, `◄` is back, `►` is next; none
of them is conditional on a mode, so there is no mode. `13`'s *"one mode,
entered by a deliberate press, indicated by a dimmed row and four white
arrows"* has nothing left to enter.

**A tap on a pad keeps its one meaning**: jump. `13` §5 gave a second tap a
second job, which [15](15-views.md#the-return-trip) then tried to give a
third. Both are withdrawn — `◄` is the return gesture, on its own button,
visible, with an affordance.

## Back, and where you were

One rule, shared with [15](15-views.md#the-return-trip) and unchanged by
this document:

```text
Origin: client name -> pane id
recorded  when a client is about to be switched
          AND its current pane is not a view pane
one deep, never overwritten by a view -> view move
```

So `next → next → next → back` lands you in nvim, not three agents deep.
**The queue is a trip; home is one press.** Chained jumps were `13`'s and
#3's hardest open question and the queue answers it by construction: there
is no reason to walk back through agents when the only place you want is
the one you started from.

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

A week of this answers `13` §10 with data rather than opinion:

- **how many jumps came from `next` versus from a pad.** If you keep
  hand-picking, the queue is wrong and grouping deserves its second look.
- **how deep the queue was when you pressed.** If `queued` is almost always
  1, the 61% was an artefact of one unusual hour and `12`'s whole premise
  is weaker than this document assumes.
- **how long the thing you chose had been waiting.** If you consistently
  answer something younger than the head, the ordering is wrong — recency
  or project may beat age, and that is the version of §10's experiment
  worth running.

This replaces the row-4 perceptual test as the gate, because it measures
the decision rather than a proxy for it. The row-4 test only matters if
rows are the answer, and the first thing to find out is whether they are
the question.

## What this unblocks

[15](15-views.md#what-is-cleared-to-build) held two of M7's three steps
behind #8. This clears both:

| held on | because | now |
|---|---|---|
| step 3, the return trip | `13` §5 had already spent the second tap | `◄` is the gesture; no tap is overloaded |
| step 2, adoption | `page:row:col` might be replaced one-way | addressing is unchanged and declared |

Neither was blocked on rows being a good idea. Both were blocked on not
knowing, which is the cost of 776 lines with no decision attached.

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

**One rename is forced.** `Button::Page(u8)` (`src/surface/addr.rs:22`)
becomes `Button::Top(u8)`: four of the eight no longer have anything to do
with pages, and `Page(5)` meaning *a dark unassigned button* is a name that
lies. The CC mapping is untouched — `Top(1)` is still `CC91`.

## Validation

**Unit-testable:**

- the head is the longest-waiting, not the most recent
- a muted slot is never the head
- the slot you are viewing is never the head, and two clients get two heads
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
