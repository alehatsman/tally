# 03 — The surface

The hardware: a Novation Launchpad Mini MK3, in programmer mode, over USB
MIDI.

Read out of Novation's *Programmer's Reference Manual*, then **checked
against the device on 2026-10-16** (firmware `00 03 04 01`). **The manual is
accurate.** Port names, addressing, the inquiry response and the worked
examples all came back exactly as written. The two things that turned out
wrong were both *ours* — an arithmetic slip in the byte budget, and an
assumption about port exclusivity that does not hold on macOS.

Everything below is `src/surface/` (pure: addressing, palette, the bytes)
and `src/device.rs` (the one I/O type, `Device`). `Device` has two ways of
moving the same bytes: a local `midir` port, or — when `TALLY_BRIDGE` names
an address — a socket to `tallybridge`, which holds the port on the far
side of the WSL line ([13](13-bridge.md)). Nothing above `Device` can tell
which. The bridge changes no byte in this document, only who opens the port: the
substring match below runs on the bridge's side of the line, and the
Live-mode restore becomes the bridge's own duty when `tallyd` is gone.

## Two ports, and which one we want

| interface | for |
|---|---|
| `LPMiniMK3 DAW In/Out` | Session mode, faders — the DAW surface |
| `LPMiniMK3 MIDI In/Out` | Custom modes **and Programmer mode** |

We want the **MIDI** interface and Programmer mode, which gives the whole
surface — 8×8 grid, top row, right column, logo — as one flat address space.

Port names differ per platform and per OS release. **Match on a substring,
never on an exact string**, and never on index: `LPMiniMK3 MIDI` is the
stable part. CoreMIDI exposes *two* names per endpoint and `midir` may
report either —

```text
kMIDIPropertyName          'LPMiniMK3 MIDI Out'
kMIDIPropertyDisplayName   'Launchpad Mini MK3 LPMiniMK3 MIDI Out'
```

— and `LPMiniMK3 MIDI` is a substring of **both**. Matching
`Launchpad Mini MK3` instead would have grabbed the DAW endpoints too.

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

### The port is not exclusive, and we assumed it was

❌ An early draft asserted *"one process owns the port"*. **False on
macOS.** CoreMIDI multiplexes: two processes opened the same endpoint and
both talked to the device, the second getting a correct mode readback while
the first was holding the port and driving LEDs. No error to either party.

So the failure mode this design expected — *"port is busy, report it"* —
never arrives. What it costs:

- **Two `tallyd` instances would fight**, each repainting 81 LEDs over the
  other every tick, with nothing noticing. The mitigation is **ours, not the
  kernel's**: a single-instance `flock` ([08](08-daemon.md#one-process-no-socket)).
- A *different* app holding the device — Ableton, Novation Components — is
  not excluded either. The grid flickers between two opinions. A real-use
  hazard rather than a startup error, so it is a thing to notice and quit,
  not a thing to detect.

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

**Programmer mode disables the device's Setup menu** (hold Session). A
process that exits without sending the Live-mode message leaves a Launchpad
that has lost a feature with no obvious way to get it back short of a power
cycle. Restoring Live mode on the way out — *including when signalled* — is
a hard requirement, not politeness ([08](08-daemon.md#shutdown-is-not-optional)).

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

`index = row*10 + col`, **row 1 at the bottom**, col 1 at the left. The 8×8
grid sends Note On/Off; the top row and right column send Control Change.
For *lighting*, both accept either message type — the table says what the
device **sends when pressed**, not what it accepts.

Row 1 being the bottom is a trap worth naming once: the natural way to write
a grid in code is top-left-first, and the hardware is bottom-left-first. Do
the flip in exactly one place, in the surface layer, and never again.

Note Off arrives as either `8n` or `9n` with velocity 0. Handle both.

## Colour

Three channels select the **lighting mode**; the velocity (or CC value)
selects a palette entry 0–127:

| channel | note status | CC status | effect |
|---|---|---|---|
| 1 | `90` | `B0` | static |
| 2 | `91` | `B1` | flashing between this colour (B) and the static/pulsing one (A) |
| 3 | `92` | `B2` | pulsing, dark → full |

**The animation is free.** Flashing is a 50% duty cycle at one beat; pulsing
is a two-beat ramp. Both run on the device's own clock (120 bpm when no MIDI
clock is supplied). A host that wants a blinking pad sets it once and stops
thinking about it — no timer, no repaint loop, no drift. This is the single
biggest reason to prefer the palette channels over RGB for anything that
moves.

**All eleven verified on hardware.** `src/surface/palette.rs` is the one
definition; what each one *means* is [05](05-interaction.md):

| entry | colour | used for | on the device |
|---:|---|---|---|
| 0 | off | nothing here, and nothing this gesture would touch | unlit — see the note below |
| 1 | dark grey | **available, not urgent** — a live arrow, a `>` that would launch, `Drums`, a pad a held modifier will skip, and under a held Move with a pad in hand, every pad it could land on | a dim glow, cooler than "grey" suggests |
| 3 | white | **what your hand is doing** — a held modifier, `◄` with an origin, `Drums` while an art is playing | white |
| 5 | red | exited non-zero; the corner resting in Stop; a pad that hold would terminate | red |
| 7 | dark red | *the daemon had a problem* — a `lost` session, an unreachable anchord on the logo; and a pad whose ended rows a held Stop would forget | a deeper red, clearly distinct from 5 |
| 9 | **amber** | **needs an answer** — and the only thing that moves on the resting surface | orange |
| 13 | yellow | nothing in the colour language. The Uprising's smile, which is why ([05](05-interaction.md#the-arts)) | yellow with a green cast |
| 21 | **green** | **alive**, static, working and quiet alike | green |
| 23 | dark green | unused. It *was* "exited clean" ([D1](10-decisions.md#d1--three-greens)) | green — close to 21, but distinguishable |
| 37 | turquoise | Move at rest, and **a session is here** under a held Move | cyan |
| 45 | blue | unused. It *was* the action buttons at rest, until kill took the colour of its own consequence | blue |

Three of these took a judgment a photograph could not make, and all three
were checked by looking, which is the only way they *can* be checked:

- **Amber reads as urgent from across the room.** The single perceptual
  claim the whole attention design rests on. Checked first as a hard flash,
  which worked, then changed to a pulse and re-checked: still catches the
  eye ([D2](10-decisions.md#d2--the-hard-amber-flash)).
- **21 and 23 are tellable apart at a glance.** They photograph almost
  identically. Confirmed legible in place, once, in one room; worth
  re-checking under different light before trusting it forever.
- **1 is visibly lit against 0.** Less obvious than it sounds, because the
  pads are **physically white silicone** — an *unlit* pad still looks white
  in a lit room, so "off" never reads as black. 1 shows a hotspot that an
  unlit pad does not. Worth knowing before designing anything around "dark
  means nothing there".

**Two entries render nothing and are kept rather than deleted.** What the
device does with 23 and 45 is a fact that cost a device and an afternoon,
and the sentence beside each is why it stopped being spent. 13 and 37 are
the proof it was worth keeping them: both were retired, and both came back.

## The whole surface in one message

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
strategy is settled before it is written: keep the last frame, diff it, emit
one message containing only the changed pads. A full repaint is also one
message, which makes "repaint everything on reconnect" cheap and therefore
the right recovery.

Note the asymmetry: flashing via SysEx takes **both** colours in one
colourspec, where flashing via channel 2 only sets B and inherits A from
whatever was there. The SysEx form is the one to use — it is total, so the
rendered frame is a pure function of the model.

### The three worked examples, verbatim

The only concrete bytes here that come from Novation rather than from our
reading of them. They are the goldens ([09](09-testing.md)):

```text
90 0B 05   note on, ch 1: pad 11 (bottom-left) static red
91 51 13   note on, ch 2: pad 81 (top-left) flashing yellow

F0 00 20 29 02 0D 03     SysEx, three colourspecs:
   00 0B 0D                pad 11 static yellow
   01 0C 0C 0D             pad 12 flashing green/yellow
   02 0D 0D              pad 13 pulsing yellow
F7
```

Two traps are visible in the first line alone. `0B` is **11 decimal written
in hex** — the addressing scheme is decimal row-column and the wire is hex,
so `pad(1,1)` is `0x0B` and not `0x11`. And it is the *bottom* row. Both
look plausible when wrong.

### What a frame costs on the wire

| frame | bytes |
|---|---|
| header + command + terminator | 8 |
| static colourspec | 3 each |
| pulsing colourspec | 3 each |
| flashing colourspec | 4 each |
| RGB colourspec | 5 each |

A full repaint of all 81 LEDs is **251 bytes** static and **332** in the
pathological all-flashing case. A steady-state diff is a handful of pads and
well under 50.

Both the size and the delivery are confirmed. A 248-byte frame addressing
all 80 non-logo pads lit every one of them, in one message, with the logo
correctly untouched. `MIDISend` never blocked and never refused the 328-byte
all-flashing form, and the device answered a mode readback after 100
back-to-back worst-case frames — so it neither chokes nor desyncs.

One measurement from that session is worth **not** trusting: a 100-frame
burst clocked at ~60 MB/s, which is impossible over USB MIDI. That number is
the host enqueueing into CoreMIDI and returning, not the device receiving.
The thing that actually closed this question was looking at the grid.

## MIDI clock drives the breathing

```text
clock tick   F8    24 per quarter note, on the open port
clock start  FA    once, when tempo is first established
clock stop   FC    on shutdown, so the device returns to its own 120
```

The device **does** sync its LED animations to MIDI clock on the
`LPMiniMK3 MIDI` port — measured against a live grid with
`examples/clock.rs`, five phases with silent baselines either side:
no clock → the device's own 120; 60 bpm → breathed slower; 150 → faster; 60
again → slower; `FC` → back to 120.

This trades *"set it once and forget"* for one byte at 24–56 Hz, forever.
What is kept is the important part: the **ramp** still runs on the device,
so a pulse is still smooth under load and does not stutter. What is given up
is only the beat boundary. The mapping is in
[05](05-interaction.md#the-tempo-says-how-long).

Nothing about the clock is in `Frame`. The tempo is not part of a picture —
it is the rate the picture is drawn at, and keeping it out of `Frame` keeps
the renderer a pure function of state.

## Text scrolling

```text
F0 00 20 29 02 0D 07 <loop> <speed> <colourspec> <ascii...> F7
F0 00 20 29 02 0D 07 F7                                   ← stop
```

`<speed>` is pads/second; `40h`–`7Fh` is read as negative (scrolls
left-to-right). `<ascii...>` is **ASCII, and that is a hard constraint**:
any byte ≥ `0x80` inside a SysEx is read as a status byte and ends the
message early.

The scroll overlays without destroying the lighting underneath, and does not
cover the top row or the logo.

> **tally does not use this.** Everything above is accurate and was
> implemented correctly. It was then held down once on real hardware and
> deleted: eight pads wide is about two legible characters at a time
> ([D3](10-decisions.md#d3--the-title-scroll)). Kept documented because the
> device does do this and the next person will want to try it.
>
> `STOP_SCROLL` stays in the code regardless: it is how a device left
> mid-scroll by anything else is put back.

## Other messages worth knowing

```text
brightness     F0 00 20 29 02 0D 08 <0-127> F7
LED sleep      F0 00 20 29 02 0D 09 <0=off,1=on> F7
LED feedback   F0 00 20 29 02 0D 0A <internal> <external> F7
device inquiry F0 7E 7F 06 01 F7   →  F0 7E 00 06 02 00 20 29 13 01 00 00 <v v v v> F7
mode readback  F0 00 20 29 02 0D 0E F7  →  F0 00 20 29 02 0D 0E <0=live,1=prog> F7
```

The inquiry came back byte-exact:

```text
F0 7E 00 06 02 00 20 29 13 01 00 00 00 03 04 01 F7
                └Novation  │  └application, not bootloader
                           └Launchpad Mini MK3        └firmware
```

It is how we confirm we found a Launchpad Mini MK3 and not something else
whose port name happened to match — `00 20 29` is Novation, `13` is this
model, `01` means the application (`11` means it booted into the
bootloader, which is a different device from our point of view).

The **mode readback** is the only way to prove the Live-mode restore
actually happened, rather than assuming it from a clean exit. Round trip for
either query is **~1.9 ms** (1.50–2.07 over 7 warm samples), so it is cheap
enough to do on every shutdown and every two seconds as a heartbeat.

**Turn internal feedback off.** By default the device lights a pad when you
press it, which fights the renderer: a press would briefly show the
hardware's idea of the colour instead of ours. The model owns every pixel.

**Brightness (`08h`) and LED sleep (`09h`) are documented and unspent.** Dim
the whole surface while nothing wants you, full brightness the moment
something does; the same thought overnight. Both are orthogonal to the
colour language — but the colour language is built on *bright means where
you are*, so it is the wrong axis to lean on, and a separate, smaller idea
if it is ever specced.

## What the hardware cannot do

- **No per-pad text.** 64 pads, one colour each. Characters scroll across
  the whole grid or not at all.
- **No pressure, no velocity in.** The Mini is unweighted — press is press.
  Every gesture is built from *which button*, *down or up*, and *time*.
- **No hot-plug notification, in either direction.** `midir` enumerates
  ports; it does not tell you when one appears. Poll, every two seconds.

  **And a failed write is not how you learn it went away.** ❌ An early
  draft assumed the send would error once the device was unplugged. It does
  not: CoreMIDI accepts sends to a removed endpoint without complaint, so
  `render` keeps returning `Ok` into a void while the host goes on diffing
  against a frame nothing has. Observed — the Launchpad was unplugged and
  replugged, came back dark, and `tallyd` never noticed.

  The only signal is **absence of a reply**. A mode readback every two
  seconds costs ~2 ms; three missed probes means the device is gone. The
  reply doubles as a mode check, which matters because **a replugged
  Launchpad comes back in Live mode** — so a host that kept its connection
  has to notice it is no longer in programmer mode and re-enter it, or the
  grid stays dark forever with nothing in the model having changed.

  Over the bridge a hang-up *is* knowable: the socket closes, and
  `Device::render` fails on the next tick rather than after the silence
  window ([D17](10-decisions.md#d17--a-failed-write-is-not-how-you-learn-the-device-went-away)).
  A cable pulled mid-session behind a live bridge is not yet established
  — [13](13-bridge.md#the-checklist-when-the-launchpad-is-plugged-in) item 8
  is still owed — and until it is, the seven-second probe silence above is
  the backstop.
- **Latency is not a problem.** A host→device→host round trip measures
  **~1.9 ms**. The ~10 ms budget hold-then-tap chording needs has a 5×
  margin.
- **Colour is approximate.** 128 fixed palette entries chosen for music
  software. RGB mode exists but costs the free animation. Pick the palette,
  and pick colours that differ in *brightness* as well as hue — the thing
  being optimised is legibility from across a room.

## The dependency budget

| need | choice | why not the alternative |
|---|---|---|
| MIDI I/O | [`midir`](https://crates.io/crates/midir) | the only maintained cross-platform Rust MIDI crate with real SysEx support. CoreMIDI/ALSA/WinMM behind one API |
| Launchpad protocol | **ours** | `launchy` and `tally-mini-mk3` both exist and both are more than we need. The protocol above is roughly 150 lines of byte-pushing with no logic in it; a dependency here buys an abstraction over nine devices we do not own, and costs the ability to say exactly what went on the wire when a pad lights wrong |
| anchor client | **ours, ported** | anchor's socket is newline-JSON plus a 13-byte binary frame header. teleport's `anchor_client.rs` is the reference; port it rather than inventing a second dialect of the same protocol, which is a second place for the offset arithmetic to be subtly wrong |

Three hundred lines of hardware code we own beats a crate that abstracts a
product line. Total: `libc`, `midir`, `serde`, `serde_json`, `toml` — and
`tallybridge`, which is ours: the bridge's wire, std-only, in this
repository ([13](13-bridge.md)). No async runtime — the 16 ms polling loop the device needs is the only clock
this program has.
