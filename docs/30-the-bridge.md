# 30 — The bridge

The Launchpad is on a Windows desk. The agents are in WSL. USB does not
cross that line without a custom kernel — the stock WSL2 kernel is built
without `CONFIG_SND_USB_AUDIO`, so `usbipd attach` hands over a device
that never appears under `/dev/snd` and `midir`'s ALSA backend finds no
port at all.

So the process is cut in two. **`tallybridge` runs on Windows and owns the
device. `tallyd` runs in WSL and owns everything else.** They share a
socket.

```text
   Windows                          │  WSL2
                                    │
   Launchpad ──USB──▶ tallybridge ──┼──TCP──▶ tallyd ──▶ tmux ──▶ anchord
                           │        │            │
                           │        │            └─ switch-client
                           └─ raise the window   │   (the pane, WSL-side)
                                    │
```

## What the bridge is not

It is not half of tally. It holds no model, no chord state, no frame
diffing, no `Button` decoding and no knowledge of what a pad means. Every
sentence of [02](02-surface.md) stays on the WSL side, where its tests
are. The bridge moves bytes and raises a window.

That is the whole design rule, and it decides every question below. When
something could live on either side, it lives in `tallyd`.

## The seam

[`src/device.rs`](../src/device.rs) is already the line. `Device` finds
the port, sends SysEx, decodes presses, and restores Live mode on drop;
`Link` in `tallyd.rs` owns one and reconnects it on the reconcile clock.
Nothing above `Link` knows a MIDI port exists.

Two implementations behind one interface:

| | owns | where |
|---|---|---|
| `device::Local` | a `midir` connection | today's `Device`, unchanged behaviour |
| `device::Remote` | a socket to the bridge | new |

`Device` becomes a trait with the method set `Link` already calls —
`ping`, `reprogram`, `render`, `repaint`, `poll`, `restore`, `abandon`,
`clock`, `clock_start`, `clock_stop`. `open` stays an inherent associated
function on each impl, because it returns `Self`.

**One signature has to change.** `Device::poll` returns
`impl Iterator + '_`, which is not object-safe. It becomes
`fn poll(&self) -> Vec<Event>`. The single caller at `tallyd.rs` already
does `.collect()` into a `Vec`, so this is a move, not a cost.

## The wire

Length-prefixed binary frames. One byte of kind, four bytes of big-endian
length, then the payload.

```text
kind  len          payload
0x01  00 00 00 01  F8                  tallyd → bridge, to the device
0x01  00 00 01 9C  F0 00 20 29 ...     tallyd → bridge, a full repaint
0x02  00 00 00 03  90 0B 7F            bridge → tallyd, from the device
0x10  00 00 00 0F  {"verb":"raise"}    tallyd → bridge, control
0x11  00 00 00 14  {"device":"gone"}   bridge → tallyd, control
```

`len` is capped at **64 KiB**. A peer that announces more is hung up on
rather than allocated for; the largest real frame is a repaint at a few
hundred bytes.

Kinds `0x01` and `0x02` carry raw MIDI and are never inspected by the
bridge. Kinds `0x10` and `0x11` carry one JSON object, which is why
`serde_json` — already a dependency — is the only thing the codec needs.

### One MIDI message per frame

The bridge must not coalesce. `decode` in `device.rs` matches on a whole
message, and `midir` already hands the callback exactly one; if two
presses were allowed to share a frame because TCP delivered them
together, `decode` would return `None` and both would vanish. The frame
boundary **is** the message boundary, in both directions.

### `TCP_NODELAY`, on both ends

Not an optimisation. The tempo ([17](17-the-tempo.md)) is a stream of
single-byte `0xF8` clock pulses at 24 PPQN, and Nagle's algorithm exists
specifically to hold small writes back until the previous one is
acknowledged. Left on, the clock arrives in coalesced bursts and the
device's two-beat breath stutters.

### Who listens

**`tallyd` listens; the bridge connects.** `TALLY_BRIDGE_LISTEN`,
defaulting to `127.0.0.1:7373`; the bridge takes `--connect host:port`.

This direction is chosen because WSL2 forwards Windows `localhost` to a
listener inside WSL with no configuration: no Windows Firewall rule, no
reading the host address out of `/etc/resolv.conf`, and it behaves the
same under NAT and mirrored networking. Connecting the other way needs
all three.

⚠️ **Unverified on hardware.** Whether localhost forwarding reaches a
listener bound to `127.0.0.1` or needs `0.0.0.0` has changed across WSL
releases. `TALLY_BRIDGE_LISTEN` exists so that is a config line and not a
patch. Binding `0.0.0.0` puts the socket on the local network — if that
turns out to be required, it needs a note here saying so.

## The control verbs

| frame | direction | meaning |
|---|---|---|
| `{"verb":"raise"}` | → bridge | bring the terminal forward |
| `{"verb":"open"}` | → bridge | no terminal exists; make one |
| `{"device":"present"}` | → tallyd | a Launchpad is on the port |
| `{"device":"gone"}` | → tallyd | the port went away |

`raise` replaces the hardcoded macOS `open -a` at `jump.rs`, `raise()`.
On Windows the bridge shells out to PowerShell — `AppActivate` against
the terminal's window title — for the same reason the macOS path shells
out to `open`: it is the one genuinely machine-specific string in the
file, and it is best-effort. **A failed raise is never an error.** tmux
has already switched the pane, so the move has happened; the window just
did not come forward.

Default terminal is Windows Terminal. `TALLY_TERMINAL` names the window
title to activate; `TALLY_TERMINAL_CMD` overrides the whole command, as
on macOS.

### Presence is a message, not a guess

`Device::open` returns `NotFound` when nothing matches
`PORT_SUBSTRING`. Over a socket there are two different silences — the
bridge is down, or the bridge is up and the Launchpad is unplugged —
and they want the same grid but different logs. The bridge sends
`device` on connect and on every change.

The identity handshake is **not** special-cased. `Remote` sends
`sysex::INQUIRY` down the pipe and waits for the reply exactly as the
local path does, and `REPLY_TIMEOUT` stays at 2s: it was set generous for
a ~345ms cold round-trip, and a loopback hop is noise beside that.

## Restore is the bridge's own duty

This is the one place the rule above is broken on purpose.

Programmer mode disables the device's Setup menu, and `Device::drop`
exists so that every path out puts it back ([08](08-lifecycle.md)). If
`tallyd` dies, or the socket drops, or WSL shuts down, the WSL side can
no longer reach the device — the exact moment the restore is needed is
the moment it cannot be sent from there.

So **the bridge restores Live mode itself** on client disconnect and on
its own exit: stop scroll, clear all 81 LEDs, Live, readback. Three
constant byte strings, duplicated from `surface::sysex` onto the Windows
side, with this paragraph as the reason. They are constants in the
manual and they do not move.

## Lifecycle

| state | tallyd | bridge | grid |
|---|---|---|---|
| both up, device present | renders | pipes | live |
| bridge not started | listens, logs once | — | dark |
| bridge up, Launchpad unplugged | `NotFound`, retries on the reconcile clock | sends `gone`, polls the port | dark |
| socket drops | drops the device, keeps listening | restores Live, retries `--connect` | dark, Setup menu intact |
| tallyd restarts | relistens; first frame is a full repaint | reconnects | repainted from scratch |
| bridge killed hard | sees EOF, drops the device | — | ⚠️ Setup menu left disabled; replug fixes it |

Reconnection on both sides reuses the existing policy: retry on the
reconcile clock, never give up, and `abandon()` the device so the way out
does not warn about a restore that could not have worked.

## What this does not change

- `anchord`, every PTY, and every agent stay in WSL and are untouched.
- `switch-client` stays in WSL. The bridge never learns a pane address;
  tally's tmux server ([24](24-the-tally-server.md)) is entirely WSL-side.
- macOS keeps the local path. `Remote` is only reached when
  `TALLY_BRIDGE_LISTEN` is set, so a Mac with a Launchpad on it behaves
  exactly as it does today, and nothing in this document is on that path.
- The dependency budget. `tallybridge` is a third `[[bin]]` in this crate
  so the codec has one definition compiled into both sides, and it adds
  no dependency `tallyd` does not already have.

## Validation

Unit, no hardware:

- codec round-trips every kind; a frame split across two reads
  reassembles; a truncated frame is not a message; an oversize `len` is a
  hang-up and not an allocation.
- `Remote` drives a fake bridge in-process: identity handshake, a press
  arriving as one `Event`, `render` diffing exactly as the local path
  does, `poll` draining.
- `device` presence transitions produce the same `Link` states the local
  `NotFound` path produces.

⚠️ Needs the actual Windows box, and none of it can be argued in advance:

| | what it would settle |
|---|---|
| WinMM port name | `docs/02:33` claims Windows decorates it with nothing. Never run. |
| localhost forwarding | which bind address actually works |
| clock jitter over the hop | whether the two-beat breath survives TCP |
| `AppActivate` | how often Windows refuses the foreground steal |
| a day beside it | the only instrument that has ever caught anything here |

That last row is not a formality. The hard amber flash and the green
working pulse both passed a glance on real hardware and failed a day of
use, and this adds a network hop to the surface those were measured on.
