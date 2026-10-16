# 05 — Interaction

## The colour language

The constraint that decides everything: **this must be readable from across
the room, without focusing on it.** That rules out subtle hue differences
and it rules out more than a handful of states.

**The grid shows what is alive or what is wrong.** Everything else is dark.

| state | colour | mode | reads as |
|---|---|---|---|
| nothing here | off | — | — |
| working | 21 green | **pulsing** | alive, producing output |
| quiet | 21 green | static | alive, idle |
| **needs an answer** | 9 amber | **flashing** vs 0 | **look at me** |
| ended badly | 5 red | static | go and look |
| lost | 7 dark red | static | the daemon cannot account for it |

Four colours and off. The rule is one sentence: **green means alive**, and
motion inside green means producing output. Anything not green is gone.

### Why it is one sentence

An earlier table had eight states and **three greens** — working, quiet, and
exited-clean. It failed in a way worth recording, because the failure is not
the one that gets designed against: the three were perfectly *legible* side
by side on real hardware, and nobody could *remember* which was which. A
colour language is a thing you recall, not a thing you resolve.

The fix was to stop green meaning two unrelated things. Hue now answers the
only question worth asking first — **can I talk to it, or must I restart
it?** Motion is a free refinement inside that answer, not a fourth thing to
learn.

Three states were deleted outright rather than recoloured:

- **"bound, not running"** — a slot with a spec and nothing in it looks the
  same as an empty one. Which pads spawn what is *your layout*, held in
  muscle memory, which is the entire reason a slot is a durable place
  ([03](03-model.md#a-slot-is-not-a-session)). Lighting it tells you
  something you already know.
- **"starting"** — it lasts about a second, and off-to-green is already the
  feedback that the press landed.
- **"exited, code 0"** — it needs nothing and its slot is free again, so it
  stops being tally's business. The row stays in anchor for `anchor logs`;
  the pad goes dark. **Failures stay visible precisely because they are the
  ones you have to go and look at** — that asymmetry is the design, not an
  oversight.

The cost, stated plainly: a finished agent looks like one that never ran.
That is tolerable because a *failed* one does not, and because an
interactive agent rarely exits at all — Claude Code sits at a prompt when
it is done, so "alive and idle" is the common resting state, not "exited".

### Only one thing flashes

Working pulses — a slow two-beat ramp. Attention flashes — a hard 50%
blink. They are different gestures rather than two intensities, so several
breathing pads do not compete with the one that wants you. Everything else
is still.

**`lost` is deliberately not animated**, and this is the case that proves
the rule. It arrives in **bulk**: anchord restarts and every running session
becomes a question mark at once. Sixty-four flashing pads is noise, and a
grid of *red* would say "twenty things failed" — false, and it sends you
debugging instead of purging. Dark red, static, is the truth: the daemon
lost track of these. It is the same colour the logo uses for an unreachable
anchord, so **dark red means the daemon had a problem, wherever you see
it**.

A muted slot ([04](04-signals.md#the-state-a-pad-renders)) renders the state
*underneath* its attention — green, pulsing or not. It does not get a colour
of its own: another thing competing for the eye is exactly what this section
refuses, and you muted it to stop looking at it.

**There is no "focused" colour.** An earlier draft had the slot you are
looking at render white, and it is wrong twice: tallyd would have to poll
tmux for the current window on every tick to know, and white would overwrite
*working* on the one pad whose working state you most want to see. The
window you are focused on is already in front of your face. The grid is for
the sixty-three you cannot see.

### The two rows that are not slots

The top row and right column are controls, and they get their own two
colours so the grid never has to be read for them:

| button | resting | held / active |
|---|---|---|
| page, holds no session | off | — |
| page, holds sessions | 37 turquoise, static | 3 white — the page you are on |
| page, off-screen attention | 9 amber, **flashing** vs 37 | — |
| action button (right column) | 45 blue, static | 3 white while held |

Page buttons use turquoise and action buttons blue because they must not be
mistakable for *any* slot state at a glance. Nothing in the slot palette is
near either.

An amber-flashing page button says **which** page wants you; the logo says
**that some page does**. Both, deliberately: the logo is visible from
further away and the page button is what you act on. See
[03-model.md](03-model.md#pages).

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
| 9 amber, flashing vs 0 | a session on a page you are not looking at needs an answer |

Unreachable wins. If anchord is down there is no session list, so there is
nothing that could be known to want you, and a logo doing both at once would
have to alternate — which is a third animation, on the one light whose whole
job is being unambiguous.

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

Mute toggles, and it is written to `tl.mute` on the session
([03](03-model.md#where-the-binding-lives)) so it survives tallyd restarting
and is visible to anything else reading anchor's labels. A mute dies with
the session it was set on, which is right: it was that agent you stopped
believing, not that pad.

While an action button is held, the grid **recolours to show what the
gesture would do**: holding kill dims every pad that is not killable and
reds the ones that are. The surface answers "what happens if I press this"
before you press it, which is the only honest way to put a destructive
action on a button with no label.

### The exact rules of a chord

Four of these are one sentence each and all four are load-bearing, because
between them they decide whether a fat finger kills an agent.

1. **The modifier must be down before the slot goes down.** Tapping a slot
   and *then* grabbing kill does nothing — the slot press already resolved
   as a jump. Order is not a detail here; it is the difference between a
   gesture and a race.
2. **The action fires on slot press-down, not release.** A chord is
   deliberate by construction (you are holding something with your other
   hand), so there is nothing left for a release to confirm, and firing on
   release would mean sliding off the pad silently cancels.
3. **The 400ms hold-to-scroll does not apply while a modifier is held.** A
   held slot under a held modifier is a kill that you are being slow about,
   not a title request. One gesture per press, decided at press-down by
   whether a modifier is down.
4. **Releasing an action button without tapping is a no-op. Always.** This
   is the escape hatch that justifies hold-then-tap over a prefix in the
   first place: you can change your mind with the hand that is already
   committed.

Releasing the *slot* first, then the modifier, is the normal case and means
nothing extra. Pressing a second slot while still holding the modifier fires
the gesture again on that slot — the modifier is a mode for as long as it is
down, not a one-shot.

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

**The device takes ASCII and a title is UTF-8.** So the title is sanitised
on the way out, in the surface layer, by one rule: printable ASCII
(`0x20`–`0x7E`) passes, everything else becomes a space, runs of spaces
collapse, and the result is truncated to 64 characters — about nine seconds
of scrolling, which is already longer than anyone holds a pad. Dropping the
em-dashes and box-drawing characters an agent puts in its title is a better
outcome than refusing to scroll, and a far better one than sending a byte
above `0x7F` inside a SysEx, where the device will read it as `F7` and end
the message early.

A slot with no title scrolls nothing and the hold is a no-op — not an error,
not a placeholder. The title is a signal that is allowed to be absent
([04](04-signals.md#3-osc-title--what-it-is-doing)).

## What a press does *not* do

**No key repeat, no double-tap, no long-press-to-activate.** One tap, one
action. The only timing in the whole surface is the 400ms hold that starts
a title scroll, and that one is forgiving in the right direction: a slow tap
scrolls a title, which is harmless, rather than spawning something.

**No input.** You cannot answer the agent from the grid. The grid tells you
which window to look at; the window is where you type
([01-scope.md](01-scope.md)).
