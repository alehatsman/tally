# 30 — The bridge

The Launchpad is on a Windows desk. The agents are in WSL. USB does not
cross that line: the stock WSL2 kernel is built without
`CONFIG_SND_USB_AUDIO`, so `usbipd attach` hands over a device that never
appears under `/dev/snd` and `midir`'s ALSA backend enumerates no port at
all. Building a kernel is the other answer and this document is not it.

So the process is cut in two. **`tallybridge` runs on Windows and owns the
device. `tallyd` runs in WSL and owns everything else.**

```text
   Windows                            │  WSL2
                                      │
   Launchpad ──USB──▶ tallybridge ────┼──TCP──▶ tallyd ──▶ tmux ──▶ anchord
                                      │           │
                      powershell.exe ◀┼───────────┘  raise, by interop
                      (raise the window)
```

## What the bridge is not

It holds no model, no chord state, no frame diffing, no `Button` decoding
and no idea what a pad means. Every sentence of [02](02-surface.md) stays
on the WSL side, where its tests are. The bridge moves bytes.

That is the design rule, and it decides every question below: **when
something could live on either side, it lives in `tallyd`.** There is one
deliberate exception, argued in [Restore](#restore-is-the-bridges-own-duty).

## The seam

[`src/device.rs`](../src/device.rs) is already the line. `Device` finds
the port, sends SysEx, decodes presses and restores Live mode on drop;
`Link` in `tallyd.rs` owns one and reconnects it on the reconcile clock.
Nothing above `Link` knows a MIDI port exists.

| | owns | where |
|---|---|---|
| `device::Local` | a `midir` connection | today's `Device`, behaviour unchanged |
| `device::Remote` | a socket to the bridge | new |

`Device` becomes a **trait** with the method set `Link` already calls —
`ping`, `reprogram`, `render`, `repaint`, `poll`, `restore`, `abandon`,
`clock`, `clock_start`, `clock_stop`. `open` stays an inherent associated
function on each impl, because it returns `Self`.

**One signature has to change.** `poll` returns `impl Iterator + '_`,
which is not object-safe, so it becomes `fn poll(&self) -> Vec<Event>`.
The single caller already does `.collect()` into a `Vec`
(`tallyd.rs`, the `events` binding in `main`), so this is a move, not a
cost.

`Link::device` becomes `Option<Box<dyn Device>>` and `Link::connect`
calls a factory that reads the environment once:
`TALLY_BRIDGE_LISTEN` set → `Remote::open`, unset → `Local::open`.
**A Mac with a Launchpad on it never reaches the new path.**

## The wire

Length-prefixed binary frames. One byte of kind, four bytes of big-endian
length, then the payload.

```text
kind  len          payload
0x01  00 00 00 01  F8                  tallyd → bridge, to the device
0x01  00 00 01 9C  F0 00 20 29 ...     tallyd → bridge, a full repaint
0x02  00 00 00 03  90 0B 7F            bridge → tallyd, from the device
0x11  00 00 00 14  {"device":"gone"}   bridge → tallyd, control
```

`len` is capped at **64 KiB**. A peer announcing more is hung up on rather
than allocated for; the largest real frame is a repaint at a few hundred
bytes.

Kinds `0x01` and `0x02` carry raw MIDI and the bridge never inspects
them. Kind `0x11` carries one JSON object, which is why `serde_json` —
already a dependency — is all the codec needs. **`0x10`, tallyd → bridge
control, is reserved and currently unused**; see
[Raise](#raise-does-not-need-the-bridge).

### One MIDI message per frame

The bridge must not coalesce. `decode` in `device.rs` matches on a whole
message and `midir` hands its callback exactly one; if two presses shared
a frame because TCP delivered them together, `decode` would return `None`
and both would vanish. The frame boundary **is** the message boundary, in
both directions.

### `TCP_NODELAY`, on both ends

Not an optimisation. The tempo ([17](17-the-tempo.md)) is a stream of
single-byte `0xF8` clock pulses at 24 PPQN, and Nagle's algorithm exists
to hold small writes back until the previous one is acknowledged. Left
on, the clock arrives in bursts and the device's two-beat breath
stutters.

### Who listens

**`tallyd` listens; the bridge connects.** `TALLY_BRIDGE_LISTEN`,
suggested `127.0.0.1:7373`; the bridge takes `--connect host:port`.

This direction is chosen because WSL2 forwards Windows `localhost` to a
listener inside WSL with no configuration: no Windows Firewall rule, no
reading the host address out of `/etc/resolv.conf`, and the same
behaviour under NAT and mirrored networking. Connecting the other way
needs all three.

⚠️ **Unverified.** Whether that forwarding reaches a listener bound to
`127.0.0.1` or needs `0.0.0.0` has changed across WSL releases.
`TALLY_BRIDGE_LISTEN` takes a full address so that is a config line and
not a patch. **If `0.0.0.0` turns out to be required, the socket is on
the local network and this document must say so in plain words** — there
is no authentication on it, and a frame on it drives the grid.

### Presence is a message, not a guess

`Local::open` returns `NotFound` when nothing matches `PORT_SUBSTRING`.
Over a socket there are two different silences — the bridge is down, or
the bridge is up and the Launchpad is unplugged. They want the same grid
and different logs, so the bridge sends `{"device":...}` on connect and
on every change.

The identity handshake is **not** special-cased. `Remote` sends
`sysex::INQUIRY` down the pipe and waits for the reply exactly as the
local path does, and `REPLY_TIMEOUT` stays at 2s: it was set generous for
a ~345ms cold round-trip and a loopback hop is noise beside that.

`DEVICE_SILENCE` (7s) and the `ping` that feeds it are unchanged and now
cover the whole chain. A bridge that is up but wedged looks exactly like
a Launchpad that stopped answering, which is the correct reading of it.

## Raise does not need the bridge

The first draft of this document had a `{"verb":"raise"}` control frame,
because `raise()` in `jump.rs` is a hardcoded macOS `open -a` and the
window to raise is a Windows window. That was wrong, and the reason is
worth keeping.

**WSL runs Windows binaries.** Interop is on by default, so `tallyd` can
invoke `powershell.exe` directly from WSL — the same shape as the macOS
path, a best-effort shell-out to the one genuinely machine-specific
string in the file. No socket handle threaded into `jump.rs`, no control
verb, no coupling between the window manager and the MIDI pipe.

So `raise()` gains an override and nothing else:

| env | default | on this desk |
|---|---|---|
| `TALLY_RAISE_CMD` | `open -a $TALLY_TERMINAL` (macOS, as today) | a `powershell.exe` `AppActivate` against the window title |
| `TALLY_TERMINAL_CMD` | `open -na ...` (exists already) | `wt.exe new-tab wsl.exe ...` |

Both are **split on spaces and are not a shell**, matching
`terminal_command`'s existing rule. A command needing quoting goes in a
`.ps1` or a `.cmd` and the env var names that file.

The terminal window's title is `HOME_SESSION` — `tally` — because
`terminal_command` already passes `--title tally`. That is the string
`AppActivate` matches.

⚠️ **Unverified, and two ways it can fail.** Windows restricts
foreground stealing, so `AppActivate` may refuse. Interop can be turned
off in `/etc/wsl.conf`, in which case `powershell.exe` is not executable
from WSL and the reserved `0x10` control frame becomes the fallback.
**A failed raise is never an error** — tmux has already switched the
pane, so the move has happened and only the window did not come forward.

## Restore is the bridge's own duty

The one place the design rule is broken, on purpose.

Programmer mode disables the device's Setup menu, and `Device::drop`
exists so every path out puts it back ([08](08-lifecycle.md)). If
`tallyd` dies, or the socket drops, or WSL shuts down, the WSL side can
no longer reach the device — **the moment the restore is needed is the
moment it cannot be sent from there.**

So the bridge restores Live mode itself, on client disconnect and on its
own exit: `STOP_SCROLL`, a dark repaint, `LIVE`, then `MODE_READBACK` and
a readback, because a write is not an acknowledgement. Those constants
are duplicated from `surface::sysex` onto the Windows side with this
paragraph as the reason. They are fixed bytes out of Novation's manual
and they do not move.

## Lifecycle

| state | tallyd | bridge | grid |
|---|---|---|---|
| both up, device present | renders | pipes | live |
| bridge not started | listens, logs once | — | dark |
| bridge up, Launchpad unplugged | `NotFound`, retries on the reconcile clock | sends `gone`, polls the port | dark |
| socket drops | drops the device, keeps listening | restores Live, retries `--connect` | dark, Setup menu intact |
| bridge wedged, socket open | `DEVICE_SILENCE` fires, drops it | — | dark |
| tallyd restarts | relistens; first frame is a full repaint | reconnects | repainted from scratch |
| bridge killed hard (`taskkill /F`) | sees EOF, drops the device | — | ⚠️ Setup menu disabled until replug |

Both sides retry forever on the existing reconcile shape, and a device
that leaves is `abandon()`ed so the way out does not warn about a restore
that could not have worked.

## What this does not change

- `anchord`, every PTY and every agent stay in WSL, untouched.
- `switch-client` stays in WSL. The bridge never learns a pane address;
  tally's tmux server ([24](24-the-tally-server.md)) is entirely
  WSL-side. **The bridge is not told which pane to focus** — that
  decision never leaves `tallyd`.
- macOS. `Remote` is reached only when `TALLY_BRIDGE_LISTEN` is set.
- The dependency budget. `tallybridge` is a third `[[bin]]` in this crate
  so the codec has one definition compiled into both sides, and it adds
  no dependency `tallyd` does not already have.

## For the session that picks this up

You are on the Windows box. This was specified on a Mac with no Windows
hardware in reach, so **everything marked ⚠️ above is a claim, not a
fact**, and the first milestone exists to turn five of them into one or
the other before any of the rest is built.

Read in this order: this document, then
[02](02-surface.md) for the device protocol, then
[08](08-lifecycle.md) for why the restore is not optional, then
`src/device.rs` and `Link` in `src/bin/tallyd.rs`, which are the only two
places the WSL side changes shape.

The build order, exit criteria and the probes for the ⚠️ rows are
[M17 in the roadmap](../ROADMAP.md#m17--the-bridge). **Do M17.0 first and
report the five answers before writing the codec** — two of them can
move the design, and one of them (the bind address) can put a socket on
the local network, which is a decision rather than a detail.

Environment, stated once:

- Rust for Windows is the **MSVC** toolchain
  (`rustup target add x86_64-pc-windows-msvc`), built natively on
  Windows, not cross-compiled from WSL. `midir` picks WinMM there with no
  feature flags.
- `tallyd`, `anchord`, `tmux` and every agent stay inside WSL. Nothing
  about them changes and nothing about them should be ported.
- The repo is one crate and builds on both sides. `cargo test` in WSL
  covers the codec and `Remote`; the Windows build covers `tallybridge`.
