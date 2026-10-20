# tally

[![ci](https://github.com/alehatsman/tally/actions/workflows/ci.yml/badge.svg)](https://github.com/alehatsman/tally/actions/workflows/ci.yml)

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

The pitch: **the agents you are running are a physical object on your
desk.** Not a list you have to go look at. Sixty-four lit pads, one per
slot, each telling you from across the room whether it is working, done, or
stuck waiting on an answer — and one press away from the terminal it is
running in.

## The name

A **tally light** is the lamp on a television camera that shows which one is
live. A gallery has a dozen cameras and one tally lit, and the whole point
of it is that nobody has to ask — you look up and you know where to look.

That is the claim this project makes. The launcher half is the lesser half;
the product is sixty-four lamps that are right.

## Status

**Shipped and driving a real desk.** Nineteen milestones, M0–M18, in daily
use.
The hardware protocol was read out of Novation's manual and verified
against the device; the grid, the jump, the chords, `tally spawn`, the
queue, the tempo and the launch column followed, and then the surface ran
out of buttons. Every build item in [the roadmap](ROADMAP.md) is done.

**The number it was waiting on is in, and the amber is not noise.** The
gate since M1 was a week of `attention.jsonl`: *of the ambers raised, how
many did you go to, and how many aged out?* It was closed on
**2026-10-19**: roughly three in four sent you somewhere you went.

**The counts live in [the roadmap](ROADMAP.md#f0--measure-the-amber-), with
the date they were taken on, and not here.** A rate recomputed from a
growing journal is a number that moves, and a moving number written into
the front page is one nobody can check later. What belongs here is the
verdict it bought: build on the amber rather than rebuild it.

It took one day rather than seven because the instrument only started
working that morning. Reading the 55 hours before it found the journal
**did not record a plain pad jump** — the gesture the product exists for —
so it held the ambers that were raised and almost none of the answers.
That was five lines
([#43](https://github.com/alehatsman/tally/issues/43)); everything before
it can answer questions about navigation and nothing about the amber.

**The question had needed restating, too.** Amber used to mean *the
detector believes it rang and went quiet*; since the hooks it means *an
agent finished a turn*, about ten times an hour. A hook-raised amber is
never really *wrong* — it can only be **correct and ignored**
([04](docs/04-signals.md#what-the-instrument-cannot-see-and-what-the-signal-now-means)).
It turns out it mostly is not.

**Read the number with its window.** One day, and a busy one — twelve
agent sessions inside seven hours, and a day spent watching the grid is a
day with a high answer rate. It is enough to decide what it was gating
(build on the amber, rather than rebuild it) and not enough to claim the
rate holds at a quieter density.

**Three features were removed by their own measurements**, which is the
habit this project is actually built on: the spec system and
spawn-from-a-dark-pad, because `~/.config/tally/` was empty and a config
file cannot know a directory in advance; the overview, because `lit` sat
at 4 on all nine presses and the map never once crossed the empty middle
its design argues from; and one word, because `tl.attention` meant both
*blocked* and *finished* to a reader that turned out to be a program.

**The thing worth reading is what a day of use found.** The amber — the
signal this whole product is a bet on — never fired once against the agent
it was built for. Not a tuning problem: Claude Code does not ring the
terminal bell at all, so the detector was correct, the suite was green, and
there was nothing to detect.

That is one entry in [the record](docs/10-decisions.md), which is kept on
purpose: the three greens, the hard amber flash, the title scroll, the
green working pulse that was legible at a glance and tiring by the
afternoon, the bell that was never rung, and a row of eight panes that was
correct, addressable, tested, and on a real screen was eight 23-column
slivers nobody could read. **Each cost a build to find and none could have
been argued out in advance.**

## Install

```sh
cargo install --git https://github.com/alehatsman/tally --tag v0.9.0 tally
```

The trailing `tally` names the package: the repository holds two, and
`tallybridge` is the Windows half ([13](docs/13-bridge.md)). Drop `--tag`
to build the default branch instead ([12](docs/12-install.md)).

Then **the four hook lines**, which are the step the product lives on —
without them the grid is a launcher with liveness and the amber never
fires:

```json
"Notification":     "tally attention blocked || true"
"Stop":             "tally attention done    || true"
"UserPromptSubmit": "tally attention off     || true"
"PreToolUse":       "tally attention off     || true"
```

**[docs/12-install.md](docs/12-install.md) is the whole sequence** — what
it needs first, the real `settings.json` shape, how to check the hooks
arrived, what to do when a pad never goes amber, and how to take it off
again.

It needs [anchor](https://github.com/alehatsman/anchor) running, tmux, and
a Launchpad Mini MK3. Not on crates.io: one USB device, one OS, a tag is
the right size.

**On Windows**, the agents and `tallyd` run in WSL and a small bridge on
Windows owns the device ([13](docs/13-bridge.md)). **It has driven a real
Launchpad**, 2026-10-20: pads lit from WSL, presses back, the amber
breathing, the raise, a clean stop. The kill and unplug lines of its
checklist are still owed. The hook lines go in WSL's `settings.json`,
where the agents are.

## The documents

Everything in `01`–`09` is **shipped and running**, and states current
truth only — no amendments to apply in your head. `10` is why it says
that, `11` is what is specified and deliberately not built, `12` is how
to get it running, and `13` is the bridge — built, run on the device,
and owed the second half of its checklist.

| | |
|---|---|
| [01-scope.md](docs/01-scope.md) | the closed list, how to reject a feature, and what we may not ask anchor for |
| [02-model.md](docs/02-model.md) | a slot is a durable place; addresses, pages, config, and where the binding lives |
| [03-surface.md](docs/03-surface.md) | the hardware: MIDI, programmer mode, the palette, and what it cannot do |
| [04-signals.md](docs/04-signals.md) | how a pad knows the agent needs an answer — watchers, hooks, precedence |
| [05-interaction.md](docs/05-interaction.md) | the colour language, all 81 buttons, the chords, the queue, the tempo, the arts |
| [06-views.md](docs/06-views.md) | the jump: tally's own tmux server, a view is a pane, and how a moved session takes it along |
| [07-cli.md](docs/07-cli.md) | `spawn`, `fork`, `ls`, `purge`, `mute`, `attention` — each for a gap that was measured |
| [08-daemon.md](docs/08-daemon.md) | the loop, the lock, shutdown, failure modes, and why there is no socket |
| [09-testing.md](docs/09-testing.md) | testing a thing whose output is light, and what a green suite does not prove |
| [10-decisions.md](docs/10-decisions.md) | **the record** — what was built, what use found, what replaced it |
| [11-unbuilt.md](docs/11-unbuilt.md) | the level (a veto over a stale amber), and the graveyard |
| [12-install.md](docs/12-install.md) | what a stranger does, in order — including the hook lines the amber does not exist without |
| [13-bridge.md](docs/13-bridge.md) | the Launchpad on Windows, the agents in WSL: five questions (four answered on the desk, one from the protocol), the wire, and the hardware checklist, six of eleven passed |

[ROADMAP.md](ROADMAP.md) is the order it arrived in, and the finish line is
the only part of this repo that is a plan rather than a record.

## What it is

A client of [anchor](https://github.com/alehatsman/anchor), and nothing
else. anchor owns the PTYs; tally owns a grid of LEDs and an opinion about
what they mean.

- **slots** — 64 pads, each a durable binding to *a thing you run*, not to a
  session id
- **signals** — working / idle / needs-you / exited. Liveness comes from
  anchor; *needs-you* comes from hooks the agent calls
- **gestures** — jump, launch into a row, kill, forget, and move a pad
  somewhere else
- **jump** — the press that puts the real terminal in front of you

## What it is not

**Not a terminal multiplexer.** tmux does that, and the jump target is a
tmux pane. tally makes panes — one per live slot, on its own tmux server,
where a row is a session and a column is a window — but exactly the ones the
grid implies, and no way to arrange *them*. The layout is not a feature; it
is the grid, reflected. **Your own tmux is never touched**, and that is a
property of the socket rather than of good behaviour: a `switch-client`
cannot reach a client it cannot list.

**Not a session daemon.** It never owns a PTY, never holds a control lease,
and goes away without anything noticing. anchor exists precisely so this
layer can be disposable.

**Not a MIDI library.** It drives one device, in one mode, in one way.

**Not a Stream Deck.** Sixty-four buttons that each do a different thing is
a configuration problem nobody solves twice; sixty-four buttons that each do
*the same thing to a different agent* is a product.

## License

MIT
