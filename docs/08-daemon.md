# 08 — The daemon

`tallyd` owns the MIDI port and the model built from `anchor list`. That is
the whole of it.

## One process, no socket

There is a daemon whether we want one or not — something has to hold the
device and the state. **The OS will not enforce that ownership**: on macOS
CoreMIDI multiplexes, two processes can drive the device simultaneously, and
neither is told ([03](03-surface.md#the-port-is-not-exclusive-and-we-assumed-it-was)).

So `tallyd` takes a **single-instance lock** itself: an exclusive `flock` on
`$XDG_RUNTIME_DIR/tally.lock`, held for the process lifetime, acquired
*before* the port is opened. A second instance finds it held, says so, and
exits — rather than silently fighting the first one for 81 LEDs every two
seconds.

A lock file is not a control socket and the distinction matters: nothing is
read from it, nothing is written to it, and no protocol crosses it. **It is
a mutex with a path.**

What there is **not** is a control socket. The obvious design is `tallyd`
holding state and a `tally` CLI querying it over a second unix socket, and
that is one socket, one protocol, one framing and one set of error codes
more than this program needs. Everything `tally` would ask for is already
readable from somewhere else:

| `tally` wants | gets it from |
|---|---|
| what is on the grid | `anchor ls`, same as tallyd |
| slot specs | `~/.config/tally/slots.toml` |
| to rebind a slot | edit the file, or `anchor label` |
| to jump to a slot | runs the same tmux commands tallyd would |

So `tally` is a second binary over the same two sources of truth, and tallyd
notices config changes on its reconcile tick. **No IPC between our own two
binaries.**

> **The bridge is a socket between two of our binaries, and not an
> exception to this.** On a Windows desk `tallyd` talks to `tallybridge`
> over loopback TCP ([13](13-bridge.md)) — but what crosses it is the
> Launchpad's own MIDI, one message per frame, and nothing about the
> grid. Nothing asks `tallyd` a question over it and `tallyd` accepts no
> connection on it. It is the USB cable, extended across a VM boundary.

The rule generalises past our own binaries, and the test case is the
attention hook. A `Notification` hook needs to tell tallyd that an agent is
waiting, and the obvious shape is a FIFO — a path, a reader task, EOF and
reopen handling, a line format. It does not get one: it writes
`tl.attention` on the session, and tallyd reads it from the `list` it is
already making. **The thing that already has a socket gets to keep being the
only one.**

The cost is a bounded staleness — up to one tick before the grid reflects an
edit — which is invisible at 2 s and exactly the latency the rest of the
reconcile already has.

> One deliberate widening is on the table and not built: a `claude agents
> --json` read as a **veto** over a stale amber ([11](11-unbuilt.md#the-level)).
> The roster is still `anchor ls` alone and the clock is still the reconcile
> tick; the new read would answer exactly one question about a pad that is
> *already* amber.

## Who starts it

A launchd user agent, declared in `~/dotfiles` and applied with provision.
Not a `service install` subcommand: that exists in anchor because anchor is
shipped to strangers.

`tallyd` must tolerate being started before anything else exists — no
device, no anchord, no tmux server. A missing Launchpad is not a startup
failure: the thing it watches is anchord, which it can do blind, and the
device is picked up on a later tick whenever it appears. It reconciles into
a working state when they appear, and says which one is missing while they
do not.

## The reconcile loop

One loop, 16 ms, doing four things in order — but only two of them on
every wake:

```text
1. device?    poll midir ports; (re)connect, enter programmer mode, repaint
                                                           (every wake)
2. anchord?   list all sessions; on error light the logo red and leave the
              model alone. With it: which rows anchord would forget, and
              which panes a tmux client is looking at -- everything
              asked of another process, asked on one clock
                                                           (every 2 s)
3. watchers   start one per live slot session that lacks one; reap the
              finished
                                                           (every 2 s)
4. resolve    the sessions from step 2 against slots.toml and the signal
              map, then model -> frame, diff against last, emit one SysEx
                                                           (every wake)
```

Step 4 is on the fast clock and the other half of step 2 is not, and that
split is the whole shape: **resolving is a function of `now`**, so amber,
the journal's rising edges and the tempo all move inside a tick. *Listing*
is a question for another process, and asking it 62 times a second to hear
the same answer 124 times out of 125 is a fork storm, not freshness
(review w8 F2). A press that changes the world — a tap, `►`, `◄` — pulls
the reconcile forward to the next wake rather than waiting out the 2 s.

Step 2's error branch is the one to get right: **an unreachable anchord must
not empty the model.** Lighting the logo and rendering a stale grid is
honest — those sessions were there a moment ago and probably still are —
where clearing it asserts they are gone.

**The first failed `list` is a different failure**, and only the journal
feels it. A stale grid is last tick's truth; at login there is no last
tick, so the sessions the daemon holds are empty because nobody has been
asked. Step 4 would read that as every replayed slot having been answered
and write a fall per slot — `after_ms` spanning the downtime, the queue
cleared, and a second rise for each the moment anchord comes up, which is
[#7](https://github.com/alehatsman/tally/issues/7) by another road. So the
journal is **fall-blind until the first `list` succeeds**
(`journal::Journal::saw_listing`). Later failures are not guarded: by then
the map it resolves is the stale truth rather than an empty lie.

**Watchers outlive a failed tick.** They are attached to anchord directly,
so a tick that could not `list` says nothing about whether a watcher is
still receiving bytes; tearing them down on a failed poll would turn one
slow response into a full re-attach storm.

Two seconds is teleport's reconcile cadence for the same job. **Presses are
not on this loop** — input is event-driven off the MIDI callback and renders
immediately. The loop is for things that appear without telling us. It wakes
every 16 ms; the reconcile fires every 2 s, the art advances every other
wake, and the two timed gestures read `elapsed` on the line that feeds the
chord state machine.

### What the loop holds

The loop body was 425 lines holding **23 `let mut`** in one scope, with five
helpers taking between four and seven of those locals back by `&mut`. That
is a struct, passed one field at a time. The cost was not aesthetic: the
loop body was not callable, so it had 7 tests against `model`'s 50 — and
this project has twice paid for bugs of exactly one shape, *two correct
modules run in the wrong order*
([D9](10-decisions.md#d9--two-correct-modules-in-the-wrong-order)), which is
the shape a 425-line scope of loose mutable state is built to produce and
unable to test.

Four names, not one struct. A `Daemon` with 23 fields would make every
helper take `&mut self` and say nothing about which half of the state it
touches.

| name | holds | the invariant it carries |
|---|---|---|
| `Link` | `device`, `last_try`, `last_reply`, `repaint` | a device is **present or absent**, and absence is a normal state; the next frame after any (re)connect is **total** |
| `Beat` | `bpm`, `next`, `started` | pulses are sent when due and **dropped when missed**, never caught up |
| `Waiting` | `head`, `queued` | both are **last tick's**, because they are what the finger was looking at when it landed |
| `Grip` | `corner`, `hand`, `home` | the timed and multi-press gestures, which live here precisely because `chord.rs` is pure and has no `now` |

`Link` earns the most. A device leaves by exactly **one path** — a failed
write and seven seconds of silence both reach it — and that path guarantees
the two things the failure table promises: the next frame on any device is
total, and the chord state is cleared, so the release that went out with the
cable cannot leave a modifier held and turn the first tap after a replug
into a kill. `repaint` goes **inside** `Link` and not beside it: it is not a
rendering preference, it is the answer to "has this device seen a frame from
this process", and the only correct place to set it is wherever the device
arrives or departs.

`Grip` takes two methods and not one, and the split is the thing to notice:
`aim` handles `▲` **before** the art is offered the press, `feel` handles
`CC19` **after**. That ordering is not tidiness — it is what the art
swallows and what it does not. A held `▲` scrolls the window, the window is
not what an art is covering, and the grid underneath is where you want to
land when you leave it. Collapsing the two into one method moves `▲` to the
far side of the art and silently changes what the surface does. It was
caught by reading the code, not by a test.

What deliberately **stays a local**: `offset`, `sessions`, `reachable`,
`grid`, `purgeable`, `chords`, `journal`, `reel`, `config`, `tmux`,
`anchor`, `watchers`. These are owned-once-and-read, or already have a type
that names them. Moving them into a struct would be motion, not structure.

## Shutdown is not optional

**Programmer mode disables the device's Setup menu.** A process that exits
without sending the Live-mode message leaves a Launchpad that has lost a
feature with no obvious way to get it back.

So shutdown must:

1. stop any running text scroll
2. clear the surface — **all 81 LEDs, the logo included**
3. send Live mode — `F0 00 20 29 02 0D 0E 00 F7`
4. **read the mode back** and log loudly if the reply is not `00`
5. flush, then close the port

Verified on hardware, on SIGTERM, including the Setup menu working again
afterwards. Tested both with and without step 2, and **both leave the device
correct** — the device repaints its own Live-mode lighting either way. Step
2 stays because it is the explicit option: it leaves the surface in a state
we chose rather than whatever our last frame happened to be.

**Step 2 says *81* for a reason.** An implementation that clears the 8×8
grid and the control rows but skips index 99 leaves the health light
burning, and the health light is a claim about a process that is in the
middle of not existing — green asserting *the stack is up*, or dark red
asserting a fault nobody is watching for any more. **Dark is what says
tallyd is not running** ([05](05-interaction.md#the-logo-is-the-health-light)),
so index 99 is the one LED whose clearing carries information. This is not
hypothetical; the first harness had exactly that bug.

**Step 4 is the one that is easy to skip and worth keeping.** The restore is
a write with no acknowledgement, so a clean exit proves nothing about
whether the device received it. The readback costs ~1.9 ms and turns "we
sent it" into "it happened". It is how the restore was confirmed at all.

It must happen on **SIGTERM and SIGHUP**, not only on a clean exit — then
re-raise, so the process still looks killed to whatever was waiting on it.
This is precisely the discipline anchor's `attach` applies to terminal raw
mode, for precisely the same reason: *a terminal left raw outlives the
process that did it.* So does a Launchpad left in programmer mode. `SIGINT`
is handled too, because unlike anchor's attach there is no foreground
terminal here for `^C` to mean anything else.

### None of that happens in a signal handler

Four MIDI writes and a port close are nowhere near async-signal-safe, and a
handler that allocates is a handler that can deadlock on the allocator it
interrupted. So the signal does one thing: **it wakes the main loop.** The
handler is a relaxed atomic store and nothing else; the loop reads that
store at the top of every 16 ms tick and breaks out of it, the shutdown
sequence runs on the normal path with the port it already owns, and the
re-raise — `SIG_DFL` then `raise()` — is the last statement.

The same reasoning puts a **panic hook** on the list. A panic unwinds, so an
RAII guard owning the port restores Live mode on the way out for free;
`std::panic::set_hook` covers a panic on a thread that is not holding it.
Neither helps with `panic = "abort"`, which is a reason not to set it.

**It still cannot be made perfect.** `SIGKILL`, a double panic and a power
cut all leave the device in programmer mode. The mitigation is that
**start-up is idempotent and total**: tallyd re-enters programmer mode,
turns internal feedback off, and paints all 81 LEDs from the model, so
whatever state the previous process died in is overwritten rather than
inherited. **Recovering on start is worth more than exiting cleanly, because
only one of the two is reliable** — and it was proved by the clean exit that
fixed it belonging to a *different process* from the one that broke it.

## Failure modes

| what broke | what you see | what tallyd does |
|---|---|---|
| Launchpad unplugged | nothing (it is unplugged) | heartbeat goes unanswered, device dropped; anchord polling continues ✅ |
| Launchpad replugged | the whole grid returns by itself | reconnects, re-enters programmer mode, **repaints totally** ✅ |
| port held by another app | grid flickers between two opinions | nothing — CoreMIDI reports no conflict, so there is nothing to detect. Notice and quit the other app |
| a second `tallyd` started | it exits immediately | the `flock` is held; the running instance is undisturbed |
| anchord down | logo dark red, grid frozen | retries; grid is **not** cleared ✅ |
| anchord restarted | slots go dark red (`lost`) | truthful — anchor lost track of them ✅ |
| tmux server gone | jump recreates it | spawns a view on demand |
| a watcher dropped | nothing | supervisor restarts it from `scanned_to` |
| config file edited | changes appear next tick | re-reads `slots.toml`; a file that fails to parse is logged and the **previous** config is kept |
| tallyd itself dies | lights freeze, then stale | next start repaints from anchor's labels ✅ |
| tallyd wedges | the grid **stops breathing** mid-ramp | nothing — and that is the indicator ([05](05-interaction.md#the-tempo-says-how-long)) |

### What a real anchord restart showed

A rebuild of anchord bounced it. Measured from a polling loop: the socket
refused connections for **21 seconds**, after which every running session
came back `lost`, reason `daemon_restart`.

**The `lost` rendering, confirmed by eye.** Three live pads became three
**dark red** pads. That settles the colour empirically — three *bright* red
pads would have read as three failures and sent someone to debug agents that
never failed.

**The frozen grid, confirmed separately.** The restart above only ever
showed its *end state* to a human; nobody watched the device across the
outage, and an earlier revision wrote the frozen grid up as observed when it
was inferred. So it was re-run under control, on a **private anchord** on
its own socket, killed deliberately and held dead for as long as it took to
look:

| anchord | logo | pads | says |
|---|---|---|---|
| healthy | dark¹ | **green** | these are alive |
| **killed** | **dark red** | **still green** | I cannot tell |
| restarted | dark¹ | **dark red** | the daemon lost these |

¹ As observed. These runs predate the health light gaining a lit idle; a
healthy anchord now reads **green**, not dark
([05](05-interaction.md#the-logo-is-the-health-light)). The dark red rows —
the ones this test existed to prove — are unchanged.

The middle row is the one worth the trouble. `tallyd` knew its watchers had
died and its session list was stale, and it kept showing the last truth it
had — because a blank grid means *"nothing is running"* and the honest
answer was *"I cannot tell"*. Both halves matter and were checked
separately: green pads alone would prove only that it does not lie by
clearing; the dark red logo is what says the staleness is known.

Reproducing it needs no outage of anything shared:

```sh
export XDG_DATA_HOME=/tmp/tl-test     # short: a socket path is capped at 103 bytes
anchord &                             # a private daemon on its own socket
tallyd &                              # finds it by the same XDG rule
anchor create --label tl.slot=1:2:2 -- sleep 3600
kill -9 %1                            # and leave it dead
```

**Live mode genuinely restored, after it was genuinely broken.** A `SIGKILL`
left programmer mode on with no process alive to turn it off — the device's
own buttons were under host control and its Setup menu unreachable. After
the next `tallyd` started and then exited on `SIGTERM`, a mode readback
returned `0E 00` **and** the device's own mode buttons responded to presses
again, which they cannot do in programmer mode.

**The last row, tested by `SIGKILL`.** No restore ran, so the device was
left in programmer mode holding a stale frame — the genuinely unrecoverable
case this document admits to. Then the world was changed underneath it: one
session terminated, another created, while nothing was watching. On restart
`tallyd` rebuilt all three pads correctly — the survivor green, the
terminated one red, the new one green — and repainted all 81 LEDs so no
ghost of the old frame survived. The logo was dark, which under the
behaviour of the day was its own small confirmation — today the same run
would end on green.

**Nothing was carried across the kill.** The grid was reconstructed entirely
from `anchor list`, which is the whole argument for putting the binding in a
label rather than in a sidecar database.

## The one thing tallyd writes

`~/.local/state/tally/attention.jsonl` ([04](04-signals.md#the-instrument))
— the only state tallyd owns that is not rebuildable from `anchor list` plus
a config file, and it is read back at startup because it is the instrument
the product is gated on.

**`Journal` owns its sink.** The daemon used to hold a `Journal` (the
resumed map) and a separate `Option<File>` (the append handle), threading
the second through two helpers, so the rule *a write error retires the sink*
was implemented twice. A rule written twice is a rule that will be written a
third time, and the third one will forget the `flush` — and the flush is not
incidental: the journal is deliberately unbuffered, because the transitions
it exists to count are exactly the ones a `SIGKILL` would strand in a
buffer.

Each write is best-effort and self-retiring: on error it reports once and
closes the sink, and the daemon keeps running. **The grid lighting up
matters more than the log.** The line builders stay free functions over
`impl Write`, because that is what the transition tests assert against — a
string, never a file.

That disposability is the point. **tallyd holds no state that cannot be
rebuilt from `anchor list` plus a config file**, and being disposable is the
whole reason it is allowed to be the most fragile thing in the stack.

## Module shape

Every module that *decides* something is pure, and that is a consequence of
where the boundaries were drawn rather than of discipline:

```text
surface  <-  slot, chord, signal
             effect  <-  chord, signal, slot
             render  <-  effect, chord, signal, surface
             model   <-  effect, render, anchor, config, signal, slot, surface
             act     <-  effect, model, anchor, chord, config, slot
```

A DAG, and it was not always. `render` imported `model::{Consequence,
Effect}` while `model` imported `render::Health` — the one cycle this crate
has had. `Pad`, `Effect`, `Consequence` and `effect()` are the shared
vocabulary of three modules (`model` builds the pads, `render` colours them,
`act` carries them out), so they live in `effect.rs`, which depends on
`chord`, `signal` and `slot` and nothing else.

`Pads` stays in `model`: it borrows the resolved grid and the config, and it
remains **the one place a `Pad` is built** — the property that keeps the
preview and the press from drifting.

**One predicate, written once.** `queue::head`, `queue::depth` and
`queue::oldest` each spelled out the same eligibility test, and `head` and
`depth` were character-identical, while the module's own documentation
argued *"one predicate, read by the renderer and by the press; two would
eventually disagree, and a button that lights up and then does nothing is
one you press twice."* There were three. Worse, `oldest` deliberately
**omits** the `viewing` half — the tempo counts a request you are sitting in
front of and have not answered, where `►` must not offer you where you
already are — and that real, argued difference was visible only by diffing
three function bodies. One shared iterator makes it explicit at the two call
sites that want it.

**Three address grammars stay three**, deliberately: `slot::parse` is
canonical, `journal::parse_address` is a documented superset that argues for
itself, and `cli::address` asks `slot::parse` first and only re-walks the
text to say *which* part was wrong, which is the whole reason it has its own
copy.
