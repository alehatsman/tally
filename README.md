# tally

A Novation Launchpad Mini [MK3] as the front panel for agent sessions.

```text
  ┌───────────────────────────────┐
  │ ▲ ▼ ◄ ► ▣ ◈ · ·   navigate    │        press a live pad    → jump to it
  │ ┌───────────────┐ ┌─┐         │        press > beside it   → new agent, same folder
  │ │               │ │●│         │        hold the corner     → kill or forget, previewed
  │ │   8 x 8       │ │●│ launch  │        a pad that needs you breathes amber
  │ │   64 slots    │ │●│         │   ┌──────────┐      ┌──────────────────┐
  │ │               │ │●│         │──▶│  tallyd  │─────▶│ anchord          │
  │ └───────────────┘ └─┘         │   │ (this)   │      │ PTY: claude      │
  └───────────────────────────────┘   └────┬─────┘      │ PTY: claude      │
         USB MIDI, programmer mode          │            │ PTY: codex       │
                                            │            └──────────────────┘
                                            ▼
                                     tmux switch-client
                                     (your actual window)
```

The pitch: **the agents you are running are a physical object on your desk.**
Not a list you have to go look at. Sixty-four lit pads, one per slot, each
telling you from across the room whether it is working, done, or stuck
waiting on an answer — and one press away from the terminal it is running
in.

## Status

**Shipped and driving a real desk.** Seventeen milestones and a Launchpad
Mini MK3 on the desk. The hardware protocol in
[docs/02-surface.md](docs/02-surface.md) was read out of Novation's manual
and verified against the device ([M0](ROADMAP.md#m0--prove-the-manual));
the grid, the jump, the chords, `tally spawn`, the queue, the tempo, the
launch column and the overview followed
([M1](ROADMAP.md#m1--the-grid-tells-the-truth)–[M11](ROADMAP.md#m11--the-overview)),
and then the surface ran out of buttons
([M12](ROADMAP.md#m12--the-corner-carries-two-verbs)–[M16](ROADMAP.md#m16--the-arts)).

Every build item in [the roadmap](ROADMAP.md) was done, and then the
Launchpad was asked to sit on a Windows desk with the agents in WSL —
which USB will not do without a kernel build.
[M17](ROADMAP.md#m17--the-bridge) is the answer and is **specified, not
built**: it was written on a Mac with no Windows hardware in reach, so it
opens with five probes rather than a codec. Everything else open here is
the half only use can close, and
[What is next](ROADMAP.md#what-is-next) is the one section to read before
building anything else.

**The thing worth reading is what a day of use found.** The amber — the
signal this whole product is a bet on — never fired once against the agent
it was built for. Not a tuning problem: Claude Code does not ring the
terminal bell at all, so the detector was correct, the suite was green, and
there was nothing to detect. The attention half now comes from hooks the
agent calls ([M6](ROADMAP.md#m6--the-amber-exists),
[docs/14](docs/14-attention.md)).

That is one of seven times here that a surface fact only showed up on the
surface, and the list is kept on purpose: the three greens, the hard amber
flash, the title scroll, the top row, the bell that was never rung, the
green working pulse that was legible at a glance and tiring by the
afternoon ([M9](ROADMAP.md#m9--the-tempo-says-how-long)), and — most
recently — a row of eight panes that was correct, addressable, tested, and
on a real screen was eight 23-column slivers nobody could read
([docs/24](docs/24-the-tally-server.md)). Each of those cost a build to
find and none of them could have been argued out in advance.

Two of them needed more than looking. The hard amber flash and the green
pulse both passed a glance on real hardware and failed a **day** beside the
thing, which is a different instrument and the slowest one here.

What remains is the only exit criterion that cannot be hurried — **a week
of `attention.jsonl`, and a number out of it.** The journal holds a little
over a day. Nine milestones have shipped on top of a signal whose
trustworthiness has never been measured to the standard
[M1](ROADMAP.md#m1--the-grid-tells-the-truth) set for it, and that is the
honest statement of where this stands.

## The name

A **tally light** is the lamp on a television camera that shows which one is
live. A gallery has a dozen cameras and one tally lit, and the whole point of
it is that nobody has to ask — you look up and you know where to look.

That is the claim this project makes. The launcher half is the lesser half;
the product is sixty-four lamps that are right.

## What it is

A client of [anchor](https://github.com/alehatsman/anchor), and nothing
else. anchor owns the PTYs; tally owns a grid of LEDs and an opinion
about what they mean.

- **slots** — 64 pads, each a durable binding to *a thing you run*, not to a
  session id
- **signals** — working / idle / needs-you / exited. Liveness comes from
  anchor; *needs-you* comes from hooks the agent calls, because it never
  rang the bell ([docs/14](docs/14-attention.md))
- **gestures** — jump, launch into a row, kill, forget, move a pad
  somewhere else, and one held button that turns the grid into a map of
  itself
- **jump** — the press that puts the real terminal in front of you

## What it is not

**Not a terminal multiplexer.** tmux does that, you already use it, and
the jump target is a tmux pane. tally does make panes — one per live
slot, on [its own tmux server](docs/24-the-tally-server.md), where a row
is a session and a column is a window — but it makes exactly the ones the
grid implies and gives you no way to arrange *them*. The layout is not a
feature; it is the grid, reflected. **Your own tmux is never touched**,
and that is a property of the socket rather than of good behaviour: a
`switch-client` cannot reach a client it cannot list.

**Not a session daemon.** It never owns a PTY, never holds a control lease,
and goes away without anything noticing. anchor exists precisely so this
layer can be disposable.

**Not a MIDI library.** It drives one device, in one mode, in one way.

**Not a Stream Deck.** It does not run arbitrary macros. Sixty-four buttons
that each do a different thing is a configuration problem nobody solves
twice; sixty-four buttons that each do *the same thing to a different agent*
is a product.

## Design

Every document below is settled design that is **shipped and running**,
except the three listed separately at the end. Where two of them disagree,
the later one wins and says so in its own first paragraph — the reversals
are kept rather than edited away, because the record of what the hardware
changed our minds about is the most useful thing here.

Two conventions, so the reversals stay readable rather than becoming a
diff you have to apply in your head. A ⚠️ marks a paragraph a later
document amended; a ⛔ marks a section kept only as the record of a
decision that was reversed. **A measurement inside a shipped document is
history, not status** — `14` records a gap that `14` itself closed, and a
spec once blocked on reading it the other way.

| doc | what |
|---|---|
| [01-scope.md](docs/01-scope.md) | the closed list, and how to reject a feature |
| [02-surface.md](docs/02-surface.md) | the hardware: MIDI, programmer mode, palette, what it costs |
| [03-model.md](docs/03-model.md) | slots, pages, and where the binding actually lives |
| [04-signals.md](docs/04-signals.md) | how a pad knows the agent needs an answer |
| [05-interaction.md](docs/05-interaction.md) | the colour language, the gestures, and what a press does not do |
| [06-jump.md](docs/06-jump.md) | the press that moves your eyes to the right window |
| [07-seams.md](docs/07-seams.md) | what we ask anchor for, and where teleport fits |
| [08-lifecycle.md](docs/08-lifecycle.md) | who starts tallyd, and why Live mode must be restored |
| [09-testing.md](docs/09-testing.md) | testing a thing whose output is light |
| [10-open-questions.md](docs/10-open-questions.md) | what is asserted here but not proven |
| [11-cli.md](docs/11-cli.md) | `tally spawn` and `tally fork`, for when the folder comes from you, and what `purge` and `mute` do |
| [12-the-layout.md](docs/12-the-layout.md) | what all 81 buttons do, why the grid is flat, and which arrangements are closed |
| [14-attention.md](docs/14-attention.md) | the bell that is never rung, and the hooks that replace it |
| [15-views.md](docs/15-views.md) | a view is addressed as a pane: it outlives its agent, and a jump is a round trip |
| [16-the-queue.md](docs/16-the-queue.md) | `►` goes to the longest wait, `◄` brings you back, and nothing is stored for either |
| [17-the-tempo.md](docs/17-the-tempo.md) | the breathing rate says how long the oldest thing has waited |
| [18-the-launch-column.md](docs/18-the-launch-column.md) | `>` spawns into its row, and the row already knows the folder |
| [20-the-overview.md](docs/20-the-overview.md) | `Session` held shows which of the sixty-four rows exist |
| [21-the-arts.md](docs/21-the-arts.md) | the grid as a thing to look at, and the menu that picks one |
| [22-the-mode-button.md](docs/22-the-mode-button.md) | the corner carries two verbs — Stop and Move — and a double-tap swaps them |
| [24-the-tally-server.md](docs/24-the-tally-server.md) | tally's own tmux server: a row is a session, a column is a window |
| [26-the-dead-pane.md](docs/26-the-dead-pane.md) | a killed view names the conversation it was running, as a command you can paste |
| [27-the-slot-collision.md](docs/27-the-slot-collision.md) | a corpse must not hold the slot: a view is found by address as well as by id |
| [31-the-moved-view.md](docs/31-the-moved-view.md) | a moved session takes its view with it: the pane follows the label, and the pad it left is free |

Not design, and nothing below is built:

| doc | what |
|---|---|
| [23-gesture-brainstorm.md](docs/23-gesture-brainstorm.md) | **brainstorm** — fork, sync, and spawn-four: three asks, and the two buttons left to pay for them |
| [25-the-squad.md](docs/25-the-squad.md) | **spec, and the one that needs a product decision first** — `User` + `>` puts four agents in a row; the leftmost is the master and you talk only to it. It does not fit `01`'s six bullets, it spends the last free button, and it has seven open questions |
| [28-the-level-and-the-edge.md](docs/28-the-level-and-the-edge.md) | **spec** — a hook is an edge, `claude agents --json` is a level, and the level may only veto |
| [29-what-the-loop-holds.md](docs/29-what-the-loop-holds.md) | the daemon's state has names, the journal owns its sink, and `effect` is the vocabulary below `model` and `render` |
| [30-the-bridge.md](docs/30-the-bridge.md) | **spec, unbuilt, and five of its facts have never been run** — the Launchpad on a Windows desk and the agents in WSL: `tallybridge` owns the device, `tallyd` owns everything else, the wire between them is raw MIDI, and the build order is [M17](ROADMAP.md#m17--the-bridge) |

## License

MIT
