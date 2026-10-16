# 03 — The model

## A slot is not a session

The tempting model is "pad 34 shows session `01JABC…`". It is wrong, and it
is wrong in a way that only shows up after a day of use: sessions die.
An agent finishes, you kill it, `anchord` restarts and every session goes
`lost`. If the pad *is* the session, your muscle memory is erased every time
that happens, and a grid you cannot learn is a grid you look at instead of
feel.

So: **a slot is a durable place, and a session is what is currently in it.**
This is tmux's window index, and it is load-bearing for the same reason —
`prefix 3` is the third window whether or not the thing in it is the process
that was there this morning.

```text
slot  = (page, row, col)              the address of a pad, forever
spec  = cwd + argv + labels           what pressing an empty slot spawns
bind  = slot -> session id            what is in it right now, or nothing
```

Press an **unbound** slot → spawn its spec. Press a **bound** slot → jump.
A session exiting unbinds the slot; the spec stays. The pad goes from green
to dim grey and is ready to run the same thing again.

## Where the binding lives

**In anchor's labels.** Not in a sidecar database.

```text
tl.slot   = "work:3:5"      page, row, col
tl.spec   = "claude"        which spec spawned it, for respawn
```

anchor already offers exactly this: arbitrary caller-owned key/value, never
interpreted by the daemon, queryable by exact match
(anchor `docs/02-model.md`, `docs/04-protocol.md`). teleport stores a
Claude resume id the same way, for the same reason it is stated there: *it
is written once and needs nothing kept in step.*

What that buys:

- **tallyd restarts and the grid is unchanged.** It is reconstructed from a
  `list`, not recovered from state tallyd was trusted to persist.
- **Two clients cannot disagree.** There is one copy of the binding, and
  it is next to the session it describes.
- **Someone else's session can claim a slot.** `anchor create --label
  tl.slot=work:3:5 -- claude` from a shell script lights a pad. The
  surface is scriptable without the surface knowing it.
- **A dead session keeps its address.** anchor keeps exited rows until
  purged, so "the thing that was in 3:5 and failed" is still addressable —
  which is exactly when you want to look at it.

The cost is one `list` per reconcile tick, which is the call tallyd is making
anyway.

**Specs live in a file**, not in labels: `~/.config/tally/slots.toml`.
A spec describes a slot that may have never run, so there is no session to
hang it on.

```toml
[[slot]]
at   = "work:3:5"
spec = "claude"
cwd  = "~/projects/futurumlab/anchor"

[[spec]]
name = "claude"
argv = ["claude"]
login_shell = true
```

Two tables rather than one because a spec is reused across slots and a slot
is a position. This is deliberately close to teleport's `presets.toml` —
same job, same shape, and a later merge should be possible without a
migration.

## Pages

Eight pages, selected by the top row (`CC91`–`CC98`). 512 addressable slots,
of which you will use maybe twenty — pages are not capacity, they are
**grouping**. One page per repo, or per machine, or per kind of work.

A page indicator is lit when it holds any session; it **flashes amber when
a session on a page you are not looking at needs an answer**. That is the
whole reason pages are in M-something-early rather than "later": without it,
switching away from a page means going blind to it, and a surface you have
to remember to check is no better than a terminal you have to remember to
check.

## Lifecycle of a slot

```text
         ┌─────────┐  config          ┌─────────┐
         │ unbound ├──────────────────│  bound  │
         │  (off)  │                  │  spec   │
         └────┬────┘                  │ (grey)  │
              │                       └────┬────┘
              │ press ──────────────────── │ press
              ▼                            ▼
         ┌──────────────────────────────────────┐
         │ running — green / amber / white      │
         └──────┬───────────────────────────────┘
                │ exit
                ▼
         ┌──────────────────────────────┐
         │ exited — dim green or red    │  press: jump to the scrollback
         └──────┬───────────────────────┘  kill-chord: purge
                │ purge
                ▼
             back to bound (grey) or unbound (off)
```

Note what is **not** here: no restart policy, no supervision, no "keep this
slot full". That is systemd, and anchor's scope doc already refused it. A
slot that goes dark stays dark until a human presses it.

## Limits

- 8 pages × 64 = 512 slots. No pagination beyond that, ever.
- One session per slot. A slot with two is a slot you cannot read.
- Label values follow anchor's limits; `tl.slot` is a fixed-shape string, so
  a malformed one is a bug in tallyd, not user input to validate.
- A session carrying an `tl.slot` for a page/row/col out of range is
  **ignored, not clamped**. It was written by something that disagrees with
  us about the address space, and putting it somewhere arbitrary would be
  worse than leaving it off the grid.
