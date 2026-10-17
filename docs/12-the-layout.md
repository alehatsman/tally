# 12 — The layout

What each of the eighty-one buttons does, why the grid is flat, and which
arrangements are closed.

This is the current surface. The address model is
[03](03-model.md#the-address-pinned), the colour language is
[05](05-interaction.md#the-colour-language), and the queue that drives the
top row is [16](16-the-queue.md).

## The surface, and what the legends mean

Checked against Novation's user guide, *Hardware Overview*, p.8 — what is
physically printed on the device, and what tally does with it:

```text
 ▲    ▼    ◄    ►   │ Session │  Drums   Keys   User │ ◈
                    │         │  └──── "Custom" ────┘ │
┌──────────────────────────────────────────────────┐  ┌──────┐
│                                                  │  │  >   │  ×7
│                     8 × 8                        │  │  >   │
│                                                  │  │  >   │
└──────────────────────────────────────────────────┘  ├──────┤
                                                      │ Stop │
                                                      │ Solo │
                                                      │ Mute │
```

Top row is `CC91`–`CC98`, logo `CC99`, right column `CC89` down to `CC19`
([02](02-surface.md#addressing)).

| printed | what it means to the device | what it means here |
|---|---|---|
| ▲ ▼ | move the session outline across **scenes** (rows) | scroll the window one row |
| ◄ ► | move it across **tracks** (columns) | back where you were standing / go to the longest wait |
| Session | return to the overview | **held**: the grid becomes the whole 64-row map |
| Drums | one-shot sample pads | the arts: 64 pads doing something that is not work |
| Keys, User | custom modes | nothing. Two buttons, deliberately unspent |
| `>` | **launch a scene** — the whole row fires | spawn into that row, inheriting its directory |
| Stop / Solo / Mute | **cycles** the bottom row's verb | the corner's two verbs, swapped by a double-tap |
| ◈ logo | brand | the health light |

**Every legend is doing something close to what it says**, and that is not
decoration — it is the cheapest form of learnability this device offers. A
button whose printed word and whose behaviour agree is a button you do not
have to remember. The two exceptions are honest ones: `Keys` and `User` do
nothing at all, because an inert button that lights up like a live one is
an invitation to press it.

## Every position, as it stands

| position | resting | tap | held |
|---|---|---|---|
| grid pad, nothing there | off | spawn its spec, if `slots.toml` names one | — |
| grid pad, bound | green / amber / red / dark red | **jump** ([06](06-jump.md)) | — |
| `Top(1)` ▲ | 1 dark grey if there is a row above | scroll up one | past 500 ms: **home**, back to row 1 |
| `Top(2)` ▼ | 1 dark grey if there is a row below | scroll down one | — |
| `Top(3)` ◄ | 3 white if an origin is recorded | **back** | — |
| `Top(4)` ► | 9 amber pulsing if anything waits | **next** — the longest wait | — |
| `Top(5)` Session | 1 dark grey, always | — | **the map**: 64 rows, tap to go ([20](20-the-overview.md)) |
| `Top(6)` Drums | 1 dark grey; 3 white while playing | **arts** ([21](21-the-arts.md)) | — |
| `Top(7)` Keys | off | nothing | nothing |
| `Top(8)` User | off | nothing | nothing |
| `Action(1..7)` `>` | 1 dark grey if the row can inherit | **launch** into that row ([18](18-the-launch-column.md)) | — |
| `Action(8)` `CC19` | 5 red in Stop, 37 turquoise in Move | double-tap **swaps the verb** | the modifier ([22](22-the-mode-button.md)) |
| `CC99` logo | the health light ([05](05-interaction.md#the-logo-is-the-health-light)) | — | — |

**Dark does nothing, lit does exactly one thing, and there is no state to
remember.** That is the rule the whole top row runs on, and the one
exception proves it: `Session` and `Drums` are always lit, because they
always have something to show.

## The grid is flat, and that is the decision

Sixty-four rows of eight. The grid is an **eight-row window** onto them,
slid one row at a time by `▲▼`. An address is `page:row:col`, you write it,
and nothing ever moves it ([03](03-model.md#the-address-pinned)).

Pages are still how an address is *spelled* — eight pages of eight rows
stack into the sixty-four — and they are not a place you go. A page
boundary is an arithmetic detail: page 1 row 8 and page 2 row 1 are
adjacent, and scrolling past them does nothing special, because nothing
special is there.

Three things decided this, and all three are measurements rather than
tastes:

**There is usually no *the* amber pad.** Time-weighted over an hour of real
use: 61% of the time more than one slot was asking, 15% of the time exactly
one was. And the choice was not close — when two or more are amber, the gap
between the longest and shortest wait is a median of 386 s. **The
bottleneck is ordering, not locating**, and no arrangement of the grid
orders anything. That is `►`'s job ([16](16-the-queue.md)) and the
breathing rate's ([17](17-the-tempo.md)).

**At the observed density there is no locating problem either.** Seven rows
of sixty-four have ever held a session, five slots lit on a busy hour.
Finding a lit pad among sixty-four mostly-dark ones is not work.

**A grid you cannot learn is a grid you look at instead of feel.** Any
scheme where the daemon assigns addresses — by project, by urgency, by
recency — takes the addresses away from you, and a slot is a *durable
place* precisely so your muscle memory survives the sessions in it
([03](03-model.md#a-slot-is-not-a-session)).

Grouping still exists; you just do it yourself. The slots that asked for
attention in that hour were `1:1:1`, `1:2:1`, `1:3:1`, `1:8:1`, `1:8:2` —
one project per row, lane 1 durable, lane 2 the overflow. That is a
convention over declared addresses, it costs nothing, and it is stable
because you chose it.

## Closed, and why they stay closed

The design space here is small enough to state. Each of these is a real
idea with a real reason it is not the layout.

**Rows are projects, columns are lanes.** The strongest of them, and it
fails on the measurement above: it answers *where* when the question is
*which first*. It also invents a capacity cliff — eight rows of seven lanes
for a workload of five live sessions — which flat addressing does not have,
and which then needs signalling to tell you a row is full. A sixty-fourth
of the address space per project, assigned by a daemon, to solve a locating
problem nobody has.

**Row = urgency band** — top row wants you, middle is working, bottom is
dead. Self-sorting, always one row to look at, and **the pads move while
you reach for them.** You cannot build muscle memory against a target that
relocates, and a mis-press is a kill. This is the load-bearing rejection:
it also closes the compacted map ([20](20-the-overview.md#rejected)) and
anything else that re-sorts the grid under your hand.

**Column = project, row = lane.** Symmetric and worse. The right column is
adjacent to *rows*, so `>` stops meaning anything.

**Answering the agent from the grid.** Mechanically possible — `tmux
send-keys` reaches the pane, which is the path a keyboard takes, so
`tallyd` would take no lease and anchor would need no new call
([25](25-the-squad.md) measured it). It is still refused, and not on the
mechanism: **it is a button that answers an amber light rather than a
command.** You cannot read what you are agreeing to from a pad, and
narrowing it to "the one answer safe to give without reading" is the same
button with a smaller blast radius. `►` spends that press on navigation
instead, and navigation commits nothing.

**A cycling verb key with four modes.** Launch / Stop / Solo / Mute on one
button, Ableton-style. Four mode axes where the surface wants none, each
indicated by one small key's hue. The corner *does* carry two verbs now
([22](22-the-mode-button.md)), and the difference is the whole argument:
two fit on a toggle, the grid does not change until you hold, and at that
moment it repaints entirely in the mode's colour before anything can fire.
Three needs a cycle, a cycle needs you to remember the order, and the next
idea after that is a menu.

**Solo**, in the sense of *silence the other thirty-one*. Invisible global
state that needs self-clearing rules and a filter threaded through the
queue, the tempo and the logo — all to answer *"these are the ones I care
about right now"*. Move answers that physically, by putting them on row 1.
**Arrangement beats state.**

## What the device has and nothing spends

Two documented capabilities, still unused:

- **Global brightness** ([02](02-surface.md#other-messages-worth-knowing),
  `08h`). One message, no model change. Dim the whole surface while nothing
  wants you, full brightness the moment something does. Orthogonal to the
  colour language — but the colour language is built on *bright means where
  you are*, so it is the wrong axis to lean on and a separate, smaller idea
  if it is ever specced.
- **LED sleep** (`09h`). The same thought overnight.

The third one on this list was **MIDI clock**, and it was spent: the
breathing rate is the age of the oldest thing waiting
([17](17-the-tempo.md)).

## Still open

**[#8](https://github.com/alehatsman/tally/issues/8)** — the grouping
question, held against a week of `kind:"nav"` lines. The number that
decides it is `queued`: if it is almost always 1, then the 61% above was
one unusual hour, there *is* usually a single amber pad, and grouping
deserves its second look. Every `next` press so far has reported
`queued: 1`.

**`Keys` and `User` are the last two free buttons**, and the bar for
spending one is a button taken from something that works
([18](18-the-launch-column.md)). [25](25-the-squad.md) proposes `User`.
