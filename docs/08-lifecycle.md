# 08 — Lifecycle

## One process, no socket

`tallyd` owns the MIDI port and the model built from `anchor list`, so
there is a daemon whether we want one or not.

**The OS will not enforce that ownership.** An earlier draft said the port
was exclusive; on macOS it is not — CoreMIDI multiplexes, two processes can
drive the device simultaneously, and neither is told
([02-surface.md](02-surface.md#the-port-is-not-exclusive-and-we-assumed-it-was)).
So `tallyd` takes a **single-instance lock** itself: an exclusive `flock` on
`$XDG_RUNTIME_DIR/tally.lock`, held for the process lifetime, acquired
*before* the port is opened. A second instance finds it held, says so, and
exits — rather than silently fighting the first one for 81 LEDs every two
seconds.

A lock file is not a control socket and the distinction matters: nothing is
read from it, nothing is written to it, and no protocol crosses it. It is a
mutex with a path.

What there is **not** is a control socket for it. The obvious design is
`tallyd` holding state and an `tally` CLI querying it over a second unix socket,
and that is one socket, one protocol, one framing and one set of error
codes more than this program needs. Everything `tally` would ask for is
already readable from somewhere else:

| `tally` wants | gets it from |
|---|---|
| what is on the grid | `anchor ls`, same as tallyd |
| slot specs | `~/.config/tally/slots.toml` |
| to rebind a slot | edit the file, or `anchor label` |
| to jump to a slot | runs the same tmux commands tallyd would |

So `tally` is a second binary over the same two sources of truth, and tallyd
notices config changes on its reconcile tick. **No IPC between our own two
binaries.** Reject the socket.

The rule generalises past our own binaries, and the test case is the
attention hook. A `Notification` hook needs to tell tallyd that an agent is
waiting, and the obvious shape is a FIFO — a path, a reader task, EOF and
reopen handling, a line format. It does not get one: it writes
`tl.attention` on the session, and tallyd reads it from the `list` it is
already making ([04-signals.md](04-signals.md#4-a-hook--it-told-us)).
**The thing that already has a socket gets to keep being the only one.**

The cost is a bounded staleness — up to one tick before the grid reflects
an edit — which is invisible at 2s and exactly the latency the rest of the
reconcile already has.

## Who starts it

A launchd user agent, declared in `~/dotfiles` and applied with provision
([07-seams.md](07-seams.md#provision)). Not a `service install` subcommand:
that exists in anchor because anchor is shipped to strangers.

`tallyd` must tolerate being started before anything else exists — no device,
no anchord, no tmux server. A missing Launchpad is not a startup failure:
the thing it watches is anchord, which it can do blind, and the device is
picked up on a later tick whenever it appears. It reconciles into a working state when they
appear, and says which one is missing while they do not.

## Shutdown is not optional

**Programmer mode disables the device's Setup menu**
([02-surface.md](02-surface.md#entering-and-leaving-programmer-mode)). A
process that exits without sending the Live-mode message leaves a Launchpad
that has lost a feature with no obvious way to get it back.

So shutdown must:

1. stop any running text scroll
2. clear the surface — **all 81 LEDs, the logo included**
3. send Live mode — `F0 00 20 29 02 0D 0E 00 F7`
4. **read the mode back** — `F0 00 20 29 02 0D 0E F7` — and log loudly if
   the reply is not `00`
5. flush, then close the port

✅ Verified on hardware, on SIGTERM, including the Setup menu working again
afterwards. Tested both with and without step 2, and **both leave the device
correct** — the device repaints its own Live-mode lighting either way, so
the handover is not fragile. Step 2 stays because it is the explicit option:
it leaves the surface in a state we chose rather than whatever our last
frame happened to be.

Step 2 says *81* for a reason. An implementation that clears the 8×8 grid
and the control rows but skips index 99 leaves the health light burning —
and a lit logo means **anchord unreachable**
([05](05-interaction.md#the-logo-is-the-health-light)). Exiting while
asserting a fault that is not happening is worse than exiting messily. This
is not hypothetical; the M0 harness had exactly that bug.

Step 4 is the one that is easy to skip and worth keeping. The restore is a
write with no acknowledgement, so a clean exit proves nothing about whether
the device received it; the readback costs ~1.9 ms
([02](02-surface.md#other-messages-worth-knowing)) and turns "we sent it"
into "it happened". It is how the restore above was confirmed at all.

and it must happen on **SIGTERM and SIGHUP**, not only on a clean exit —
then re-raise, so the process still looks killed to whatever was waiting on
it. This is precisely the discipline anchor's `attach` applies to terminal
raw mode, for precisely the same reason: *a terminal left raw outlives the
process that did it.* So does a Launchpad left in programmer mode.

`SIGINT` is handled too, because unlike anchor's attach there is no
foreground terminal here for `^C` to mean anything else.

### None of that happens in a signal handler

Four MIDI writes and a port close are nowhere near async-signal-safe, and a
handler that allocates is a handler that can deadlock on the allocator it
interrupted. So the signal does one thing: **it wakes the main loop.** A
`tokio::signal` stream raced against the reconcile tick, the shutdown
sequence running on the normal path with the port it already owns, and the
re-raise — `SIG_DFL` then `raise()` — as the last statement.

The same reasoning puts a **panic hook** on the list, and the docs should
not have called a panic unrecoverable without trying. A panic unwinds, so
an RAII guard owning the port restores Live mode on the way out for free;
`std::panic::set_hook` covers a panic on a thread that is not holding it.
Neither helps with `panic = "abort"`, which is a reason not to set it.

**It still cannot be made perfect.** `SIGKILL`, a double panic and a power
cut all leave the device in programmer mode. The mitigation is that
**start-up is idempotent and total**: tallyd re-enters programmer mode,
turns internal feedback off, and paints all 81 LEDs from the model, so
whatever state the previous process died in is overwritten rather than
inherited. Recovering on start is worth more than exiting cleanly, because
only one of the two is reliable.

## The reconcile loop

One loop, 2s, doing four things in order:

```text
1. device?    poll midir ports; (re)connect, enter programmer mode, repaint
2. anchord?   list all sessions, keep those with a tl.slot key, resolve
              each against slots.toml; on error light the logo red and
              leave the model alone
3. watchers   start one per live slot session that lacks one; reap the
              finished
4. render     model -> frame, diff against last, emit one SysEx
```

Step 2's error branch is the one to get right: **an unreachable anchord must
not empty the model.** Lighting the logo and rendering a stale grid is
honest — those sessions were there a moment ago and probably still are —
where clearing it asserts they are gone. Same distinction anchor drew for
teleport ([01-scope.md](01-scope.md#the-boundary-stated-plainly)).

Watchers outlive a failed tick. They are attached to anchord directly, so a
tick that could not `list` says nothing about whether a watcher is still
receiving bytes; tearing them down on a failed poll would turn one slow
response into a full re-attach storm.

Two seconds is teleport's reconcile cadence for the same job
(`anchor_watch.rs:31`). Presses are **not** on this loop — input is
event-driven off the MIDI callback and renders immediately. The loop is for
things that appear without telling us.

## Failure modes, and what each looks like

| what broke | what you see | what tallyd does |
|---|---|---|
| Launchpad unplugged | nothing (it is unplugged) | heartbeat goes unanswered, device dropped; anchord polling continues ✅ seen |
| Launchpad replugged | the whole grid returns by itself | reconnects, re-enters programmer mode, **repaints totally** ✅ seen |
| port held by another app | grid flickers between two opinions | nothing — CoreMIDI reports no conflict, so there is nothing to detect. Notice and quit the other app |
| a second `tallyd` started | it exits immediately | the `flock` is held; the running instance is undisturbed |
| anchord down | logo dark red, grid frozen | retries; grid is **not** cleared ✅ seen |
| anchord restarted | slots go dark red (`lost`) | truthful — anchor lost track of them ✅ seen |
| tmux server gone | jump recreates it | spawn a view window on demand |
| a watcher dropped | nothing | supervisor restarts it from `scanned_to` |
| tallyd itself dies | lights freeze, then stale | next start repaints from anchor's labels |

| config file edited | changes appear next tick | re-reads `slots.toml`; a file that fails to parse is logged and the **previous** config is kept |

### What a real anchord restart showed, and what it did not

A rebuild of anchord on 2026-10-16 bounced it. Measured from a polling
loop: the socket refused connections for **21 seconds** — a bootstrap that
failed and had to be retried — after which every running session came back
`lost`, reason `daemon_restart`.

✅ **Confirmed by eye: the `lost` rendering.** Three live pads became three
**dark red** pads. That settles the colour empirically — three *bright* red
pads would have read as three failures and sent someone to debug agents
that never failed. anchord knows it lost them; it does not know they
failed; the pad says only what anchord can support
([05](05-interaction.md#only-one-thing-flashes)).

✅ **The frozen grid, confirmed separately.** The restart above only ever
showed its *end state* to a human — nobody watched the device across the
outage, and an earlier revision of this file wrote the frozen grid up as
observed when it was inferred. So it was re-run under control: a **private
anchord** on its own socket (`XDG_DATA_HOME` pointed at a scratch
directory, the shared daemon untouched), killed deliberately and **held
dead for as long as it took to look**.

All three states were seen, in order:

| anchord | logo | pads | says |
|---|---|---|---|
| healthy | dark | **green** | these are alive |
| **killed** | **dark red** | **still green** | I cannot tell |
| restarted | dark | **dark red** | the daemon lost these |

The middle row is the one worth the trouble. `tallyd` knew its watchers had
died and its session list was stale, and it kept showing the last truth it
had — because a blank grid means *"nothing is running"* and the honest
answer was *"I cannot tell"*. Both halves matter and were checked
separately: green pads alone would prove only that it does not lie by
clearing; the dark red logo is what says the staleness is known.

Reproducing this needs no outage of anything shared:

```sh
export XDG_DATA_HOME=/tmp/tl-test     # short: a socket path is capped at 103 bytes
anchord &                             # a private daemon on its own socket
tallyd &                              # finds it by the same XDG rule
anchor create --label tl.slot=1:2:2 -- sleep 3600
kill -9 %1                            # and leave it dead
```

The row that matters is the last one. **tallyd holds no state that cannot be
rebuilt from `anchor list` plus a config file**
([03-model.md](03-model.md#where-the-binding-lives)). That is what makes it
disposable, and being disposable is the whole reason it is allowed to be
the most fragile thing in the stack.
