# 13 — The bridge

**The Launchpad on a Windows desk, the agents in WSL.**

> ⚠️ **Status, 2026-10-20: built, tested without hardware, and not yet
> run against a Launchpad.** No Mini MK3 was plugged into the Windows
> desk while this was written. Four of the five questions that held it in
> [11](11-unbuilt.md) were answered by running something on that desk;
> the fifth is answered from the protocol and says so. Everything that
> needs the device is marked ⬜ and collected in
> [the checklist](#the-checklist-when-the-launchpad-is-plugged-in), which
> is twenty minutes with the cable.

USB does not cross the WSL line: the stock WSL2 kernel is built without
`CONFIG_SND_USB_AUDIO`, so `usbipd attach` hands over a device that never
appears under `/dev/snd`, and `midir`'s ALSA backend enumerates no port.
Building a kernel is the other answer and this document is not it.

So the process is cut in two. **`tallybridge` runs on Windows and owns the
port. `tallyd` runs in WSL and owns everything else.**

```text
   Windows                                │  WSL2 (mirrored networking)
                                          │
   Launchpad ──USB──▶ tallybridge ◀───────┼──TCP 127.0.0.1:7373── tallyd ──▶ tmux, anchord
                      (listens)           │                       (connects)
                                          │
                      powershell.exe ◀────┼───────────────────────┘  the raise, by interop
```

## The one rule

**When something could live on either side, it lives in `tallyd`.** The
bridge moves bytes. It holds no model, no chord state, no frame diffing
and no idea what a pad means; every sentence of [03](03-surface.md) and
[05](05-interaction.md) stays on the WSL side, where the tests are.

**The single exception is the Live-mode restore**, and it is the bridge's
own duty because the moment it is needed is the moment the WSL side can no
longer reach the device ([below](#restore-is-the-bridges-own-duty)).

## The five questions, answered

Each was an ⚠️ in the deleted design (`docs/30-the-bridge.md` in git,
before `17a5b23`). They were run on the desk itself — Windows 11 Pro build
26200, WSL 2.7.3.0, kernel 6.6.114.1, Ubuntu 24.04, `%USERPROFILE%\.wslconfig`
with `networkingMode=mirrored` and `firewall=true`, `/etc/wsl.conf` with
`systemd=true` and interop left at its default.

### 1. Does a `127.0.0.1` listener reach across? ✅ Both ways, and nowhere else

**Checked rather than assumed, 2026-10-19.** A Python listener bound to
`127.0.0.1:47301` inside WSL accepted a connection from PowerShell's
`TcpClient` on Windows; a .NET `TcpListener` on Windows'
`IPAddress.Loopback:47302` accepted one from Python in WSL. Both carried a
line of data.

The other half is the security answer. With the Windows listener up,
`netstat -ano` showed it as `127.0.0.1:47303 LISTENING` and nothing else,
and connecting to it on this machine's own LAN addresses was refused — on
all three (`192.168.1.156`, `192.168.50.2`, `100.81.57.75`) from Windows,
and on `192.168.1.156` from WSL. Mirrored networking shares the host's
addresses with the VM, so that is as close to *from the network* as one
machine gets.

**So the socket is loopback-only, and that is enforced, not configured.**
`tallybridge --listen` refuses any address that is not loopback, with no
flag to override it. NAT networking is not supported: there, WSL reaches
Windows only on a virtual interface's address, and a bridge listening
there is a bridge on a network.

What loopback still admits, named: **any process on this machine**,
either side of the line, can connect. It matters which side listens, and
the direction was chosen on this ([who listens](#who-listens)): a stranger
on the socket can light pads and read presses, and can hold the bridge
so `tallyd` cannot connect. It cannot reach `tallyd`, anchord or a
session, because `tallyd` accepts nothing. Anything that can run code as
you can already do worse — run `wsl.exe` and talk to anchord — so there
is no authentication on this one. Another account on the same Windows machine
could connect too; this is a one-person desk, and that is the assumption.

### 2. Does `AppActivate` raise the window? ❌ Not alone. ✅ With an Alt press first

**Checked, 2026-10-19**, against a real Windows Terminal window titled
`tally-probe`, with Chrome in the foreground, by calling `powershell.exe`
from inside WSL the way `tallyd` would, and reading
`GetForegroundWindow()` afterwards:

| from WSL | returned | foreground after |
|---|---|---|
| `(New-Object -ComObject WScript.Shell).AppActivate('tally-probe')` | `True` | Chrome — unchanged |
| the same, again | `True` | Chrome — unchanged |
| the same, aimed back at Chrome from the terminal | `True` | the terminal — unchanged |
| `SendKeys('%')` then `AppActivate` — [`bridge/raise.ps1`](../bridge/raise.ps1) | — | **the terminal** |
| `raise.ps1`, again | — | **the terminal** |
| `raise.ps1`, back to Chrome | — | the terminal — unchanged |
| `raise.ps1`, back to Chrome, retried | — | **Chrome** |

**`AppActivate` returns `True` when it did nothing**, so its result cannot
be the check. Windows' foreground lock refuses a process that did not
receive the last input — and a pad press is MIDI, not input Windows
counts. A synthetic Alt press makes `raise.ps1` the process that did,
which the lock honours: **three raises of four**. One raise measured
**426 ms** from outside WSL, most of it starting processes.

**A failed raise is still never an error**: tmux has already switched the
pane, so the move happened and only the window stayed behind. The hook was
already there — `TALLY_RAISE_CMD` ([12](12-install.md#the-environment-it-reads))
— so no tally code changed for this. The Alt lands on whatever had the
foreground; most programs ignore a bare tap, a few focus their menu bar.

### 3. Is interop on? ✅ Yes — but from a systemd unit it cannot reach the desktop

**Checked, 2026-10-19.** `powershell.exe -NoProfile -c "echo ok"` from a WSL
shell printed `ok`; `/etc/wsl.conf` does not disable interop. So the raise
stays a shell-out and needs no control frame on the wire.

**The finding the question did not ask for, and the one that decides how
`tallyd` is started.** A systemd user unit is the obvious way to run a
daemon in WSL, and from one, interop *runs* Windows programs without
letting them *show* anything. Run 2026-10-20 against a live Windows
Terminal window behind Chrome, the unit rows with
`systemd-run --user --wait --pipe`:

| started from | command | exit | result |
|---|---|---|---|
| a systemd unit | `raise.ps1` | 0 | **no raise, 3 of 3** |
| a systemd unit | `wt.exe -w new --title …` | 0 | **no window, 2 of 2** |
| a systemd unit, `--setenv=WSL_INTEROP=` a live `wsl.exe` session's socket | `raise.ps1` | 0 | raised, 2 of 2 |
| a `wsl.exe` shell, as a control | `raise.ps1` | 0 | raised, 2 of 2 |
| a `wsl.exe` shell | `wt.exe -w new --title …` | 0 | a window |

The third row is the explanation, as far as it was taken: a `wsl.exe`
that Windows started in your session carries that session's interop
socket, and what is launched through it reaches the desktop; a unit
gets WSL's own, and what is launched through that does not. Why the
second lands off the desktop was not dug into. Exit codes are 0
throughout, so nothing would ever have said so. **`tallyd` on this desk
therefore runs under a `wsl.exe` started at logon, beside the bridge, not
as a systemd unit** ([12](12-install.md#windows-with-wsl)) — handing a
unit a session's socket works, but only while that session lives.

The smaller one: a unit's `PATH` has no `/mnt/c` in it (`powershell.exe
not on PATH`, then the same binary by its full path ran fine), so every
Windows binary in tally's environment is written as an absolute
`/mnt/c/...` path, whichever way `tallyd` is started.

### 4. Does the device survive `taskkill /F` of the bridge? ❌ No, from the protocol

**Reasoned, not run** — it needs the device. `taskkill /F` is
`TerminateProcess`: no handler, no destructor, nothing of the bridge's
runs. The driver closes the port on the way down and sends the device
nothing; the device holds its mode in its own RAM and the protocol
documents no host watchdog. So **the device stays in programmer mode, with
the Setup menu disabled**, exactly as `SIGKILL` of `tallyd` leaves it on a
Mac ([08](08-daemon.md#shutdown-is-not-optional)).

The mitigation is the same as there — recover on start, because that is
the half that is reliable. **A bridge that starts finds the Launchpad,
reads its mode back, and puts it into Live mode if it is not**, before it
listens. So the recovery from a `taskkill /F` is starting the bridge again,
or a replug. ⬜ The checklist runs it.

What *is* covered: **Ctrl+C and closing the console window** run a handler
that ends the session, lets the restore run on the normal path, and only
then lets the process go — the same shape as `tallyd`'s signals, and
inside the five seconds Windows gives a console window that is closing. ⬜ Unverified
on hardware.

### 5. Does the clock cross the hop without stuttering? ✅ The hop adds under a millisecond

**Measured, 2026-10-19**, with a std-only probe compiled natively on both
sides: a WSL sender writing the wire's exact clock frame, `01 00 01 F8`,
at 24 PPQN to a Windows receiver that timestamps each arrival and replies
with nothing until the end, so no ACK can piggyback.

| | |
|---|---|
| round trip, WSL → Windows → WSL, 2000 pulses | **p50 212 µs, p99 362 µs, max 772 µs** |
| round trip inside WSL, for comparison | p50 376 µs, p99 880 µs, max 1761 µs |
| jitter the hop adds to each gap, 140 bpm, `TCP_NODELAY` on, 3000 pulses | **p50 0.15 ms, p99 0.37 ms, max 0.61 ms** |
| the same, tallyd's 16 ms tick | p50 0.28 ms, p99 0.44 ms, max 0.61 ms |
| connect to first byte | 0.7–1.3 ms |

`TCP_NODELAY` is on at both ends and is **required**: one of the two
tick-paced runs with it off held a pulse for a 59 ms gap, which is Nagle
waiting for an ACK the receiver had no reason to send. With it on, the
worst jitter the hop added in any final run was 0.61 ms, so **on this
evidence it is also sufficient**. The breath itself
is ⬜ — whether the device's ramp looks the same with the hop in it is a
thing to look at.

**Two things the measurement found that are not the bridge's**, recorded
because both move the numbers above and neither is a hop:

- **The loop already quantises the clock harder than the hop does.**
  `Beat::pulse` sends at most one pulse per 16 ms wake and schedules the
  next from *now*, so a 17.9 ms gap (140 bpm) leaves as 32 ms and a
  41.7 ms gap (60 bpm) as 48 ms: the device sees ~78 and ~52 bpm. The
  real `tallyd` at the 60 bpm floor delivered pulses 47.3 ms apart (p50,
  Windows' clock) across the line. That was `tallyd` on every platform,
  and it is **fixed on main since this was measured** — `edcafa3`
  schedules the next pulse from when it was due, so the emitted rate is
  the commanded one. The hop numbers above came from a std-only probe, not
  from `Beat`, so they stand; only the 47.3 ms figure is the old schedule.
- **WSL's monotonic clock ran ~8% fast against Windows' on this box.**
  Fitting arrival against departure over five runs gave a rate of
  0.917–0.921, and a 10 s sleep in WSL measured 7.37 s on WSL's own
  realtime clock, which was being stepped back to compensate. One-way
  timings across the line are fitted for rate before anything is read from
  them, and every `Duration` in `tallyd` is ~8% short in real time here.

## What the answers changed

Against the deleted design, four things moved, and all four were cheaper
than what they replaced.

**The bridge listens; `tallyd` connects.** The old reason for the other
direction — WSL forwards Windows' `localhost` into the VM and not the
reverse — is gone under mirrored networking (question 1). What decides it
now is the seam: `Link` already calls `Device::open` on the reconcile
clock and treats `NotFound` as an unplugged cable. A bridge that is not
running *is* a refused connect, which *is* `NotFound`, so the bridge
slots into a retry loop that exists rather than needing a listener held
across reconnects. And it puts the attack surface on the device, not on
the daemon ([question 1](#1-does-a-127001-listener-reach-across--both-ways-and-nowhere-else)).

**`Device` stays one type.** The old design made it a trait for a second
implementation, which is the move [01](01-scope.md#how-to-reject-a-feature)
forbids, and it was not needed: every line of `Device` — identity,
programmer mode, the diff, the probe, the restore — is about bytes, and
only the thing carrying them differs. `Device` holds a `Port`, a `midir`
connection or a socket; a reader thread turns frames into the same
`Event`s on the same channel the `midir` callback feeds. **`poll` kept its
signature, `Link` did not change, and nothing above `Link` can tell.**

**The raise needed a script, not a frame.** Interop is on (question 3),
so the reserved control frame stays unbuilt; `AppActivate` alone is
refused (question 2), so `raise.ps1` presses Alt first.

**The restore asks before it acts.** After a clean `tallyd` exit the device
is already in Live mode, and what re-sending Live mode to a device in Live
mode does has not been measured. A readback has.

## The seam

[`src/device.rs`](../src/device.rs) is the whole tally-side change, and
[`bridge/`](../bridge) is the whole Windows side.

| | owns | when |
|---|---|---|
| `Port::Midi` | a `midir` connection | `TALLY_BRIDGE` unset — every Mac, every Linux box with the device on it, behaviour unchanged |
| `Port::Bridge` | a socket to `tallybridge` | `TALLY_BRIDGE=127.0.0.1:7373` |

Two behaviours differ, both because a socket knows something `midir`
cannot ([D17](10-decisions.md#d17--a-failed-write-is-not-how-you-learn-the-device-went-away)):
`render` fails once the bridge has hung up, even for a frame with nothing
new, so `Link` drops the device on the next tick instead of after seven
seconds of failing clock pulses; and `Drop` does not raise the Setup-menu
alarm for a bridge that is already gone, because the restore is then the
bridge's. `Error::Bridge` is new, and `Link`'s existing catch-all logs it.

**The bridge is a second package in this repository, not a third
`[[bin]]`.** `tally`'s library does not build on Windows — `anchor.rs` is
a Unix socket and `lock.rs` is `flock` — and making it would be `cfg`
across half the crate for a binary that needs none of it. So
`bridge/` is a workspace member whose **library** is the wire and the
session, std-only, and `tally` depends on it: one codec, compiled into
both ends. The bridge's copies of four constants — Live mode, the
readback, stop-scroll and the port substring — cannot come from `tally`
for the same reason, and `tests/bridge.rs` asserts they are tally's bytes.

## The wire

```text
kind  len    payload
00    00 02  01 01               bridge → tallyd, first frame: hello v1, present
01    00 01  F8                  tallyd → bridge: one clock pulse
01    00 FB  F0 00 20 29 ...     tallyd → bridge: a full dark repaint
01    00 03  90 0B 7F            bridge → tallyd: a press
```

One byte of kind, a big-endian `u16` length, the payload. **One MIDI
message per frame, both ways**: `decode` matches a whole message, as
`midir` hands its callback one, and TCP would happily deliver two presses
in one read. The bridge never inspects a MIDI frame and never re-splits a
stream. The length type is the cap — 64 KiB, against a largest real
frame of 332 bytes. The details are in
[`bridge/src/wire.rs`](../bridge/src/wire.rs), where the tests are.

**Presence is a message.** The bridge's first frame on every connection is
a hello: *present*, *absent* (no port matched), or *refused* with a reason
(a port matched and would not open — on Windows usually another program
holding it). Over a socket there are two silences where a local port has
one, and they want the same grid and different logs: a refused connect is
*bridge down*, `absent` is *cable out*, and both are `NotFound` to `Link`.
`refused` is logged, because it is not ordinary.

Nothing else is special-cased. The identity handshake is `INQUIRY` down
the pipe and the reply back, exactly as on a local port, inside the same
`REPLY_TIMEOUT`. `DEVICE_SILENCE` and the ping that feeds it are unchanged
and now cover the whole chain: a bridge that is up but wedged looks like a
Launchpad that stopped answering, which is the right reading of it.

### Who listens

`tallybridge` on `127.0.0.1:7373` by default; `tallyd` connects when
`TALLY_BRIDGE` names that address. **One connection at a time**: a second
`tallyd` waits in the backlog, hears no hello inside two seconds, and logs
that rather than hanging. The single-instance lock in WSL means that is a
stranger, not a second daemon.

## Restore is the bridge's own duty

Programmer mode disables the Setup menu, and `Device`'s `Drop` exists so
every path out puts it back. Over the bridge that restore still runs — on
`SIGTERM`, `tallyd` sends the same four messages over the socket and reads
the mode back through it. **But if `tallyd` is killed, or WSL shuts down,
or the socket drops, the moment the restore is needed is the moment it
cannot be sent from there.**

So every session end — `tallyd` hung up, seven seconds of silence, a
device that refused a write, a socket that broke — is followed by the
bridge's own restore: read the mode back; if it is not Live, stop any
scroll, send Live mode, read it back again. **A write is not an
acknowledgement**, so the result is one of *already Live*, *restored* or
*unconfirmed*, and the last is logged in the words `tallyd` uses for the
same failure.

The dark repaint `tallyd` sends first is left out. It was measured on
hardware to make no difference to where the device ends up
([08](08-daemon.md#shutdown-is-not-optional)), and it would be the
bridge's only copy of a frame.

**The silence is what catches a WSL shutdown.** `tallyd` pings every 2 s
and pulses the clock in between, so a quiet socket is not an idle one. A
VM that disappears may never send the FIN, and seven seconds — the same
figure `tallyd` uses for the same job in the other direction — is how long
the bridge waits before treating it as gone. ⬜ A real `wsl --shutdown`
was not run: it would have taken down every agent on this box.

## Lifecycle

| state | tallyd | bridge | grid |
|---|---|---|---|
| both up, device present | renders | pipes | live |
| bridge not running | `NotFound` every 2 s, silent | — | dark |
| bridge up, Launchpad unplugged | `NotFound` every 2 s, silent | hello *absent*, logged once | dark |
| port held by another program | `bridge: …` logged every 2 s | hello *refused*, logged | dark |
| cable pulled mid-session | sees the hang-up and drops the device on the next tick — or, if WinMM takes writes to a removed port as CoreMIDI does, its own 7 s silence drops it ⬜ | the session ends with it; restore *unconfirmed* ⬜ | dark |
| `tallyd` stops cleanly | restores over the socket, hangs up | restore: *already Live* | Live mode |
| `tallyd` killed (`SIGKILL`) | — | sees the hang-up at once → **restores** | Live mode |
| WSL shuts down | — | seven seconds of silence → restores ⬜ | Live mode |
| `tallyd` restarts | reconnects; first frame is total | new session | repainted |
| bridge stopped with Ctrl+C | sees the hang-up, drops the device | ends the session, restores, exits ⬜ | Live mode |
| bridge killed (`taskkill /F`) | sees the hang-up, drops the device | — | ❌ programmer mode until the bridge restarts or a replug |

**The `SIGKILL` row was run across the real line**, 2026-10-20, with the
real `tallyd` in WSL against the real `tallybridge` session on Windows and
a fake Launchpad behind it: `tallyd` reported *Launchpad connected*,
painted, pinged and clocked; killed, the bridge saw the hang-up at once
and reported `Restored`. The clean-stop row was run the same way and
reported `AlreadyLive`, because `tallyd`'s own restore had crossed the
socket first.

## What this does not change

- **anchord, every PTY, tmux and every agent stay in WSL**, untouched.
  The bridge never learns a pane address and is never told which pane to
  focus.
- **macOS and Linux.** `Port::Bridge` is reached only when `TALLY_BRIDGE`
  is set.
- **The dependency budget.** `tallybridge` is ours and adds nothing
  `tally` did not already have: `midir`, and std.
- **[08](08-daemon.md#one-process-no-socket)'s rule.** This is a socket,
  and it is not a control socket: it carries the device's own bytes and
  nothing else, and nothing queries `tallyd` over it. It is the USB cable,
  extended across a VM boundary.

> **The bridge may not grow.** Every argument for putting one more thing
> on the Windows side is an argument for a second implementation of tally,
> and the second one has fewer tests. If something seems to need to live
> there, the seam is in the wrong place — move the seam, in a document,
> before moving the code.

## What was tested, and where

| | where | runs on |
|---|---|---|
| the codec: every kind round-trips, a frame split byte by byte reassembles, back-to-back frames stay two, a truncated frame is an error and does not block, an unknown kind is hung up on, an oversize payload is refused | `bridge/src/wire.rs` | Windows and WSL |
| the session on a real loopback socket with a fake device: hello then both ways, one message per frame, restore after a hang-up, *already Live* left alone, silence ends a session and still restores, a refused write ends one, a non-tallyd peer is hung up on | `bridge/src/session.rs` | Windows and WSL |
| the port choice, the loopback-only parser | `bridge/src/session.rs`, `bridge/src/main.rs` | Windows and WSL |
| **the real `Device` against the real session**, end to end: start-up bytes in order, a diff, two presses as two events, the restore on drop, every refusal, a hang-up noticed on the next frame, the copied constants | `tests/bridge.rs` | WSL (and any Unix) |
| the real `tallyd` against the real bridge across the WSL line, with no device: *absent* logged once, `tallyd` silent | by hand, 2026-10-20 | both |
| the real `tallyd` against the real session across the line with a fake Launchpad: clean stop and `SIGKILL` | by hand, 2026-10-20 | both |

## The checklist, when the Launchpad is plugged in

Twenty minutes, in order. Each line is one thing to do and one thing to
see; a ❌ on any of the first three stops the rest.

1. ⬜ **The port names.** `tallybridge --list` on Windows. Two lines should
   carry `<- used`, one `in` and one `out`, and they should be the
   Launchpad's **MIDI** interface, not its DAW one. Windows' class driver
   names a device's second interface `MIDIIN2 (…)` / `MIDIOUT2 (…)`; if
   both interfaces match the substring, the numbered one is chosen
   ([`session::pick`](../bridge/src/session.rs)). If nothing matches,
   find the right substring in the list and pass `--port`. If the wrong
   pair is chosen, that is a code change in `pick`, and this line is
   where it is found.
2. ⬜ **A pad lights from WSL.** Start both halves the way
   [12](12-install.md#6-start-both-halves-at-logon) says — the logon line,
   run by hand the first time. `tallyd` says *Launchpad connected*, the
   bridge names the two ports, and the grid shows what `tally ls` says,
   logo green. Then sign out and in once: both are running again.
3. ⬜ **A press reaches WSL.** Tap a lit pad: the jump happens. Hold a
   modifier and watch the preview — that is a chord crossing the hop.
4. ⬜ **The breath.** Look at a pulsing pad for a minute. It should look as
   it does on a Mac; a stutter is question 5 coming due, and the fix would
   be moving the clock to the bridge, the largest change this design has.
5. ⬜ **The raise.** Put another window in front, press a pad. The
   terminal comes forward most times (question 2 measured three of four).
   A miss is only the window: the pane has switched either way.
6. ⬜ **Clean stop.** Stop `tallyd` — `SIGTERM`, or Ctrl+C in its terminal.
   The bridge logs *Launchpad in Live mode (AlreadyLive)*. Hold Session:
   **the Setup menu opens.**
7. ⬜ **`tallyd` killed.** Start it, then `kill -9` it. The bridge logs
   `Restored`. **The Setup menu opens.** This is the row the restore duty
   exists for.
8. ⬜ **Cable out.** With both running, unplug the Launchpad. Within about
   seven seconds `tallyd` logs a disconnect and the bridge ends the
   session; note which of the two said it first.
   Plug it back in: the grid comes back by itself, total, with neither
   process restarted.
9. ⬜ **Bridge stopped with Ctrl+C.** In its console: it ends the session,
   logs the restore, and exits. **The Setup menu opens.**
10. ⬜ **Bridge killed hard.** `taskkill /F /IM tallybridge.exe` while a
    session is live. Expected ❌: the Setup menu is dead (question 4).
    Start the bridge again: it logs *found the Launchpad out of Live mode
    at start: Restored*, and **the Setup menu opens.**
11. ⬜ **A day beside it.** Not a formality: the hard amber flash and the
    green working pulse both passed a glance on real hardware and failed a
    day ([10](10-decisions.md)), and this adds a hop to the surface they
    were measured on. A working day on the Windows desk with no restart of
    either binary, and the breath still legible at the end of it.

Record the answers here, in place of the ⬜, the way the five above were
recorded: what was run, what was seen, the date.
