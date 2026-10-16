# 17 — The tempo says how long

The grid says *something wants you*. It has never said *and it has been
waiting eleven minutes*, and that is the one thing you need in order to
decide whether to get up.

[16](16-the-queue.md) answers it for the press — `►` goes to the oldest.
This answers it for the glance.

## The question, with numbers

Forty-nine answered waits, from `attention.jsonl`:

```text
p10      7s     you were already looking
p25     50s
p50    187s
p75    335s
p90  1,076s     eighteen minutes
max  2,403s     forty minutes
```

**Two orders of magnitude, and the pad looks identical at both ends.** A
seven-second amber and a forty-minute amber are the same colour, the same
brightness, the same rhythm. The surface has one amber and it is spent;
there is no second colour to give this, and
[05](05-interaction.md#why-it-is-one-sentence) is four paragraphs on why
adding one fails.

## What the device already gives away

✅ From [02](02-surface.md#colour), verified on hardware:

> Flashing is a 50% duty cycle at one beat; pulsing is a two-beat ramp.
> Both run on the device's own clock (**120 bpm when no MIDI clock is
> supplied**).

That parenthesis is the whole feature. Supply a clock and the host owns
the beat — the ramp still runs on the device, still with no repaint loop
and no drift *within* a beat, but how long a beat lasts becomes ours.

So there is a free signal dimension sitting unused, orthogonal to colour,
costing no new concept and no agreement about what a pad means.

## The mapping

**Tempo is the age of the oldest thing waiting.** Not the count, not the
newest, not the average — the same quantity `►` sorts by, so the button
and the breathing cannot disagree.

```text
oldest wait   0s ────────────── 900s ──────────▶
       bpm   60 ────────────── 140 ───────────── 140 (clamped)
      cycle 2.0s ───────────── 0.86s
```

| | |
|---|---|
| nothing waiting | **60 bpm** — calmer than the device's own default |
| something just asked | 60 bpm — amber already says *look*; it is not old yet |
| p50, ~3 min | ~77 bpm |
| p75, ~6 min | ~90 bpm |
| 15 min and beyond | **140 bpm**, and no faster |

Linear in age, clamped at both ends. 900 s because the observed spread
puts p75 at 335 s and p90 at 1,076 s — the knee of the real distribution,
not a round number chosen first.

**The range is deliberately narrow and centred near today.** The device's
unsynced 120 bpm sits inside it. The slow end is *slower* than the grid has
ever been and the fast end is barely faster, because the failure this
design has already paid for once is a surface that is correct and
**stressful**: the hard amber flash was confirmed on hardware as reading
"urgent" and then confirmed in use as exhausting
([05](05-interaction.md#nothing-blinks)). A tempo that sprints is that
mistake with a different knob.

### It is global, and that is the right scope

One clock drives every animation — the amber *and* the green working
pulse. There is no per-pad tempo and there must not be one.

That is not a limitation to work around; it is what makes this readable
from across the room. The surface acquires one aggregate mood: **calm
while nothing is ageing, quickening as the oldest thing ages.** You read it
without looking at any particular pad, which is the only kind of reading
that happens at six metres.

A consequence worth naming: a lone eleven-minute amber speeds up the green
pulse of four agents that are working fine. That is correct. The room is
not calm — something has been ignored for eleven minutes — and the surface
should not pretend otherwise just because the thing being ignored occupies
one pad.

## The cost: a clock is a loop

**This trades "set it once and forget" for one byte, forever.** MIDI clock
is `0xF8` at **24 pulses per quarter note**, so the host emits it
continuously: 24 Hz at 60 bpm, 56 Hz at 140. `tallyd` grows a timer it did
not have.

Stated plainly because [02](02-surface.md#colour) sells the palette
animations precisely on *no timer, no repaint loop, no drift*, and this
gives one third of that back. What is kept is the important third: the
**ramp** still runs on the device, so a pulse is still smooth under load
and still does not stutter. What is given up is only the beat boundary.

The arithmetic is also not close. A repaint is 64 pads of sysex; this is
one byte at tens of hertz. The reconcile loop already runs at 0.5 Hz and
the device poll faster than that.

### Frozen breath is a feature

If `tallyd` wedges, the clock stops, and the device **stops breathing
mid-ramp** rather than carrying on at its own 120.

That is a liveness indicator the grid has never had. Today a wedged
`tallyd` leaves a perfectly normal-looking surface showing a frozen
snapshot — the worst failure a status display can have, because it is
indistinguishable from "nothing changed". A grid that has stopped breathing
is visibly wrong from across the room.

**It must not be mistaken for calm**, which is why the floor is 60 bpm and
not 30: a very slow breath and a stopped one have to be tellable apart, and
2 s per cycle is slow enough to read as calm while still obviously moving.

## Interfaces

```text
clock tick   F8                     24 per quarter note, on the open port
clock start  FA                     once, when tempo is first established
clock stop   FC                     on shutdown, so the device returns to its own 120
```

Nothing sysex, nothing in the frame, nothing in [`Frame`](../src/surface/frame.rs).
The tempo is not part of a picture — it is the rate the picture is drawn
at, and keeping it out of `Frame` keeps the renderer a pure function of
state.

```rust
// src/tempo.rs — pure, like src/queue.rs
/// Beats per minute for the oldest thing waiting.
pub fn bpm(oldest_wait_ms: Option<i64>) -> u16;

/// How long between `0xF8` bytes at this tempo.
pub fn tick(bpm: u16) -> Duration;
```

`Device` gains one method that writes a single byte. The oldest wait comes
from `Journal::waiting()` — the same map [16](16-the-queue.md) reads, so
again no new state.

## ✅ The thing that had to be tried first

**Does the Launchpad Mini MK3 sync its LED animations to MIDI clock on the
port `tally` opens?** The manual implies it, does not say it, and `tally`
opens `LPMiniMK3 MIDI` ([`src/device.rs`](../src/device.rs)) rather than
the DAW port.

**Yes.** Measured with [`examples/clock.rs`](../examples/clock.rs) against
a live grid — five phases, a silent baseline either side so "it changed"
was a comparison and not a memory:

```text
1/5   15s   no clock          the device's own 120
2/5   20s   clock  60 bpm     breathed slower
3/5   20s   clock 150 bpm     breathed faster
4/5   20s   clock  60 bpm     slower again
5/5   15s   STOP              back to 120
```

Reported from the desk: *"it breathes slower and faster, it works."*

Three things that fell out of it and did not need a second connection: the
probe opened its own output while `tallyd` held the port, CoreMIDI allowed
both, and the pads it was read against were the ones `tallyd` was already
pulsing — so the measurement was taken on the real surface in its real
state, not on a painted one.

This was the fourth time the project has needed this paragraph, after the
three greens, the hard amber, and the title scroll
([09](09-testing.md#what-a-green-suite-does-not-prove)). It is the first
time the hardware said **yes**.

## Edge cases

| situation | behaviour |
|---|---|
| nothing waiting | 60 bpm, the calm floor |
| anchord unreachable | 60 bpm — the logo says what is wrong; the tempo must not also panic |
| the oldest is muted | not counted; mute means *stop raising this*, and tempo is raising it |
| a slot stops asking | tempo recomputed next tick, from the new oldest |
| everything answered | back to 60 bpm, gradually is unnecessary — a step is honest |
| device unplugged | nothing to send to; resume on reconnect |
| `tallyd` exits cleanly | `FC`, so the device returns to its own 120 rather than holding a stale beat |
| `tallyd` is killed | no `FC`; the device stops breathing — see above, this is the point |
| tempo unchanged between ticks | keep sending; the clock is a stream, not an event |

## Validation

**Unit-testable:**

- `bpm(None)` is the floor, and the floor is above a rate that reads as stopped
- `bpm` is monotonic in age and clamped at both ends
- the 900 s knee and the 60/140 bounds are asserted, so changing them is a
  decision rather than a drift
- `tick` is 24 PPQN — one quarter note at 60 bpm is exactly 24 ticks in 1 s
- a muted slot does not set the tempo
- the oldest wait comes from the same map `queue::head` reads, and a test
  that they agree about which slot is oldest

**Only the hardware says:**

- [x] **the device honours MIDI clock on the port we already open** —
      confirmed, see above
- [ ] 60 bpm reads as calm and not as broken
- [ ] 140 bpm reads as *something is getting old* and not as *alarm*
- [ ] a day beside it without the quickening becoming tiring — the
      measurement the hard amber flash failed, which only hours can take
- [ ] a wedged `tallyd` is visibly frozen rather than quietly stale

## Rejected

**Per-pad tempo.** The clock is global; there is no such thing. Even if
there were, eight pads breathing at eight rates is the sixty-four-moving-pads
problem that `lost` is static to avoid.

**Tempo for the count instead of the age.** Five fresh ambers are not more
urgent than one forgotten one, and the count is already readable — they are
all lit.

**Flashing instead of pulsing as things age.** That is the hard amber
flash, which was built, confirmed on hardware, and abandoned after a few
hours beside it.

**Brightness (`08h`) instead of tempo.** Also unspent, also orthogonal, and
the wrong axis: brightness competes with the colour language, which is
built on *bright means where you are*. Dimming the whole surface while
nothing wants you is a separate and smaller idea, and it should be specced
separately if at all.
