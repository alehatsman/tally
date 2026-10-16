# 02 — The surface

Everything here is read out of Novation's *Launchpad Mini [MK3]
Programmer's Reference Manual*. **None of it is verified against hardware
yet** — see [10-open-questions.md](10-open-questions.md). The manual is the
only contract; the device was not plugged into this machine.

## Two ports, and which one we want

The device exposes **two** USB-MIDI interfaces:

| interface | for |
|---|---|
| `LPMiniMK3 DAW In/Out` | Session mode, faders — the DAW surface |
| `LPMiniMK3 MIDI In/Out` | Custom modes **and Programmer mode** |

We want the **MIDI** interface and Programmer mode. DAW mode exists so a DAW
can own the Session layout while leaving the MIDI port free for playing
notes; we have no notes to play, and Programmer mode gives us the whole
surface — 8×8 grid, top row, right column, logo — as one flat address space.

Port names differ per platform and per OS release. **Match on a substring,
never on an exact string**, and never on index: `LPMiniMK3 MIDI` is the
stable part. Linux decorates it with an ALSA client number, Windows with
nothing, macOS CoreMIDI with the device name.

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

`index = row*10 + col`, **row 1 at the bottom**, col 1 at the left. The 8×8
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

Palette entries this design actually uses (from the manual's table and
examples; **verify on hardware**):

| entry | colour | used for |
|---:|---|---|
| 0 | off | unbound slot |
| 1 | dark grey | bound, not running |
| 3 | white | the focused slot |
| 5 | red | exited non-zero |
| 7 | dark red | anchord unreachable (logo) |
| 9 | amber | needs an answer |
| 13 | yellow | starting |
| 21 | green | running |
| 23 | dark green | exited clean |
| 37 | turquoise | reserved — page markers |
| 45 | blue | reserved — action buttons |

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

### Text scrolling

```text
F0 00 20 29 02 0D 07 <loop> <speed> <colourspec> <ascii...> F7
F0 00 20 29 02 0D 07 F7                                   ← stop
```

`<speed>` is pads/second; `40h`–`7Fh` is read as negative (scrolls
left-to-right). `<colourspec>` here is `00 <palette>` or `01 <r> <g> <b>`.

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
```

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
- **Exclusive access.** One process owns the port. If Ableton has it, we do
  not, and the failure must be reported rather than retried silently.
- **Colour is approximate.** The palette is 128 fixed entries chosen for
  music software. RGB mode exists but costs the free animation. Pick the
  palette, and pick colours that differ in *brightness* as well as hue —
  the thing being optimised is legibility from across a room.
