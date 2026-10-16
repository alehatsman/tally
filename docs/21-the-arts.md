# 20 — The arts

Settled elsewhere and not reopened here: the grid is an 8×8 window onto a
64-row space ([16](16-the-queue.md)), the top row is `▲▼◄►⌂` with three
buttons carrying nothing, and the colour language is one sentence —
*green is alive, amber wants you* ([05](05-interaction.md)).

This document is about the `Drums` button, and about the sixty-four pads
doing something that is not work.

## Why a serious surface gets a toy

Two reasons, and the second is the real one.

**It is a demo.** The pitch is "the agents you are running are a physical
object on your desk", and that pitch is made by a photograph. A grid of
eleven lit pads is a true picture of a working day and a bad picture of
what the hardware can do. Sixty-four pads running a plasma field is the
opposite, and it costs the product nothing because it is behind a button
that does nothing today.

**It is a pixel test nobody has to remember to run.** Every piece here is
a *total* function of the surface — all 64 pads, every frame. A dead pad,
a mis-flipped row, a diff that stopped converging, a palette entry that is
not the colour the manual says: all of them are visible inside two seconds
of plasma, and none of them is visible on a grid where eleven pads are lit
and the other fifty-three are correctly dark. `examples/paint.rs` tests the
flip once, by hand. This tests it every time somebody shows off.

## The button

`Drums` — `Top(6)`, which has carried nothing since [16](16-the-queue.md)
made the row a queue. The rule the top row runs on is *dark does nothing,
lit does exactly one thing*, and this keeps it: dark while no art is
playing, white while one is.

One button, three pieces, and a press moves to the next:

```text
dark → Plasma → Life → Uprising → dark → …
```

No menu, no mode indicator beyond the button itself, and no way to get
lost: keep pressing and you arrive back where you started.

## Leaving

**Every other button exits.** A press on the grid, an action, an arrow —
whatever it was, the art stops, the grid comes back, and the press itself
is swallowed. That is deliberate. You left art mode by *reaching for the
thing you actually wanted*, and the pad under your finger a quarter second
ago was a sine wave, not a session. Acting on it would be acting on a grid
the eye never saw, which is the same rule [16](16-the-queue.md) applies to
a stale queue.

**`►` is the one exception: it exits and jumps.** It is the button that
means *take me to the thing that is waiting*, and making somebody press it
twice to get there would be the art mode charging rent on the one gesture
this product exists to provide.

## What the art is not allowed to hide

The amber. It is the signal the whole thing is a bet on, and a toy that
can swallow it is a defect, not a feature.

So while art is playing:

- **The logo tells the truth.** `render::logo` already has the vocabulary
  for this — an unreachable anchord is dark red, attention you cannot see
  pulses amber — and while the grid is a plasma field, *all* attention is
  attention you cannot see. The logo carries it, unchanged, at full
  brightness.
- **`►` stays lit** when something is queued, and still goes there.

Everything else on the surface goes dark. The art owns the 64 pads; it
does not own the three lights that report.

## The three pieces

### Plasma

Four sines — one across, one down, one diagonal, one radial — summed per
pad and mapped onto the Mini MK3's bright-hue column. Nothing is
simulated, there is no state, and frame *n* depends on nothing but *n*,
which makes it the one piece that cannot drift, stall, or need reseeding.

Integer arithmetic against a 64-entry sine table, not floats. Not
austerity: it makes the piece a pure function the tests can assert exact
frames from, on any machine.

### Life

Conway on an 8×8 torus, which is the smallest board the rules stay
interesting on. Two things turn the classic into something worth watching
across a room:

- **Heat, not occupancy.** A cell that is alive is at full heat; a cell
  that dies cools through the ramp over the next several frames instead of
  going dark. What you see is the glider's wake, not the glider.
- **It reseeds itself.** An 8×8 torus falls into a still life or a
  two-cycle within a few hundred generations, and a frozen grid is
  indistinguishable from a crashed daemon — the single most expensive
  failure mode this product has. So the board keeps its last four states
  and reseeds the moment it repeats one.

### Uprising

The joke, and the only piece that is *about* tally.

```text
one green pad          an agent starts
then eight, then forty they are all working
one turns amber        it has a question
then all of them       they all have questions
one turns red          it gave up
then all of them       they all gave up
                       (a beat of dark)
a yellow smile         🙂
```

It is the colour language telling a story about itself, in the order the
language was designed to be read, ending on the one palette entry the
product never uses: **yellow**, which has sat in
`surface::palette::YELLOW` marked *currently unused* since M0. It is the
face, and it is the only place it goes.

Then it winks, holds, and starts over.

## Where the code lives

`src/art.rs`, and it is pure: `Piece` and a frame counter in, a `Frame`
out. It does not know what a session is, that anchord exists, or that the
pads it is painting usually mean something. That is the same boundary
`src/render.rs` keeps, and it is what lets the whole module be tested
without a device, a daemon, or a socket.

Two consequences worth stating, because they are the parts a reviewer
should check:

- **Nothing in `src/render.rs` changes.** The colour language has one
  owner and the art is not it. `art.rs` names its own ramp.
- **The art ramp is not eyeballed.** The twelve entries in
  `surface::palette` each carry *confirmed against a real device in M0*.
  The fifteen bright hues the art uses are read off the manual's palette
  table and nothing more. For a colour that means something that would be
  unacceptable; for a plasma field it is fine, and if one of them reads
  wrong on the device, swap it — no test asserts what colour it *looks*.

## The frame rate

The loop already wakes every 16 ms. Art advances every other wake —
**~31 fps**, which is smooth to the eye and costs at most 200 bytes a
frame on a wire that took a hundred back-to-back 328-byte worst-case
frames without choking ([02](02-surface.md#what-a-frame-costs-on-the-wire)).

The diff does the rest: a plasma field changes most pads most frames and
pays for them, Life's steady state changes a handful, and the Uprising's
holds send nothing at all.

## What this does not do

- **It does not run by itself.** No screensaver, no idle timeout, no
  art after N quiet minutes. A grid that starts animating on its own is a
  grid you have to check before you trust, and the whole product is the
  claim that you do not.
- **It is not interactive.** No pads to press, no paint mode, no Pong.
  Every press exits, and that rule is worth more than a toy.
- **It does not touch anchord.** Art mode spawns nothing, kills nothing,
  and reads no session list.
