# 22 — The corner button has two modes

Settled elsewhere and not reopened here: the grid is an 8×8 window onto a
64-row space ([16](16-the-queue.md)), `>` launches into its row
([18](18-the-launch-column.md)), and `CC19` — the corner the silkscreen
labels *Stop* — is the only hold modifier left. This document is about
what that one button carries.

## There are no spare buttons

[18](18-the-launch-column.md#what-is-deliberately-not-here) ended with a
rule: *"That is the first time this surface has had nothing spare, and the
bar for the next idea is now a button taken from something that works."*

Two things want a button and neither can have one. So `CC19` carries both,
and **a double-tap toggles which**. That is not a trick: on a Mini MK3 the
bottom-right button is silkscreened **Stop Solo Mute** because in Ableton
it cycles modes by repeated presses. Novation put a mode switch there and
printed its menu on it.

| mode | at rest | what the hold means |
|---|---|---|
| **Stop** | 5 red | end what is running, forget what has ended |
| **Move** | 37 turquoise | put this pad somewhere else |

Stop is where it rests, because it has the muscle memory and the
silkscreen.

## Stop does kill and purge

[18](18-the-launch-column.md#what-is-deliberately-not-here) rejected this —
*"it widens the blast radius of the single gesture [05] spends a section
protecting"* — and the rejection is overturned here deliberately, with the
cost named rather than argued away.

**The two consequences can never share a pad.** `Pad::alive` is working,
quiet or asking; `purgeable` is exited or lost (`model.rs:161`). Disjoint
sets, so every pad under the hold has exactly one possible outcome and
exactly one colour, and the preview already colours by consequence
([05](05-interaction.md#the-held-preview)).

| pad | preview | tap |
|---|---|---|
| working, quiet, asking | 5 red | terminate it |
| hiding ended rows | 7 dark red | forget **all** of them |
| nothing at all | off | nothing |

**The cost, stated.** Purge's targets include pads that render dark: a
clean exit keeps its `tl.slot` and its log but draws as nothing, so a dark
pad can have a dozen rows behind it (`model.rs:152`). Under the old kill
chord the dark two-thirds of the grid was a free miss. It is not any more.

What makes that affordable is that **the hold is what reveals them.** They
stop being invisible at the exact moment they become targets, which is the
one property the dark-pad objection needed. A thumb that lands one pad off
forgets a dead row's directory and log; a thumb that lands one pad off
under the old chord did nothing. That is a real regression, priced at one
order below killing a live agent, and it is the reason purge gets the
dark-red *consequence* colour rather than kill's red.

It also closes the hole `model.rs:156` has been describing in the present
tense since M10 — *"the only way to see them is to hold it"* — about a
chord that no longer exists. `Pad.purgeable` has been computed every frame
and read by nobody since.

### Purge takes the whole stack

A tap forgets **every** ended row behind that address, not the newest one.
That is what `tally purge <addr>` already does ([11](11-cli.md)) and what
`model::purgeable` already returns — a `Vec<String>` per slot, newest
first.

One tap, N deletions, and no way to delete one of them from the grid. The
grid does the common thing; `anchor` does the surgical one.

**It takes the row's launch button with it.** `>` inherits `cwd` and
`command` from the newest session in the row, dead ones included
([18](18-the-launch-column.md)). Purge the last ended row in an empty row
and that row's `>` goes dark, because there is nothing left that knows the
directory. This is the truth and is drawn as such; it is not an error.

## Move is the missing verb

Nothing on this surface rearranges it. A shell spawn lands where you
typed, `>` takes the leftmost free column, and after a day the board is a
**log of where things happened to land** rather than a layout you chose.

**A move is one label write.** A pad's address is `tl.slot` (`anchor.rs:30`)
and `label()` sets it atomically in one call (`anchor.rs:520`). No process
moves, no pane is touched *by the write* — the agent never learns it
happened. The view follows, from `tallyd` ([31](31-the-moved-view.md)).

### Hold, tap the source, tap the destination

Two taps inside one hold. No new grammar: `chord.rs` already emits one
`Chord` per tap and the pairing lives above it.

```text
hold Move          every pad holding a live session goes turquoise
tap a session      it starts flashing — it is in your hand
                   every pad it can land on comes up to a dim glow
                   the occupied pads stay turquoise, and do not move
tap a dim pad      it lands there
release early      nothing happened
```

The escape hatch survives both halves: let go before picking up, or let go
with something in hand, and the board is unchanged. That is chord rule 4
([05](05-interaction.md#the-exact-rules-of-a-chord)) doing the job it was
put there for.

| | |
|---|---|
| **what moves** | the drawn session, and only it |
| **what stays** | every ended row at the old address |
| **legal target** | a pad with no live session, on the visible grid |
| **across pages** | `▲▼` under a held modifier already works; the pad in hand survives the scroll |

**The history does not follow.** Move changes one session's address. The
ended rows behind the old pad stay where they are, so the old row still
knows its directory and keeps its `>`. Dragging a session out of a row is
not a claim that the row never existed.

**The destination must be free of live sessions.** It may have ended rows
behind it — a live session with dead history underneath is what every
reused pad already looks like.

## Changing the mode

**A double-tap on `CC19` toggles it.** Red ↔ turquoise, on the button, at
rest, all the time.

A *single* tap will not do, and the reason is worth writing down because it
was the first design and it was wrong: a bare tap on the corner is a press
and a release with no pad between them, which is **exactly the shape of an
aborted hold**. Chord rule 4 — *releasing a modifier without tapping is a
no-op* — is the escape hatch the entire hold-then-tap grammar rests on
(`chord.rs:10`). Cycling on it would mean you abort a kill and silently arm
Move. Two taps cannot be confused with changing your mind.

**The clock stays out of `chord.rs`.** Same rule, same reason, same place
as the hold-`▲` timer ([20](20-the-overview.md)): that module is pure —
presses in, gestures out, no `now` — and that purity is what lets the four
chord rules be tested without a device or a timeline. `tallyd`'s 16 ms loop
has `elapsed` in hand on the line that calls `feed`, so it owns both
thresholds and `chord.rs` never emits a mode change.

| | | why |
|---|---|---|
| a tap | press and release inside 250 ms, no chord fired | a deliberate tap is ~100 ms; a hold long enough to read a preview is not |
| a double-tap | two taps inside 400 ms | long enough to be comfortable, short enough that two unrelated presses do not pair |

**It latches, and that is allowed here.** The ROADMAP rejects latched modes
on the grounds that *"the grid means two things depending on something
invisible"*. Nothing is invisible: the state is the colour of the button
that set it, and **the grid does not change until you hold** — at which
point it repaints entirely in the mode's colour under your thumb, before
anything can fire. You cannot act in the wrong mode without first seeing
the wrong colour.

## The colours

| | | |
|---|---|---|
| 5 red | Stop at rest; a pad that will be terminated | the colour of what it is about to be |
| 7 dark red | a pad whose ended rows will be forgotten | already the documented *will be forgotten* colour ([18](18-the-launch-column.md)) |
| 37 turquoise | Move at rest; **a session is here** — a pad that can be picked up, or one that is in the way | unused since [16](16-the-queue.md) |
| 37 turquoise, flashing against dark | the pad in your hand | it is one of the occupied pads, and it is the one you are carrying — and the fastest figure the device has |
| 1 dark grey | **somewhere to put it** | the only dim level there is, and it already means *available, not urgent* |
| 3 white | either mode while held | already *this is live right now* |
| 1 dark grey | present, and this gesture will not touch it | unchanged |

### Picking something up flips the question, not the board

The first version coloured the second half by *consequence* alone: the
destinations turquoise, the occupied pads dim. It read wrong, and the
reason is that it repainted the board under the thumb that had just
acted. You tap a session and two thirds of what you were looking at
changes colour for reasons that have nothing to do with the pad you
touched.

So **turquoise is not the verb, it is the fact**: a session is here. It
means that before the pick-up and after it, and the occupied board is
the one thing that does not move when your hand closes. What changes is
the *question*. Hand empty, you are reading for sessions, and the
sessions are the lit thing. Hand full, you are reading for room — and
room comes up to a **dim glow** on pads that were dark a moment ago.

**Dim, not bright, and that is the second correction.** Room was drawn
white first, on the reasoning that the thing you are looking for should
be the bright thing. It is not, because of a count: with a hand full,
*most of the grid* is a legal destination. Lighting destinations brightly
lights two thirds of the surface, and a board where two thirds is bright
has told you nothing — the pad you are carrying stops standing out in the
one frame where it is the only thing that matters.

So the levels invert here, and only here. Everywhere else bright is the
target and dim is what will be spared. With a pad in hand, dim is the
target and the only bright things are the two that are **not** room.

The pad in your hand keeps the turquoise it already had and **flashes
against dark**. Hue says it is one of the occupied; motion says it is the
one you are carrying. Making it white instead would have made the one pad
that is not a destination the brightest thing on a board being read for
destinations.

**Flash rather than pulse, because it is the only way to be faster.**
Both figures run on the one clock ([17](17-the-tempo.md)) — the clock
amber's urgency is spoken in — so there is no rate knob here that does
not also drag amber along. There is only the choice of figure: a pulse
is a two-beat ramp, a flash is one beat. Twice the rate for free, spent
on the one pad allowed to be impatient.

It flashes against **dark**, not against a second colour. Dark is the one
value on this grid that cannot be misread in this moment: an occupied pad
is turquoise and a destination is a dim grey that never goes fully out, so
the gap says *nothing*, which is exactly what a pad that is no longer
anywhere should say half the time.

**This is the only exemption to both *only amber moves* and *nothing
blinks*** ([05](05-interaction.md#nothing-blinks-and-only-attention-moves)),
and it is cheap, because both of those were measured on a surface at
**rest**. A pad in your hand is not at rest. It is one pad, for as long
as your thumb is down, on a grid a held modifier has already repainted —
there is no amber on it to compete with, and no time for a 50% chop to
become the thing that wears you out by afternoon.

`render.rs` pins both bounds as tests rather than comments: *at most one
pad in any preview moves, and it is the one in your hand*, and *the pad
in your hand is the fastest thing on the grid* — the second of which
also asserts amber was not dragged along with it.

Turquoise failed once, as a page button lit beside a white current page —
two bright colours on one row with no way to tell which was which
([05](05-interaction.md)). One button is not a row, and the failure mode
does not reach it.

**The disjointness invariant is now exempt twice.** `render.rs` asserts
that no control wears a slot colour; red was exempted when the kill button
took it, and dark red joins it here. Both are the right column, not the
grid, and position disambiguates before hue has to. Every other control is
still checked.

## The map does not learn about forgotten rows

A seam worth stating, because it was found and closed rather than
missed. The overview ([20](20-the-overview.md)) summarises what
`resolve` draws, and `resolve` drops clean exits — so a dark-but-purgeable
pad contributes **nothing** to the map. Now that Stop forgets dead
stacks, the map understates exactly where those stacks are.

Left alone, deliberately, for two reasons:

- **You can never see both.** `Session` and `CC19` held together is two
  modifiers, and two modifiers disarm the surface
  (`chord.rs:modifier`). There is no frame in which you are reading the
  map and previewing a purge at once.
- **A dead row is not a destination.** The overview exists to put the
  window somewhere; a row whose only content is forgettable history is
  not somewhere you want the window. Lighting it would be the map
  answering a question it was not asked.

The dark-pad problem is solved by holding Stop on the grid, which is
exactly what it now does.

## What is deliberately not here

**Solo**, in the sense of *silence the other thirty-one*. It was the third
mode until Move displaced it. Solo is invisible global state that needs
self-clearing rules and a filter threaded through the queue, the tempo and
the logo — all to answer *"these are the ones I care about right now."*
Move answers that physically, by putting them on row 1, with nothing to
remember and nothing to clear. **Arrangement beats state.** If a week of
Move shows people arranging and still being dragged away, solo comes back
with evidence.

**Peek** — *what is this pad?* A real hole: the surface cannot name a pad,
and some share of 341 daily jumps are identification rather than work. It
needs a sink to print into and it is the only candidate here that needs new
machinery. Deferred, not dead. Purge's preview partly answers it for free:
holding Stop shows every pad hiding history.

**Mute**, which keeps the shell ([18](18-the-launch-column.md)). It is the
starvation hatch for `►` and the tempo and it scored zero presses in a day;
giving it a third of the one button that works is not supported by
anything.

**Respawn.** Rejected in [18](18-the-launch-column.md) and not reopened: a
button that guesses which conversation you meant is a button that is wrong
at the worst moment.

**Swap, when the destination is occupied.** Two label writes are two
anchord calls, and a failure between them leaves two sessions claiming one
address. An occupied pad stays turquoise and does nothing instead — it
is *there*, which is exactly why you cannot land on it. If compaction
turns out to need it, it needs an atomic two-session op in anchord first.

**A third mode.** Two fit on a toggle. A third needs a cycle, a cycle needs
you to remember the order, and the next idea after that is a menu.

## Edge cases

| situation | behaviour |
|---|---|
| Stop on a pad that is alive **and** has ended rows behind it | terminates. Alive wins; the rows are still there afterwards |
| Stop on a `lost` session | purges it — `lost` is purgeable (`model.rs:168`) |
| Stop on a clean-exited pad that draws dark | dark red under the hold, and a tap forgets it |
| purge fails for one id in the stack | reported and dropped; the next reconcile redraws the truth ([`act`](../src/act.rs)) |
| Move with nothing live on the grid | nothing lights; the hold does nothing |
| Move, pick up, then page away and drop | lands on the new page's pad — the address is absolute |
| Move, pick up, and the session exits before the drop | the drop fails, is reported, and the pad redraws dead |
| Move onto the pad it came from | nothing; same address, no write |
| double-tap while a chord is mid-hold | ignored; a tap requires no chord to have fired |
| the mode flips while the button is still down | it always does — the second tap completes on the **release**, so the press went down under one verb and comes up under another. The release is matched by **button**, not by verb, or the old one is stranded in `held` and the grid freezes in a preview with no way out |
| mode toggled while anchord is unreachable | allowed — the mode is a surface state, not a session state |
| two modifiers down | nothing fires, unchanged (`chord.rs:modifier`) |

## Validation

**Unit-testable** — all of these now exist and pass:

- kill and purge targets are disjoint for every `Pad`
- `effect` returns the purge consequence for a pad that draws dark but has
  rows, and the kill consequence for a live one
- a purge tap passes **every** id `model::purgeable` holds for that slot
- the preview and the press read the same `effect`, still — one function,
  two readers ([`model`](../src/model.rs))
- a move writes exactly one `tl.slot` and changes nothing else
- picking up and releasing without dropping writes nothing
- a drop onto an occupied pad writes nothing
- the mode toggles on two taps and not on one, and not on an aborted hold
  — the sharpest one, because a single-tap toggle would have broken
  chord rule 4 silently
- a long press clears a half-started double, so an abort cannot complete
  a pair begun a minute earlier
- `chord.rs` still emits no mode change and still takes no clock: no
  sequence of presses alone moves the mode
- the mode survives [`Chords::clear`] — a replug forgets the hold and
  keeps the verb, because the mode is not a button that can stick down
- exactly one thing in a move animates, and it is the pad in your hand.
  After `9b2eee5` the only moving thing on the *resting* surface is
  amber, so a second breathing pad would be claiming something is
  asking — which is why the hand flashes instead of breathing, and why
  nothing else under the hold moves at all

**Only the hardware says:**

- [x] red and dark red are tellable apart under a thumb, on a grid, at
      arm's length — **the one thing this design rests on, and it holds.**
      Confirmed on a live grid. Had it failed, the honest fix was
      splitting kill and purge back onto separate gestures, and the
      merge in this document would have been a mistake
- [x] the dark pads lighting up under Stop reads as *there is history
      here* and not as *I broke something*
- [x] a double-tap is reachable without looking, and 250/400 ms are the
      right numbers — untouched since, so the guesses were right
- [ ] a week of logs: does the mode ever sit on Move, and does anything
      get purged from the grid that never got purged from the shell

### What the first press found

The mode toggle froze the surface, immediately. A double-tap completes on
the **release**, and `tallyd` flips the mode before `chord.rs` sees that
release — so the button went down under one verb and came up under
another, and a release matched by *verb* stranded the old one in `held`
forever. `modifier()` then answered `Some` with nothing under the thumb:
the whole grid sat in a held preview, in red or turquoise, and no
sequence of presses recovered it.

It accumulated, alternating which verb got stuck, so only a restart
cleared it. Fixed by matching a release on the **button** rather than the
verb, which makes the pairing true by construction instead of relying on
two pieces of state staying in step.

Worth recording because no test would have found it: every unit test fed
presses through `Chords` alone, where the mode never moves. The toggle
lives in `tallyd`, the stranding happened in `chord.rs`, and the bug was
in neither — it was in the order the two ran.
