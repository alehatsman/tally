# tally

A Novation Launchpad Mini [MK3] as the front panel for agent sessions.

```text
  ┌───────────────────────────────┐
  │ ● ● ● ● ● ● ● ●   pages       │        press an empty pad  → spawn an agent
  │ ┌───────────────┐ ┌─┐         │        press a live pad    → jump to it
  │ │               │ │●│         │        a pad that needs you flashes amber
  │ │   8 x 8       │ │●│ actions │
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

**Shipped and driving a real desk.** Six milestones, 213 tests, and a
Launchpad Mini MK3 on the desk reporting five agent sessions as you read
this. The hardware protocol in [docs/02-surface.md](docs/02-surface.md) was
read out of Novation's manual and verified against the device
([M0](ROADMAP.md#m0--prove-the-manual)); the grid, the jump, the chords,
the pages and `tally spawn` followed
([M1](ROADMAP.md#m1--the-grid-tells-the-truth)–[M5](ROADMAP.md#m5--tally-spawn)).

**The thing worth reading is what a day of use found.** The amber — the
signal this whole product is a bet on — never fired once against the agent
it was built for. Not a tuning problem: Claude Code does not ring the
terminal bell at all, so the detector was correct, the suite was green, and
there was nothing to detect. The attention half now comes from hooks the
agent calls ([M6](ROADMAP.md#m6--the-amber-exists),
[docs/14](docs/14-attention.md)).

That is the fifth time here that a surface fact only showed up on the
surface, and the list is kept on purpose: the three greens, the hard amber
flash, the title scroll, the top row, and now the bell that was never rung.
What remains is the only exit criterion that cannot be hurried — **a week
of `attention.jsonl`, and a number out of it.**

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
- **signals** — working / idle / needs-you / exited, derived from the
  session's own output
- **gestures** — spawn, jump, kill, resume, read-the-title
- **jump** — the press that puts the real terminal in front of you

## What it is not

**Not a terminal multiplexer.** No panes, no layouts, no attach. tmux does
that, you already use it, and the jump target is a tmux pane.

**Not a session daemon.** It never owns a PTY, never holds a control lease,
and goes away without anything noticing. anchor exists precisely so this
layer can be disposable.

**Not a MIDI library.** It drives one device, in one mode, in one way.

**Not a Stream Deck.** It does not run arbitrary macros. Sixty-four buttons
that each do a different thing is a configuration problem nobody solves
twice; sixty-four buttons that each do *the same thing to a different agent*
is a product.

## Design

| doc | what |
|---|---|
| [docs/01-scope.md](docs/01-scope.md) | the closed list, and how to reject a feature |
| [docs/02-surface.md](docs/02-surface.md) | the hardware: MIDI, programmer mode, palette, what it costs |
| [docs/03-model.md](docs/03-model.md) | slots, pages, and where the binding actually lives |
| [docs/04-signals.md](docs/04-signals.md) | how a pad knows the agent needs an answer |
| [docs/05-interaction.md](docs/05-interaction.md) | gestures and the colour language |
| [docs/06-jump.md](docs/06-jump.md) | the press that moves your eyes to the right window |
| [docs/07-seams.md](docs/07-seams.md) | what we ask anchor for, and where teleport fits |
| [docs/08-lifecycle.md](docs/08-lifecycle.md) | who starts tallyd, and why Live mode must be restored |
| [docs/09-testing.md](docs/09-testing.md) | testing a thing whose output is light |
| [docs/10-open-questions.md](docs/10-open-questions.md) | what is asserted here but not proven |
| [docs/11-cli.md](docs/11-cli.md) | `tally spawn`, for when the folder comes from you |
| [docs/12-layout-proposal.md](docs/12-layout-proposal.md) | a proposal: rows are projects — **not design**, tracked as [#8](https://github.com/alehatsman/tally/issues/8) |
| [docs/13-layout-review.md](docs/13-layout-review.md) | the review of it, and a narrower counter-proposal |
| [docs/14-attention.md](docs/14-attention.md) | the bell that is never rung, and the hooks that replace it |
| [docs/15-views.md](docs/15-views.md) | a view is a pane, not a window — step 1 cleared, the rest held |

## License

MIT
