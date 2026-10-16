# 02 — The surface

Read out of Novation's *Launchpad Mini [MK3] Programmer's Reference
Manual*, then **checked against the device on 2026-10-16** (firmware
`00 03 04 01`). Claims below are marked ✅ where hardware confirmed them and
⚠️ where they are still the manual's word only — see
[10-open-questions.md](10-open-questions.md) for what is left.

The headline from that session: **the manual is accurate.** Port names,
addressing, the inquiry response and the worked examples all came back
exactly as written. The two things that turned out wrong were both *ours* —
an arithmetic slip in the byte budget, and an assumption about port
exclusivity that does not hold on macOS.

## Two ports, and which one we want

The device exposes **two** USB-MIDI interfaces:

| interface | for |
|---|---|
| `LPMiniMK3 DAW In/Out` | Session mode, faders — the DAW surface |
| `LPMiniMK3 MIDI In/Out` | Custom modes **and Programmer mode** |

✅ Confirmed. CoreMIDI on macOS 15 reports exactly these names.

We want the **MIDI** interface and Programmer mode. DAW mode exists so a DAW
can own the Session layout while leaving the MIDI port free for playing
notes; we have no notes to play, and Programmer mode gives us the whole
surface — 8×8 grid, top row, right column, logo — as one flat address space.

Port names differ per platform and per OS release. **Match on a substring,
never on an exact string**, and never on index: `LPMiniMK3 MIDI` is the
stable part. Linux decorates it with an ALSA client number, Windows with
nothing, macOS CoreMIDI with the device name.

✅ Confirmed, and the substring survives a trap worth naming. CoreMIDI
exposes *two* names per endpoint and `midir` may report either:

```text
kMIDIPropertyName          'LPMiniMK3 MIDI Out'
kMIDIPropertyDisplayName   'Launchpad Mini MK3 LPMiniMK3 MIDI Out'
```

`LPMiniMK3 MIDI` is a substring of **both**, so the rule above holds
whichever one you get. Matching `Launchpad Mini MK3` instead would have
grabbed the DAW endpoints too.

### In and Out are the device's, not yours

The single easiest way to open the wrong port:

```text
our INPUT   reads  the SOURCE       named 'LPMiniMK3 MIDI Out'
our OUTPUT  writes the DESTINATION  named 'LPMiniMK3 MIDI In'
```

The names are written from the **device's** point of view, so they are
backwards from the host's. A surface layer that matches `"MIDI In"` for its
input port opens a destination it can never read from, and the failure is
silent — the port opens, and no press ever arrives.

## Entering and leaving Programmer mode

SysEx header, every message, both directions:

```text
F0 00 20 29 02 0D  <command> ... F7
```

```text
enter programmer   F0 00 20 29 02 0D 0E 01 F7
leave (Live mode)  F0 00 20 29 02 0D 0E 00 F7
readback           F0 00 20 29 02 0D 0E F7
```

Equivalently via *select layout* (`00h`) with layout `7Fh`. Either works;
`0Eh` is the dedicated one and reads better.

**Programmer mode disables the device's Setup menu** (hold Session). A
process that exits without sending the Live-mode message leaves the user
with a Launchpad that has lost a feature and no obvious way to get it back
short of a power cycle. Restoring Live mode on the way out — *including when
signalled* — is therefore a hard requirement, not politeness. See
[08-lifecycle.md](08-lifecycle.md); anchor's `attach` restores the terminal
for exactly the same reason.

## Addressing

In Programmer mode the surface is a 9×9 address space in **decimal
row-column**:

```text
   CC91 CC92 CC93 CC94 CC95 CC96 CC97 CC98 │ CC99  ← top row, logo
   ─────────────────────────────────────────┼─────
     81   82   83   84   85   86   87   88  │ CC89
     71   72   73   74   75   76   77   78  │ CC79
     61   62   63   64   65   66   67   68  │ CC69
     51   52   53   54   55   56   57   58  │ CC59
     41   42   43   44   45   46   47   48  │ CC49
     31   32   33   34   35   36   37   38  │ CC39
     21   22   23   24   25   26   27   28  │ CC29
     11   12   13   14   15   16   17   18  │ CC19
                                              ↑ right column
```

`index = row*10 + col`, **row 1 at the bottom**, col 1 at the left.
✅ Confirmed: `90 0B 05` lit the bottom-left pad, by eye. The 8×8
grid sends Note On/Off; the top row and right column send Control Change.
For *lighting*, both accept either message type — the table says what the
device **sends when pressed**, not what it accepts.

Row 1 being the bottom is a trap worth naming once: the natural way to write
a grid in code is top-left-first, and the hardware is bottom-left-first. Do
the flip in exactly one place, in the surface layer, and never again.

Note Off arrives as either `8n` or `9n` with velocity 0. Handle both.

## Colour

Three channels select the **lighting mode**, and the velocity (or CC value)
selects a **palette entry** 0–127:

| channel | note status | CC status | effect |
|---|---|---|---|
| 1 | `90` | `B0` | static |
| 2 | `91` | `B1` | flashing between this colour (B) and the static/pulsing one (A) |
| 3 | `92` | `B2` | pulsing, dark → full |

**The animation is free.** Flashing is a 50% duty cycle at one beat;
pulsing is a two-beat ramp. Both run on the device's own clock (120 bpm when
no MIDI clock is supplied). A host that wants a blinking pad sets it once
and stops thinking about it — no timer, no repaint loop, no drift. This is
the single biggest reason to prefer the palette channels over RGB for
anything that moves. See [05-interaction.md](05-interaction.md).

Palette entries this design actually uses:

| entry | colour | used for |
|---:|---|---|
✅ **All ten verified on hardware.** The manual's palette is accurate.

| entry | colour | used for | on the device |
|---:|---|---|---|
| 0 | off | unbound slot | unlit — see the note below |
| 1 | dark grey | bound, not running | a dim glow, cooler than "grey" suggests |
| 3 | white | the page you are on; an action button while held | white |
| 5 | red | exited non-zero | red |
| 7 | dark red | anchord unreachable (logo) | a deeper red, clearly distinct from 5 |
| 9 | **amber** | **needs an answer** | orange |
| 13 | yellow | starting | yellow with a green cast |
| 21 | **green** | **running** | green |
| 23 | dark green | exited clean | green — close to 21, but distinguishable |
| 37 | turquoise | page button holding sessions | cyan |
| 45 | blue | action buttons, resting | blue |

Three of these took a judgment a photograph could not make, and all three
were checked by looking, which is the only way they *can* be checked
([09-testing.md](09-testing.md#what-a-green-suite-does-not-prove)):

- **Amber flashing against 0 reads as urgent from across the room.** The
  single perceptual claim the whole attention design rests on.
- **21 and 23 are tellable apart at a glance.** They photograph almost
  identically, and they carry the two most common meanings on the grid —
  *it is alive* versus *it is done*. Confirmed legible in place; worth
  re-checking under different light before trusting it forever.
- **1 is visibly lit against 0.** Less obvious than it sounds, because the
  pads are **physically white silicone** — an *unlit* pad still looks white
  in a lit room, so "off" never reads as black. 1 shows a hotspot that an
  unlit pad does not. That the dark state is white, not black, is worth
  knowing before designing anything around "dark means nothing there".

Every one of these is used by [05-interaction.md](05-interaction.md); there
are no reserved-but-unused entries, because an entry nothing renders is an
entry nobody will notice is wrong.

### The whole surface in one message

```text
F0 00 20 29 02 0D 03 <colourspec>... F7
```

Each `<colourspec>` is `<type> <index> <data...>`:

| type | data | meaning |
|---|---|---|
| 0 | 1 byte | static, palette entry |
| 1 | 2 bytes | flashing: colour B, colour A |
| 2 | 1 byte | pulsing, palette entry |
| 3 | 3 bytes | RGB, 0–127 each |

Up to **81 entries — the entire surface — in one SysEx**. So the render
strategy is settled before it is written: keep the last frame, diff it,
and emit one message containing only the changed pads. A full repaint is
also one message, which makes "repaint everything on reconnect" cheap and
therefore the right recovery.

Note the asymmetry: flashing via SysEx takes **both** colours in one
colourspec, where flashing via channel 2 only sets B and inherits A from
whatever was there. The SysEx form is the one to use — it is total, so the
rendered frame is a pure function of the model, with no dependence on what
was on the pad before.

### The three worked examples, verbatim

The manual gives three, and they are the only concrete bytes in this
document that come from Novation rather than from our reading of it. They
are therefore the goldens — [09-testing.md](09-testing.md#1-the-surface--pure-byte-functions)
asserts the encoder reproduces each one exactly, which is knowable without
hardware:

```text
90 0B 05                      note on, ch 1: pad 11 (bottom-left) static red
91 51 13                      note on, ch 2: pad 81 (top-left) flashing yellow
F0 00 20 29 02 0D 03 00 0B 0D  pad 11 static yellow …
                    01 0B 0C 0D  … pad 11 flashing green/yellow …
                    02 0B 0D F7  … pad 11 pulsing yellow
```

Two traps are visible in the first line alone. `0B` is **11 decimal written
in hex** — the addressing scheme is decimal row-column and the wire is hex,
so `pad(1,1)` is `0x0B` and not `0x11`. And it is the *bottom* row. Both are
exactly the mistakes [09](09-testing.md) wants a test on, and both look
plausible when wrong.

### What a frame costs on the wire

Worth knowing before [throughput](10-open-questions.md) is measured, because
"81 colourspecs" is not one number:

| frame | bytes |
|---|---|
| header + command + terminator | 8 |
| static colourspec | 3 each |
| pulsing colourspec | 3 each |
| flashing colourspec | 4 each |
| RGB colourspec | 5 each |

So a full repaint of all 81 LEDs is **251 bytes** static and **332** in the
pathological all-flashing case. A steady-state diff is a handful of pads and
well under 50.

✅ Both the size and the delivery are confirmed. A 248-byte frame addressing
all 80 non-logo pads lit **every one of them**, in one message, with the
logo correctly untouched. `MIDISend` never blocked and never refused the
328-byte all-flashing form, and the device answered a mode readback after
100 back-to-back worst-case frames — so it neither chokes nor desyncs.

One measurement from that session is worth **not** trusting: a 100-frame
burst clocked at ~60 MB/s, which is impossible over USB MIDI. That number is
the host enqueueing into CoreMIDI and returning, not the device receiving.
The thing that actually closed this question was looking at the grid.

### Text scrolling

```text
F0 00 20 29 02 0D 07 <loop> <speed> <colourspec> <ascii...> F7
F0 00 20 29 02 0D 07 F7                                   ← stop
```

`<speed>` is pads/second; `40h`–`7Fh` is read as negative (scrolls
left-to-right). `<colourspec>` here is `00 <palette>` or `01 <r> <g> <b>`.

`<ascii...>` is **ASCII, and that is a hard constraint, not a preference**:
any byte ≥ `0x80` inside a SysEx is read as a status byte and ends the
message early. Titles are UTF-8, so they are sanitised before they get here
— the rule is in [05-interaction.md](05-interaction.md#the-title-scroll),
and it belongs to this layer to enforce.

Two properties make this usable rather than a gimmick: the scroll **overlays
without destroying** the lighting underneath (it comes back when the scroll
ends), and it **does not cover the top row or the logo**. So the page
indicators and the health light stay readable while a title scrolls across
the grid.

### Other messages worth knowing

```text
brightness     F0 00 20 29 02 0D 08 <0-127> F7
LED sleep      F0 00 20 29 02 0D 09 <0=off,1=on> F7
LED feedback   F0 00 20 29 02 0D 0A <internal> <external> F7
device inquiry F0 7E 7F 06 01 F7   →  F0 7E 00 06 02 00 20 29 13 01 00 00 <v v v v> F7
mode readback  F0 00 20 29 02 0D 0E F7  →  F0 00 20 29 02 0D 0E <0=live,1=prog> F7
```

✅ The inquiry came back byte-exact:

```text
F0 7E 00 06 02 00 20 29 13 01 00 00 00 03 04 01 F7
                └Novation  │  └application, not bootloader
                           └Launchpad Mini MK3        └firmware
```

The **mode readback** is listed here because it turned out to be the only
way to prove the Live-mode restore actually happened, rather than assuming
it from a clean exit. A restore that fails silently is exactly the failure
[08-lifecycle.md](08-lifecycle.md#shutdown-is-not-optional) is written to
prevent, so the thing that verifies it belongs in the loop, not in a
comment. Round-trip for either query is **~1.9 ms** (1.50–2.07 over 7
warm samples), so this is cheap enough to do on every shutdown.

**Turn internal feedback off.** By default the device lights a pad when you
press it, which fights the renderer: a press would briefly show the hardware's
idea of the colour instead of ours, and the pad would be wrong until the next
frame. The model owns every pixel.

Device inquiry is how we confirm we found a Launchpad Mini MK3 and not
something else whose port name happened to match — `00 20 29` is Novation,
`13` is this model, `01` means the application (`11` means it booted into
the bootloader, which is a different device from our point of view).

## The dependency budget

| need | choice | why not the alternative |
|---|---|---|
| MIDI I/O | [`midir`](https://crates.io/crates/midir) | The only maintained cross-platform Rust MIDI crate with real SysEx support. CoreMIDI/ALSA/WinMM behind one API. |
| Launchpad protocol | **ours** | [`launchy`](https://crates.io/crates/launchy) and [`tally-mini-mk3`](https://github.com/Anfeket/tally-mini-mk3) both exist and both are more than we need. The protocol above is roughly 150 lines of byte-pushing with no logic in it; a dependency here buys an abstraction over nine devices we do not own, and costs the ability to say exactly what went on the wire when a pad lights wrong. |
| anchor client | **ours, ported** | anchor's socket is newline-JSON plus a 13-byte binary frame header. teleport's `anchor_client.rs` is the reference; port it rather than inventing a second dialect. |

Three hundred lines of hardware code we own beats a crate that abstracts a
product line. This is the same call anchor made about containerd.

## The port is not exclusive, and we assumed it was

An earlier draft of this document asserted *"Exclusive access. One process
owns the port. If Ableton has it, we do not."* ❌ **That is false on
macOS.** Two processes opened the same endpoint and both talked to the
device; the second got a correct mode readback while the first was holding
the port and driving LEDs.

CoreMIDI multiplexes. It does not hand out exclusive ownership the way ALSA
rawmidi or WinMM do, and it reports no error to either party. So the
failure mode this document expected — *"port is busy, report it"* — never
arrives.

What that actually costs us:

- **Two `tallyd` instances would fight**, each repainting 81 LEDs over the
  other every tick, with nothing in either process noticing. There is no OS
  error to log and no `EBUSY` to retry.
- The mitigation is **ours, not the kernel's**: a single-instance lock, held
  for the life of the process. See
  [08-lifecycle.md](08-lifecycle.md#one-process-no-socket).
- A *different* app holding the device — Ableton, Novation Components — is
  not excluded either. Both will drive it, and the grid will flicker between
  two opinions. That is a real-use hazard rather than a startup error, so it
  is a thing to notice and quit, not a thing to detect.

Worth stating plainly because it inverts the design instinct: the port
needing no arbitration was the reason this document said a daemon was
unavoidable *"whether we want one or not"*
([08-lifecycle.md](08-lifecycle.md#one-process-no-socket)). The daemon is
still unavoidable, but now for the ordinary reason — something has to hold
the state — rather than because the OS enforces it.

## What the hardware cannot do

Stated now so nobody designs around it later:

- **No per-pad text.** 64 pads, one colour each. Characters scroll across the
  whole grid or not at all.
- **No pressure, no velocity in.** The Mini is unweighted — press is press.
  (The Launchpad X has velocity; this does not.)
- **No hot-plug notification.** `midir` enumerates ports; it does not tell
  you when one appears. Poll the port list. Two seconds is the cadence
  teleport's supervisor uses for the same kind of reconcile, and there is no
  reason to be cleverer.
- **Latency is not a problem.** ✅ A host→device→host round trip measures
  **~1.9 ms**. The ~10 ms budget that hold-then-tap chording needs has a 5×
  margin, so the gesture design in
  [05-interaction.md](05-interaction.md#the-exact-rules-of-a-chord) stands.
- **Colour is approximate.** The palette is 128 fixed entries chosen for
  music software. RGB mode exists but costs the free animation. Pick the
  palette, and pick colours that differ in *brightness* as well as hue —
  the thing being optimised is legibility from across a room.
