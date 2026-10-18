# 11 — Specified, not built

Two specs are written and deliberately held. Everything in `01`–`09` is
shipped and running; nothing in this file is.

Each is held for a stated reason rather than for lack of time, and each says
what would unblock it.

---

## The level

**A veto over a stale amber, never a source of one.**

Held on one thing: [the week of `attention.jsonl`](09-testing.md#the-honest-gap-today).
A veto changes what gets recorded, and measuring the architecture the hooks
actually specified is worth more than measuring it already patched.

### Why it is worth building at all

Attention is on hooks, on both edges, because the agent never rings
([D5](10-decisions.md#d5--the-bell-that-is-never-rung)). That is right and
is not reopened here.

But **a stuck pad is not a tuning problem; it is the whole product failing
in the one direction it cannot afford** — the grid saying *answer me* about
an agent that is working. The falling-edge hooks make that unlikely. They do
not make it impossible: a hook that never fires because the agent was killed
mid-turn, a `settings.json` that drifts, a wrapper that is not Claude Code
at all.

**An edge that is missed is lost forever. A level needs no falling edge.**

### What was measured

Claude Code **v2.1.236**, 2026-10-17. `claude agents --json` is documented,
needs no TTY, and returns every live session at once.

```json
{ "pid": 75590, "cwd": "…/spike3", "kind": "interactive",
  "sessionId": "915b110e-…", "name": "spike3-92",
  "status": "waiting", "waitingFor": "permission prompt" }
```

| session is | `status` | `waitingFor` |
|---|---|---|
| generating | `busy` | — |
| blocked on a tool permission dialog | `waiting` | `permission prompt` |
| blocked on a plan approval | `waiting` | `permission prompt` |
| blocked on an `AskUserQuestion` | `waiting` | `input needed` |
| at a prompt, turn finished | `idle` | — |
| at a prompt, nothing ever sent | `idle` | — |
| **sitting on the startup trust dialog** | **absent from the array entirely** | — |

One call costs **≈550 ms** and covers every session, so this is one poll for
the whole grid rather than one per pad. A submit shows as `busy` within a
single poll, and `waiting` landed **1.15 s** after the `Enter` that led to
it.

**`waiting` is held, not emitted.** It stayed true for as long as the dialog
was up and went back to `idle` the moment it was cancelled. That one
sentence is the entire argument.

### The decision

**Hooks stay the source of attention. The status may clear a false positive
and nothing else.**

```text
tl.attention set  AND  status is busy    ->  the label is stale, drop the amber
tl.attention set  AND  status is waiting ->  amber, and it is corroborated
tl.attention set  AND  status is idle    ->  amber, unchanged — this is the Stop case
tl.attention unset                       ->  nothing, whatever status says
```

**One direction only.** A veto cannot invent an amber, cannot miss one, and
cannot disagree with a hook about a pad that is dark. The only thing it can
do is end a stuck one.

**Why this is not the floor that was rejected.** Output-clearing was thrown
out because *"a signal with two writers disagreeing twice a second is worse
than one writer that can be absent."* **The objection was two writers, and
this adds none.** The hook is still the only thing that writes
`tl.attention`; `claude agents --json` is a second *reader*, of a different
fact, consulted when the label is already set. The race that killed
output-clearing cannot occur, because there is nothing to race with.

And it fails the right way: if the status is unavailable — `claude` missing,
command slow, a session that is not Claude Code at all — the veto does not
fire and the amber stands. **Losing it costs exactly what we have today.**

**It does not replace `Stop`, and that is the headline.** A finished turn
reports `idle`, and so does a session that has never been given anything —
the two are the same string. `claude agents --json` covers the dialogs,
which the config mostly auto-answers, and is **silent about the case the
amber was built for**. Anyone reading this as *the signal arrived, delete
the hooks* has it backwards.

### What it costs

**A second reader**, and [08](08-daemon.md#one-process-no-socket) has a rule
about that: *the thing that already has a socket gets to keep being the only
one.* The squad spec held that line by saying only a squad would ever read
`claude agents` and `tallyd` never would. **That line does not survive this
document**, and saying so is the point of writing it down rather than
quietly widening the rule.

What it survives *as*: the roster is still `anchor ls` alone, the clock is
still the reconcile tick, and the new read answers exactly one question —
*is this agent working right now* — about a pad that is **already amber**.
It is not a source of slots, of liveness, or of identity.

**It cannot be a source of liveness**, and the trust-dialog row is the
proof: a session sitting on *do you trust this folder* is **not in the array
at all**. A session spawned into a fresh directory would be live, visible on
the grid, and invisible to this command. anchor knows it exists. Nothing
else does.

**The cadence is not the reconcile tick.** 550 ms per call against a 2 s
tick is a quarter of the duty cycle, forever, to fix a pad that has been
wrong for minutes. A veto every ten seconds corrects a stuck amber long
before a human walks back to the desk, at a twentieth of the cost. **The
failure is slow, so the check can be.**

### Open questions

1. **Is the vocabulary bigger than three values?** `permission prompt`,
   `input needed`, and absent-entirely are what one afternoon produced.
   Nothing documents the set, so the veto must treat *unknown* as *not
   busy* — the safe direction, and it should be stated in code.
2. **Does `busy` ever lie while an agent waits?** The veto's one job is
   trusting `busy` to mean *working*. Measured it is clean, but a long tool
   call that is secretly blocked on something would read `busy` and suppress
   a legitimate amber. That is the one way this can do harm.
3. **Who else is on the grid?** `isayes`, `codex` and bare shells have no
   `claude agents` row. The veto never fires for them, which is correct —
   but it means a stuck pad has no guardrail at all on anything that is not
   Claude Code.

---

## The bridge

**The Launchpad on a Windows desk, the agents in WSL.**

**Specified, unbuilt, and five of its facts have never been run.** It was
written on a Mac with no Windows hardware in reach. Everything marked ⚠️
below is a claim, not a fact, and **the first step is to answer the five
before writing any codec** — two of them can move the design, and one can
put a socket on the local network, which is a decision rather than a detail.

### Why a bridge at all

USB does not cross that line: the stock WSL2 kernel is built without
`CONFIG_SND_USB_AUDIO`, so `usbipd attach` hands over a device that never
appears under `/dev/snd` and `midir`'s ALSA backend enumerates no port at
all. Building a kernel is the other answer, and this is not it.

```text
   Windows                            │  WSL2
                                      │
   Launchpad ──USB──▶ tallybridge ────┼──TCP──▶ tallyd ──▶ tmux ──▶ anchord
                                      │           │
                      powershell.exe ◀┼───────────┘  raise, by interop
                      (raise the window)
```

### What the bridge is not

It holds no model, no chord state, no frame diffing, no `Button` decoding
and no idea what a pad means. Every sentence of [03](03-surface.md) stays on
the WSL side, where its tests are. **The bridge moves bytes.**

That is the design rule, and it decides every question below: **when
something could live on either side, it lives in `tallyd`.** There is one
deliberate exception, argued in [Restore](#restore-is-the-bridges-own-duty).

### The seam already exists

`src/device.rs` is the line. `Device` finds the port, sends SysEx, decodes
presses and restores Live mode on drop; `Link` owns one and reconnects it on
the reconcile clock. Nothing above `Link` knows a MIDI port exists.

| | owns | where |
|---|---|---|
| `device::Local` | a `midir` connection | today's `Device`, behaviour unchanged |
| `device::Remote` | a socket to the bridge | new |

`Device` becomes a **trait** with the method set `Link` already calls.
`open` stays an inherent associated function on each impl, because it
returns `Self`.

**One signature has to change.** `poll` returns `impl Iterator + '_`, which
is not object-safe, so it becomes `fn poll(&self) -> Vec<Event>`. The single
caller already collects into a `Vec`, so this is a move, not a cost.

`Link::connect` calls a factory that reads the environment once:
`TALLY_BRIDGE_LISTEN` set → `Remote`, unset → `Local`. **A Mac with a
Launchpad on it never reaches the new path.**

### The wire

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

Kinds `0x01` and `0x02` carry raw MIDI and the bridge never inspects them.
Kind `0x11` carries one JSON object, which is why `serde_json` — already a
dependency — is all the codec needs. `0x10`, tallyd → bridge control, is
reserved and currently unused.

**One MIDI message per frame.** The bridge must not coalesce: `decode`
matches on a whole message and `midir` hands its callback exactly one, so if
two presses shared a frame because TCP delivered them together, `decode`
would return `None` and both would vanish. **The frame boundary is the
message boundary**, in both directions.

**`TCP_NODELAY`, on both ends.** Not an optimisation. The tempo is a stream
of single-byte `0xF8` clock pulses at 24 PPQN, and Nagle's algorithm exists
to hold small writes back until the previous one is acknowledged. Left on,
the clock arrives in bursts and the device's two-beat breath stutters.

**`tallyd` listens; the bridge connects.** `TALLY_BRIDGE_LISTEN`, suggested
`127.0.0.1:7373`; the bridge takes `--connect host:port`. This direction
because WSL2 forwards Windows `localhost` to a listener inside WSL with no
configuration: no firewall rule, no reading the host address out of
`/etc/resolv.conf`, same behaviour under NAT and mirrored networking.
Connecting the other way needs all three.

> ⚠️ **Unverified.** Whether that forwarding reaches a listener bound to
> `127.0.0.1` or needs `0.0.0.0` has changed across WSL releases.
> `TALLY_BRIDGE_LISTEN` takes a full address so that is a config line and
> not a patch. **If `0.0.0.0` turns out to be required, the socket is on the
> local network and this document must say so in plain words** — there is no
> authentication on it, and a frame on it drives the grid.

**Presence is a message, not a guess.** Over a socket there are two
different silences — the bridge is down, or the bridge is up and the
Launchpad is unplugged. They want the same grid and different logs, so the
bridge sends `{"device":…}` on connect and on every change.

The identity handshake is **not** special-cased: `Remote` sends the inquiry
down the pipe and waits for the reply exactly as the local path does, and
the 2 s reply timeout stays — it was set generous for a ~345 ms cold round
trip, and a loopback hop is noise beside that. The 7 s silence threshold and
the `ping` that feeds it are unchanged and now cover the whole chain. **A
bridge that is up but wedged looks exactly like a Launchpad that stopped
answering, which is the correct reading of it.**

### Raise does not need the bridge

The first draft had a `{"verb":"raise"}` control frame, because `raise()` is
a hardcoded macOS `open -a` and the window to raise is a Windows window.
That was wrong, and the reason is worth keeping: **WSL runs Windows
binaries.** Interop is on by default, so `tallyd` can invoke
`powershell.exe` directly — the same shape as the macOS path, a best-effort
shell-out to the one genuinely machine-specific string in the file. No
socket handle threaded into `jump.rs`, no control verb, no coupling between
the window manager and the MIDI pipe.

| env | default | on this desk |
|---|---|---|
| `TALLY_RAISE_CMD` | `open -a $TALLY_TERMINAL` (macOS, as today) — **exists already** | a `powershell.exe` `AppActivate` against the window title |
| `TALLY_TERMINAL_CMD` | `open -na …` (exists already) | `wt.exe new-tab wsl.exe …` |

`TALLY_RAISE_CMD` arrived ahead of this milestone and not for this reason:
the suite needed somewhere to send the raise, because unsetting
`TALLY_TERMINAL` does not disable it (`terminal_app` falls back to
Alacritty) and every integration jump was taking the screen of whoever ran
`cargo test`. The shape is the one specified here, so the bridge inherits
it rather than adding it.

Both are **split on spaces and are not a shell**. A command needing quoting
goes in a `.ps1` or a `.cmd` and the env var names that file. The window
title is `tally`, because the terminal command already passes `--title
tally` — that is the string `AppActivate` matches.

> ⚠️ **Unverified, and two ways it can fail.** Windows restricts foreground
> stealing, so `AppActivate` may refuse. Interop can be turned off in
> `/etc/wsl.conf`, in which case `powershell.exe` is not executable from WSL
> and the reserved `0x10` control frame becomes the fallback. **A failed
> raise is never an error** — tmux has already switched the pane, so the
> move has happened and only the window did not come forward.

### Restore is the bridge's own duty

The one place the design rule is broken, on purpose.

Programmer mode disables the device's Setup menu, and every path out must
put it back ([08](08-daemon.md#shutdown-is-not-optional)). If `tallyd` dies,
or the socket drops, or WSL shuts down, the WSL side can no longer reach the
device — **the moment the restore is needed is the moment it cannot be sent
from there.**

So the bridge restores Live mode itself, on client disconnect and on its own
exit: stop scroll, a dark repaint, Live, then a mode readback, because a
write is not an acknowledgement. Those constants are duplicated onto the
Windows side with this paragraph as the reason. They are fixed bytes out of
Novation's manual and they do not move.

### Lifecycle

| state | tallyd | bridge | grid |
|---|---|---|---|
| both up, device present | renders | pipes | live |
| bridge not started | listens, logs once | — | dark |
| bridge up, Launchpad unplugged | retries on the reconcile clock | sends `gone`, polls the port | dark |
| socket drops | drops the device, keeps listening | restores Live, retries `--connect` | dark, Setup menu intact |
| bridge wedged, socket open | silence threshold fires, drops it | — | dark |
| tallyd restarts | relistens; first frame is a full repaint | reconnects | repainted from scratch |
| bridge killed hard (`taskkill /F`) | sees EOF, drops the device | — | ⚠️ Setup menu disabled until replug |

Both sides retry forever on the existing reconcile shape, and a device that
leaves is abandoned so the way out does not warn about a restore that could
not have worked.

### What this does not change

- `anchord`, every PTY and every agent stay in WSL, untouched.
- `switch-client` stays in WSL. The bridge never learns a pane address;
  tally's tmux server is entirely WSL-side. **The bridge is not told which
  pane to focus** — that decision never leaves `tallyd`.
- macOS. `Remote` is reached only when `TALLY_BRIDGE_LISTEN` is set.
- The dependency budget. `tallybridge` is a third `[[bin]]` in this crate so
  the codec has one definition compiled into both sides, and it adds no
  dependency `tallyd` does not already have.

### For the session that picks this up

You are on the Windows box. Read in this order: this section, then
[03](03-surface.md) for the device protocol, then
[08](08-daemon.md#shutdown-is-not-optional) for why the restore is not
optional, then `src/device.rs` and `Link` in `src/bin/tallyd.rs` — the only
two places the WSL side changes shape.

**Answer the five ⚠️ probes and report before writing the codec.**

Environment, stated once:

- Rust for Windows is the **MSVC** toolchain, built natively on Windows, not
  cross-compiled from WSL. `midir` picks WinMM there with no feature flags.
- `tallyd`, `anchord`, `tmux` and every agent stay inside WSL. Nothing about
  them changes and nothing about them should be ported.
- The repo is one crate and builds on both sides. `cargo test` in WSL covers
  the codec and `Remote`; the Windows build covers `tallybridge`.

---

## Ideas with no document

Kept here so they are not reinvented, each one sentence and its blocker.

| | |
|---|---|
| **peek** — *what is this pad?* | A real hole: the surface cannot name a pad, and some share of 341 daily jumps are identification rather than work. The only candidate that needs machinery the surface does not have — a sink to print into. Deferred, not dead; holding Stop answers part of it for free |
| **solo** | Closed in favour of Move — *arrangement beats state*. Comes back with evidence if a week of Move shows people arranging and still being dragged away |
| **global brightness / LED sleep** | Documented, unspent, orthogonal. The wrong axis to lean on, because the colour language is built on *bright means where you are*. A separate and smaller idea if ever specced |
| **teleport URL as a secondary jump target** | Five lines once the session id is in hand. Not the default: clicking a pad on the desk should not raise a browser |
| **a second device** | Only with a second device in the room. Generalising the surface layer before a second implementation exists is guessing at the shape of the abstraction |
| **an *acting* sweep for drifted views** | [#22](https://github.com/alehatsman/tally/issues/22). The counting sweep ships ([06](06-views.md#the-sweep-counts-and-does-not-touch)); the one that repairs is held on its own number, for the same reason the level is held. Blast radius: every `tallyd` restart rearranging panes on a live grid, at the moment the daemon knows least |
| **resolving a squatter to its own address** | Same issue. Needs the page table inside `jump`, which points a dependency the wrong way. Parking gets the pad working without it |
