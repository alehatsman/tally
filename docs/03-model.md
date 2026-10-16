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

### The address, pinned

A slot address is written `page:row:col` and humans type it into shell
scripts, so both halves of it are fixed here and nowhere else.

```text
tl.slot = "work:3:5"
           │    │ └── col 1-8, 1 at the LEFT
           │    └──── row 1-8, 1 at the TOP
           └───────── page name, declared in slots.toml
```

**Row 1 is the top row.** The hardware numbers from the bottom
([02-surface.md](02-surface.md#addressing)); the flip happens in the surface
layer, once, and no other file knows about it. A label is a human artefact
and reads top-down like everything else written on a screen.

**A page is a name, not an index.** The order of the `[[page]]` entries in
`slots.toml` maps names onto the eight top-row buttons — first entry is
`CC91`, eighth is `CC98`. Names are in the label because `anchor ls` output
should be readable without the config file open; the ordering is in the file
because which button a page sits under is a layout preference, not an
identity.

Press a **dark** slot → spawn its spec. Press a **lit** slot → jump. A
session exiting cleanly unbinds the slot; the spec stays, the pad goes dark,
and pressing it runs the same thing again.

**Dark is the only affordance, and it is deliberately ambiguous.** A slot
with a spec and nothing in it looks exactly like one with no spec at all
([05](05-interaction.md#why-it-is-one-sentence)) — which pads spawn what is
your layout, held in muscle memory. The grid shows what is alive or wrong;
it does not show what is possible.

## Where the binding lives

**In anchor's labels.** Not in a sidecar database.

```text
tl.slot       = "work:3:5"  page, row, col — the binding
tl.spec       = "claude"    which spec spawned it, for respawn
tl.mute       = "1"         this slot never raises attention
tl.attention  = "1"         a hook declared it is waiting
```

`tl.attention` is written by a `Notification` hook, never by a human — see
[04-signals.md](04-signals.md#4-a-hook--it-told-us).

Only the first is required: a session carrying `tl.slot` and nothing else is
a fully functional slot. The other three are state that would otherwise have
to live inside tallyd, and putting them here is what keeps
[08-lifecycle.md](08-lifecycle.md)'s claim true — **tallyd holds nothing that
cannot be rebuilt from a `list`.** A mute that evaporates when tallyd
restarts is worse than no mute at all, because you muted that agent
precisely because it would keep ringing.

anchor already offers exactly this: arbitrary caller-owned key/value, never
interpreted by the daemon (anchor `docs/02-model.md`,
`docs/04-protocol.md`). teleport stores a Claude resume id the same way, for
the same reason it is stated there: *it is written once and needs nothing
kept in step.*

One consequence to be clear-eyed about. anchor's `list` filter is **exact
match on a value**, AND across keys (anchor `src/store.rs:96-105`) — there
is no "has this key" filter, and `tl.slot`'s value is different for every
slot. So the reconcile tick lists **everything** and keeps the rows carrying
a `tl.slot` key. That is a client-side filter over a few dozen sessions,
which is nothing, and the alternative — a second constant-valued
`tl.owner=tally` label purely to make the filter work, as teleport does with
`teleport.owner` — would make `anchor create --label tl.slot=work:3:5`
silently not light a pad. One label is the binding. Asking anchor for a
presence filter is [not ours to ask](07-seams.md#what-we-must-not-ask-anchor-for).

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

The cost is one unfiltered `list` per reconcile tick, which is the call
tallyd is making anyway.

**Specs live in a file**, not in labels: `~/.config/tally/slots.toml`.
A spec describes a slot that may have never run, so there is no session to
hang it on.

```toml
# Order is the layout: first page is CC91, eighth is CC98.
[[page]]
name = "work"
[[page]]
name = "infra"

[[slot]]
at   = "work:3:5"
spec = "claude"
cwd  = "~/projects/futurumlab/anchor"

[[spec]]
name = "claude"
argv = ["claude"]
login_shell = true
```

Three tables, each answering a different question: `[[page]]` is *where the
top row points*, `[[spec]]` is *what to run*, `[[slot]]` is *what is where*.
A spec is reused across slots and a slot is a position, so they cannot be
one table. The `[[slot]]` / `[[spec]]` pair is deliberately close to
teleport's `presets.toml` — same job, same shape, and a later merge should
be possible without a migration.

Everything in this file is optional. A `slots.toml` that does not exist is
eight unnamed pages (`1`–`8`) and no specs: the grid still lights up for
anything a shell script labelled, and only *spawning from an empty pad*
needs the file. That matters because it makes [M1](../ROADMAP.md#m1--the-grid-tells-the-truth)
— which has no spawning in it — config-free.

### What a spawn actually sends

`slots.toml` is tally's vocabulary; `create` is anchor's. The translation
is three lines and all three are places to be wrong:

| `slots.toml` | `create` | note |
|---|---|---|
| `argv = ["claude", "--x"]` | `command: "claude"`, `args: ["--x"]` | anchor splits them; we hold them together because that is how you write a command |
| `cwd = "~/p/anchor"` | `cwd: "/Users/…/p/anchor"` | **tally expands `~`, not anchord.** `~` is a shell's idea, and nothing between the config file and the PTY is a shell |
| `env = { … }` | `env` | passed through; anchor never echoes it back ([04-protocol](../../anchor/docs/04-protocol.md)) |

Labels go with it: `tl.slot` is the address being claimed and `tl.spec` is
the spec's name, so a respawn can find what to run again without the slot
table. An empty `argv` is a config error, not a spawn of the empty string.

**Size is not ours.** `create` takes `cols`/`rows` and we send anchor's
defaults, because `anchor attach` resizes the session to the terminal it is
attaching to before any output arrives (`anchor/src/bin/anchor.rs:343-349`)
and the view window is that terminal ([07](07-seams.md#anchor): we never
call `resize`). The one visible consequence: output produced between the
spawn and your first jump was wrapped at 80 columns. That is the replay of
a few seconds of startup banner, and the alternative is tally guessing at
the size of a window that does not exist yet.

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
    no spec, no session          spec, nothing running
    ┌───────────────┐            ┌───────────────┐
    │    unbound    │            │  ready (off)  │
    │  off — no-op  │            │ press: spawn  │
    └───────────────┘            └───────┬───────┘
                                         │
                      ┌──────────────────┘
                      ▼
    ┌─────────────────────────────────────────────┐        anchord
    │ running                                     │        restarted
    │  green pulsing · green static · amber pulse │────────────┐
    │  press: jump                                │            │
    └────────┬───────────────────────┬────────────┘            ▼
             │ exit 0                │ exit ≠ 0          ┌─────────────┐
             ▼                       ▼                   │    lost     │
      back to ready (off)     ┌─────────────┐            │  dark red   │
      press spawns again      │ failed, red │            └──────┬──────┘
                              └──────┬──────┘                   │ purge
                                     │ purge                    ▼
                                     └───────────────────▶ ready (off)
```

**Two different things are off, and that is the design.** `unbound` and
`ready` are indistinguishable on the grid because the alternative — a
colour for "could run something here" — is a light that tells you what you
already know, competing with the lights that do not
([05](05-interaction.md#why-it-is-one-sentence)).

`lost` hangs off `running`, not off `exited`, and that is the whole reason
it is a separate state: the process may well still be alive and orphaned.
Only a purge gets a slot out of it. See
[04-signals.md](04-signals.md#the-state-a-pad-renders).

Note what is **not** here: no restart policy, no supervision, no "keep this
slot full". That is systemd, and anchor's scope doc already refused it. A
slot that goes dark stays dark until a human presses it.

### Exited rows accumulate, and purge is the answer

A clean exit leaves its anchor row in place, still carrying `tl.slot`. The
pad goes dark anyway — `resolve` drops a clean exit before it can shadow
the pad (`src/model.rs:76`) — so pressing it spawns a *second* session
claiming the same address, and newest-wins renders the new one.

Over a week of pressing one pad that is a pile of exited rows on one slot.
This is anchor's model working as designed, not a leak: rows live until
somebody asks for them to be gone, because an exited session is still worth
`anchor logs`. The purge chord is how you ask. tally does **not** purge on
spawn — that would delete the log of the run you just finished, to tidy a
list you were not looking at.

## Limits

- 8 pages × 64 = 512 slots. No pagination beyond that, ever.
- One session per slot. A slot with two is a slot you cannot read.
- Label values follow anchor's limits — keys match `[a-z0-9_.-]{1,64}`,
  values are ≤256 bytes, ≤32 per session (anchor `docs/02-model.md`). Every
  `tl.` key is well inside all three, and `tl.slot`'s value is a handful of
  bytes.
- `tl.slot` is **user input and must be validated**, not trusted. A shell
  script writes it; so does a person at a prompt. Anything that is not
  `name:1-8:1-8` with a page name declared in `slots.toml` is **ignored, not
  clamped and not repaired** — it was written by something that disagrees
  with us about the address space, and putting it somewhere arbitrary is
  worse than leaving it off the grid. An ignored row is logged once, at
  `warn`, with the offending value; silently dropping it is how you spend an
  afternoon wondering why a pad is dark.
- Two sessions claiming the same slot: the **newest wins** and the older is
  logged and left off the grid. anchor lists newest-first, so this costs
  nothing to implement and is the behaviour you want — you just spawned the
  new one.
