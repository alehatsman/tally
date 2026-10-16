# 05 — Interaction

## The colour language

The constraint that decides everything: **this must be readable from across
the room, without focusing on it.** That rules out subtle hue differences
and it rules out more than a handful of states.

| state | colour | mode | reads as |
|---|---|---|---|
| unbound | off | — | nothing here |
| bound, not running | 1 dark grey | static | something could go here |
| starting | 13 yellow | static | hold on |
| working | 21 green | **pulsing** | it is thinking |
| quiet | 21 green | static | it is alive and not busy |
| **needs an answer** | 9 amber | **flashing** vs 0 | **look at me** |
| exited, code 0 | 23 dark green | static | done, nothing to do |
| exited, non-zero | 5 red | static | done badly |
| lost | 5 red | **flashing** vs 7 | anchord restarted under it |
| focused | 3 white | static | this is the one on your screen |

Three things move, and only three: working pulses slowly, attention flashes,
lost flashes red. Everything else is still. **Movement is the alphabet** —
if five states animate, none of them are urgent.

Both animations are the device's own
([02-surface.md](02-surface.md#colour)) — set once, no repaint loop. That is
not just an optimisation: a host-driven blink drifts, stutters under load,
and keeps blinking for one frame after the state changed. The hardware's
does not.

### The logo is the health light

`CC99` is not a slot. It is the status of the stack behind the surface:

| logo | means |
|---|---|
| off | tallyd running, anchord reachable, nothing to say |
| 7 dark red, static | **anchord unreachable** |
| 9 amber, flashing | a session on another page needs an answer |

An unreachable anchord lighting the logo rather than clearing the grid is
the point. anchor made the same distinction for teleport — *"an unreachable
anchord is reported rather than read as an empty session list"*
(anchor `ROADMAP.md`, M5). Sixty-four dark pads mean "nothing is running".
They must never mean "I cannot tell".

## Gestures

The surface has one modifier scheme, and it is **hold-then-tap**, not
prefix-then-key.

tmux's prefix is right for a keyboard — `C-t` then `x` — because a keyboard
has no notion of "still holding". A grid does. Holding a right-column
action button and tapping a pad has a property prefix mode lacks: **you can
change your mind by letting go.** For a surface where one of the gestures
kills a running agent, that matters more than the keystroke saving.

| gesture | does |
|---|---|
| tap unbound slot | spawn its spec |
| tap bound slot | jump ([06-jump.md](06-jump.md)) |
| tap page button (top row) | switch page |
| **hold** a slot (>400ms) | scroll its title |
| hold `CC19` + tap slot | **kill** |
| hold `CC29` + tap slot | respawn / resume |
| hold `CC39` + tap slot | purge (exited only) |
| hold `CC89` + tap slot | mute — stop this slot raising attention |

While an action button is held, the grid **recolours to show what the
gesture would do**: holding kill dims every pad that is not killable and
reds the ones that are. The surface answers "what happens if I press this"
before you press it, which is the only honest way to put a destructive
action on a button with no label.

Releasing an action button without tapping is a no-op. Always.

### Kill is the one that needs the care

An agent mid-task is minutes of work. A fat finger is 8mm away from the
wrong pad. So kill is: hold a *different* button, on a *different* row,
with the target *highlighted in red while you hold*, and `anchor kill`
underneath — which is a `terminate` that anchor already makes
synchronous-to-terminal-state, so the pad goes red at the moment it is
actually dead rather than when we asked.

No "are you sure" blink-and-confirm. A confirmation you give by pressing
the same pad twice is a confirmation you learn to double-tap through.

### The title scroll

Hold a pad; its title scrolls across the grid in the slot's own colour.
Release; the grid comes back.

The hardware makes this nearly free: the scroll **overlays** the lighting
without destroying it, and it does not cover the top row or logo, so pages
and health stay visible while it runs
([02-surface.md](02-surface.md#text-scrolling)). Stop it by sending the
empty text message.

This is the entire answer to "can the button show what it needs". It cannot
— 64 pads are 64 pixels — but a held pad can spell out `edit tests/pty.rs?`
at 7 pads/second, and that is enough to decide whether to jump now or later.

## What a press does *not* do

**No key repeat, no double-tap, no long-press-to-activate.** One tap, one
action. The only timing in the whole surface is the 400ms hold that starts
a title scroll, and that one is forgiving in the right direction: a slow tap
scrolls a title, which is harmless, rather than spawning something.

**No input.** You cannot answer the agent from the grid. The grid tells you
which window to look at; the window is where you type
([01-scope.md](01-scope.md)).
