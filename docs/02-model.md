# 02 — The model

## A slot is not a session

The tempting model is "pad 34 shows session `01JABC…`". It is wrong, and it
is wrong in a way that only shows up after a day of use: sessions die. An
agent finishes, you kill it, `anchord` restarts and every session goes
`lost`. If the pad *is* the session, your muscle memory is erased every time
that happens, and a grid you cannot learn is a grid you look at instead of
feel.

So: **a slot is a durable place, and a session is what is currently in it.**
This is tmux's window index, and it is load-bearing for the same reason —
`prefix 3` is the third window whether or not the thing in it is the process
that was there this morning.

```text
slot  = (page, row, col)              the address of a pad, forever
spec  = cwd + argv + labels           what claiming an empty slot runs
bind  = slot -> session id            what is in it right now, or nothing
```

Press a **dark** slot → spawn its spec, if one is declared. Press a **lit**
slot → jump. A session exiting cleanly unbinds the slot; the pad goes dark,
and pressing it runs the same thing again.

**Dark is the only affordance, and it is deliberately ambiguous.** A slot
with a spec and nothing in it looks exactly like one with no spec at all.
Which pads spawn what is your layout, held in muscle memory. The grid shows
what is alive or wrong; it does not show what is possible
([05](05-interaction.md#why-the-colour-language-is-one-sentence)).

## The address, pinned

A slot address is written `page:row:col` and humans type it into shell
scripts, so both halves of it are fixed here and nowhere else.

```text
tl.slot = "work:3:5"
           │    │ └── col 1-8, 1 at the LEFT
           │    └──── row 1-8, 1 at the TOP
           └───────── page name, declared in slots.toml
```

**Row 1 is the top row.** The hardware numbers from the bottom
([03](03-surface.md#addressing)); the flip happens in the surface layer,
once, and no other file knows about it. A label is a human artefact and
reads top-down like everything else written on a screen.

**A page is a name, not an index.** The order of the `[[page]]` entries in
`slots.toml` is the order the pages stack into the row space — the first
entry's eight rows come first, the second entry's next, and so on down to
row 64.

## Pages are a naming scheme, not a place you go

Eight pages × 8 rows × 8 columns = 512 addressable slots, of which you will
use maybe twenty. Pages are not capacity, they are **grouping**: one page
per repo, or per machine, or per kind of work.

They used to be eight buttons across the top row, and switching page meant
cutting to a different screenful. They are now purely how an address is
*written*. Eight pages of eight rows stack into one sixty-four-row space,
and the grid is an **eight-row window** onto it that `▲▼` slide one row at a
time. A page boundary is an arithmetic detail: page 1 row 8 and page 2 row 1
are adjacent rows, and scrolling past them does nothing special, because
nothing special is there. ([D7](10-decisions.md#d7--pages-as-places-you-go))

Going blind to a page was the original argument for them landing early, and
it still stands — it just argues for something else now. *Switching away
from a page means going blind to it, and a surface you have to remember to
check is no better than a terminal you have to remember to check.* The fix
is no longer a per-page indicator. It is `►` — one press that goes to
whatever has been waiting longest, wherever it is, scrolling the grid to it
([05](05-interaction.md#the-queue-and-the-two-buttons-that-read-it)). **You no longer navigate to attention;
attention navigates to you.**

## The grid is flat, and that is the decision

Sixty-four rows of eight, an eight-row window, and an address you write that
nothing ever moves. Three measurements decided it, not taste:

**There is usually no *the* amber pad.** Time-weighted over an hour of real
use: 61% of the time more than one slot was asking, 15% exactly one. And the
choice was not close — when two or more are amber, the gap between the
longest and shortest wait is a median of 386 s. **The bottleneck is
ordering, not locating**, and no arrangement of the grid orders anything.
That is `►`'s job and the breathing rate's.

**At the observed density there is no locating problem either.** Seven rows
of sixty-four have ever held a session; five slots lit on a busy hour.
Finding a lit pad among sixty-four mostly-dark ones is not work.

**A grid you cannot learn is a grid you look at instead of feel.** Any
scheme where the daemon assigns addresses — by project, by urgency, by
recency — takes the addresses away from you.

Grouping still exists; you just do it yourself. The slots that asked for
attention in that hour were `1:1:1`, `1:2:1`, `1:3:1`, `1:8:1`, `1:8:2` —
one project per row, lane 1 durable, lane 2 the overflow. A convention over
declared addresses, costing nothing, stable because you chose it.

### Layouts that are closed

| proposal | why not |
|---|---|
| **rows are projects, columns are lanes** | answers *where* when the question is *which first*; invents a capacity cliff (eight rows of seven lanes for a workload of five) that flat addressing does not have |
| **row = urgency band** | self-sorting, always one row to look at, and **the pads move while you reach for them**. You cannot build muscle memory against a target that relocates, and a mis-press is a kill. This is the load-bearing rejection — it also closes the compacted map and anything else that re-sorts the grid under your hand |
| **column = project, row = lane** | symmetric and worse: the right column is adjacent to *rows*, so `>` stops meaning anything |
| **solo** — silence the other thirty-one | invisible global state needing self-clearing rules and a filter threaded through the queue, the tempo and the logo, to answer *"these are the ones I care about right now."* Move answers that physically, by putting them on row 1. **Arrangement beats state** |

✅ **Closed on data, 2026-10-18** ([#8](https://github.com/alehatsman/tally/issues/8)).
The number that would have reopened grouping is `queued` in the nav
journal: *if it is almost always 1, the 61% above was one unusual hour and
there is usually a single amber pad.* Over 55 hours and 62 `next` presses,
**`queued==1` is 35%** — depth 2 or more is 65%, and seven-deep happened
thirteen times. The one-hour measurement holds at twenty-four times the
sample. **The flat grid is final**, and grouping does not get a second
look.

> **What that hour could not have told you, and this one does:** the
> *shape* of the queue is lumpy rather than shallow. Thirteen presses at
> depth seven is one sustained pile-up, not a steady two. An arrangement
> scheme would have had to be right about where seven simultaneous asks
> live, which is a harder question than the one it was proposed to answer.

## Where the binding lives

**In anchor's labels.** Not in a sidecar database.

| label | holds |
|---|---|
| `tl.slot` | `"work:3:5"` — page, row, col. The binding |
| `tl.mute` | `"1"` — this slot never raises attention |
| `tl.attention` | `"1"` — a hook declared it is waiting |
| `tl.killed` | `"1"` — tally ended it on purpose, so the pad is dark |
| `tl.claude` | the Claude Code conversation id, for the dead pane ([06](06-views.md#the-dead-pane-names-its-conversation)) |

Only the first is required: a session carrying `tl.slot` and nothing else is
a fully functional slot. The others are state that would otherwise have to
live inside tallyd, and putting them here is what keeps the daemon's one
claim true — **tallyd holds nothing that cannot be rebuilt from a `list`.**
A mute that evaporates when tallyd restarts is worse than no mute at all,
because you muted that agent precisely because it would keep ringing.

anchor already offers exactly this: arbitrary caller-owned key/value, never
interpreted by the daemon. What it buys:

- **tallyd restarts and the grid is unchanged.** Reconstructed from a
  `list`, not recovered from state tallyd was trusted to persist.
- **Two clients cannot disagree.** One copy of the binding, next to the
  session it describes.
- **Someone else's session can claim a slot.** `anchor create --label
  tl.slot=work:3:5 -- claude` from a shell script lights a pad. The surface
  is scriptable without the surface knowing it.
- **A dead session keeps its address.** anchor keeps exited rows until
  purged, so "the thing that was in 3:5 and failed" is still addressable —
  which is exactly when you want to look at it.

The cost is one unfiltered `list` per reconcile tick, which is the call
tallyd is making anyway ([01](01-scope.md#what-tally-asks-of-anchor-and-what-it-must-not)).

## Pages are named in a file

`~/.config/tally/slots.toml`, and it holds one table.

```toml
# Order is the layout: the first page's rows come first in the stack.
[[page]]
name = "work"
[[page]]
name = "infra"
```

**It is optional, and in practice it does not exist.** No file is eight
unnamed pages (`1`–`8`), and the grid lights up for anything a shell script
labelled.

### There used to be three tables

`[[spec]]` was *what to run* and `[[slot]]` was *what is where*, and
pressing a dark pad resolved one against the other and spawned it. All of
it is cut (#40). What killed it was not neglect but a measurement: **cwd is
the one fact a config file cannot know in advance and a shell always
knows**, so declaring a slot meant predicting work you had not done yet.
`~/.config/tally/` was empty, `tallyd` loaded *8 pages, 0 slots*, and the
only two spawn paths in daily use — `tally spawn` and the `>` column —
both take their directory from something that already knows it: a shell,
or the row's newest session ([D8](10-decisions.md#d8--slotstoml-as-the-spawn-path),
[07](07-cli.md)).

Gone with them: the `tl.spec` label, the `[[slot]]`/`[[spec]]` tables, and
**a bare tap on a dark pad does nothing at all** — there is no longer an
anchord call anywhere on that path. The scope bullet it answered is
reworded in [01](01-scope.md).

A `[[spec]]` or `[[slot]]` table left in an old file is now an unknown
key, so it is **refused loudly** rather than ignored.

### Size is not ours

`create` takes `cols`/`rows` and tally sends anchor's defaults, because
`anchor attach` resizes the session to the terminal it is attaching to
before any output arrives, and the view window is that terminal. The one
visible consequence: output produced between the spawn and your first jump
was wrapped at 80 columns — the replay of a few seconds of startup banner.
The alternative is tally guessing at the size of a window that does not
exist yet.

**The view window owns the size, and keeps owning it.** Drag the pane and
tmux sends `SIGWINCH` to what is running in it; `anchor attach` watches for
that and writes a `resize` frame on its own attached connection. The PTY
follows the window, and the agent re-renders at the width it is actually
being read at. Tally still never calls `resize` — a surface that resized a
session under whoever holds the lease is the inverse of the problem.
(This was anchor's one documented gap, anchor#15, fixed in anchor#21. Against
an older `anchor` binary the symptom is an agent rendering at the size the
pane had when you jumped to it; the workaround is to close the view and
press the pad again.)

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
    │  green static · amber pulsing               │────────────┐
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
`ready` are indistinguishable because the alternative — a colour for "could
run something here" — is a light that tells you what you already know,
competing with the lights that do not.

`lost` hangs off `running`, not off `exited`, and that is the whole reason it
is a separate state: the process may well still be alive and orphaned. Only
a purge gets a slot out of it.

Note what is **not** here: no restart policy, no supervision, no "keep this
slot full". That is systemd, and anchor's scope doc already refused it. A
slot that goes dark stays dark until a human presses it.

### Exited rows accumulate, and purge is the answer

A clean exit leaves its anchor row in place, still carrying `tl.slot`. The
pad goes dark anyway — `model::resolve` drops a clean exit before it can
shadow the pad — so pressing it spawns a *second* session claiming the same
address, and newest-wins renders the new one.

Over a week of pressing one pad that is a pile of exited rows on one slot.
This is anchor's model working as designed, not a leak: rows live until
somebody asks for them to be gone, because an exited session is still worth
`anchor logs`. `tally purge` and the Stop chord on a dark pad are how you
ask. tally does **not** purge on spawn — that would delete the log of the run
you just finished, to tidy a list you were not looking at.

## Limits

- 8 pages × 64 = 512 slots. No pagination beyond that, ever. **Four rows
  are in use**, and `lit` has never moved off four — which is an open
  question about the size of the space rather than about this limit
  ([11](11-unbuilt.md#512-slots-for-four-used-rows)).
- One session per slot. A slot with two is a slot you cannot read.
- Label values follow anchor's limits — keys match `[a-z0-9_.-]{1,64}`,
  values ≤ 256 bytes, ≤ 32 per session. Every `tl.` key is well inside all
  three.
- `tl.slot` is **user input and must be validated**, not trusted. A shell
  script writes it; so does a person at a prompt. Anything that is not
  `name:1-8:1-8` with a declared page name is **ignored, not clamped and not
  repaired** — it was written by something that disagrees with us about the
  address space, and putting it somewhere arbitrary is worse than leaving it
  off the grid. An ignored row is logged once, at `warn`, with the offending
  value.
- Two sessions claiming the same slot: the **newest wins** and the older is
  logged and left off the grid. anchor lists newest-first, so this costs
  nothing to implement and is the behaviour you want — you just spawned the
  new one. The cost is named in [07](07-cli.md#the-occupancy-check-is-advisory).
