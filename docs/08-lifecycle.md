# 08 — Lifecycle

## One process, no socket

`tallyd` owns the MIDI port. That is exclusive — one process, one device —
so there is a daemon whether we want one or not.

What there is **not** is a control socket for it. The obvious design is
`tallyd` holding state and an `tally` CLI querying it over a second unix socket,
and that is one socket, one protocol, one framing and one set of error
codes more than this program needs. Everything `tally` would ask for is
already readable from somewhere else:

| `tally` wants | gets it from |
|---|---|
| what is on the grid | `anchor list --label`, same as tallyd |
| slot specs | `~/.config/tally/slots.toml` |
| to rebind a slot | edit the file, or `anchor label` |
| to jump to a slot | runs the same tmux commands tallyd would |

So `tally` is a second binary over the same two sources of truth, and tallyd
notices config changes on its reconcile tick. **No IPC between our own two
binaries.** Reject the socket.

The cost is a bounded staleness — up to one tick before the grid reflects
an edit — which is invisible at 2s and exactly the latency the rest of the
reconcile already has.

## Who starts it

A launchd user agent, declared in `~/dotfiles` and applied with provision
([07-seams.md](07-seams.md#provision)). Not a `service install` subcommand:
that exists in anchor because anchor is shipped to strangers.

`tallyd` must tolerate being started before anything else exists — no device,
no anchord, no tmux server. It reconciles into a working state when they
appear, and says which one is missing while they do not.

## Shutdown is not optional

**Programmer mode disables the device's Setup menu**
([02-surface.md](02-surface.md#entering-and-leaving-programmer-mode)). A
process that exits without sending the Live-mode message leaves a Launchpad
that has lost a feature with no obvious way to get it back.

So shutdown must:

1. stop any running text scroll
2. clear the surface
3. send Live mode — `F0 00 20 29 02 0D 0E 00 F7`
4. flush, then close the port

and it must happen on **SIGTERM and SIGHUP**, not only on a clean exit —
then re-raise, so the process still looks killed to whatever was waiting on
it. This is precisely the discipline anchor's `attach` applies to terminal
raw mode, for precisely the same reason: *a terminal left raw outlives the
process that did it.* So does a Launchpad left in programmer mode.

`SIGINT` is handled too, because unlike anchor's attach there is no
foreground terminal here for `^C` to mean anything else.

**It cannot be made perfect.** `SIGKILL` and a panic in the wrong place both
leave the device in programmer mode. The mitigation is that **start-up is
idempotent and total**: tallyd re-enters programmer mode, turns internal
feedback off, and paints all 81 LEDs from the model, so whatever state the
previous process died in is overwritten rather than inherited. Recovering on
start is worth more than exiting cleanly, because only one of the two is
reliable.

## The reconcile loop

One loop, 2s, doing four things in order:

```text
1. device?    poll midir ports; (re)connect, enter programmer mode, repaint
2. anchord?   list sessions with an tl.slot label; else light the logo red
3. watchers   start one per live session that lacks one
4. render     model -> frame, diff against last, emit one SysEx
```

Two seconds is teleport's reconcile cadence for the same job
(`anchor_watch.rs:31`). Presses are **not** on this loop — input is
event-driven off the MIDI callback and renders immediately. The loop is for
things that appear without telling us.

## Failure modes, and what each looks like

| what broke | what you see | what tallyd does |
|---|---|---|
| Launchpad unplugged | nothing (it is unplugged) | keeps reconciling, repaints on return |
| port held by another app | — | logs it, retries; does **not** fight for it |
| anchord down | logo dark red, grid frozen | retries; grid is **not** cleared |
| anchord restarted | slots go red-flashing (`lost`) | truthful — the sessions really are lost |
| tmux server gone | jump recreates it | spawn a view window on demand |
| a watcher dropped | nothing | supervisor restarts it from `scanned_to` |
| tallyd itself dies | lights freeze, then stale | next start repaints from anchor's labels |

The row that matters is the last one. **tallyd holds no state that cannot be
rebuilt from `anchor list` plus a config file**
([03-model.md](03-model.md#where-the-binding-lives)). That is what makes it
disposable, and being disposable is the whole reason it is allowed to be
the most fragile thing in the stack.
