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

**Design and a verified hardware surface. No implementation.** The protocol
in [docs/02-surface.md](docs/02-surface.md) was read out of Novation's
programmer's reference manual and then **checked against a real Launchpad
Mini MK3 on 2026-10-16** ([M0](ROADMAP.md#m0--prove-the-manual)).

The manual turned out to be accurate: port names, addressing, device
inquiry and all three worked examples came back exactly as written, a
press round-trips in ~1.9 ms, and a full-surface repaint arrives in one
SysEx. The two claims that were wrong were ours, not Novation's — the port
is **not** exclusive on macOS, and the byte budget was off by one per
frame. Every [M0](ROADMAP.md#m0--prove-the-manual) exit criterion is met; what
little residue remains is in
[docs/10](docs/10-open-questions.md#verified-against-hardware-2026-10-16)
and none of it blocks M1.

**M1 is the next milestone, and it writes code.**

The design is in [docs/](docs/). The sequencing is in
[ROADMAP.md](ROADMAP.md). M0 is the first milestone that writes code.

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

## License

MIT
