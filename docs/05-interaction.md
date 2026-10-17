# 05 — Interaction

## The colour language

The constraint that decides everything: **this must be readable from across
the room, without focusing on it.** That rules out subtle hue differences
and it rules out more than a handful of states.

**The grid shows what is alive or what is wrong.** Everything else is dark.

| state | colour | mode | reads as |
|---|---|---|---|
| nothing here | off | — | — |
| working | 21 green | static | alive |
| quiet | 21 green | static | alive |
| **needs an answer** | 9 amber | **pulsing** | **look at me** |
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

### Nothing blinks, and only attention moves

**One thing moves**, and it breathes — the device's two-beat ramp. Amber
is reserved, so nothing else on the surface is ever that colour, and
nothing else on the resting surface ever moves.

There is exactly one exemption, and it is a preview rather than a state:
under a held Move with a pad in hand, that one pad **flashes**
([22](22-the-mode-button.md#the-colours)). It breaks both halves of this
heading at once, and it is allowed to, because both were measured on the
surface at **rest** — a pad in your hand is not. It is one pad, it lasts
as long as your thumb, and the grid it is on has no amber to be confused
with, because a held modifier repaints all 64.

It flashes rather than pulses for a reason that is not taste: both
figures run on the one clock ([17](17-the-tempo.md)), so speeding this
pad up by the clock would speed amber up too and lie about how long
something has been waiting. A flash is one beat where a pulse is two,
which is the only way to be faster without touching the tempo.

⚠️ **Working and quiet used to differ, and the table above was changed.**
Green pulsed for *producing output* and sat still for idle. The pulse was
reported as **tiring** after a day beside it, and the diagnosis is worth
keeping:

- **Five working agents is five pads in motion**, carrying something
  nobody ever acts on. The surface was busiest exactly when nothing
  needed anything.
- **The tempo is one global clock** ([17](17-the-tempo.md)). A single pad
  waiting fifteen minutes drove *every unrelated green pulse* to 140 bpm,
  where a breath cycle is 0.86 s. Urgency was being broadcast onto pads
  that had none — measured at **19% of a 10-hour day** in the 120-140
  band, against 61% at the calm floor.
- The distinction is still **known** — it is what the attention signal is
  derived from ([04](04-signals.md)) — it just stopped being **shown**.

The cost, stated plainly: you can no longer see which agents are actively
producing output. That turns out to be information you watch rather than
act on, and the pad you act on is amber.

This is the second time this section has been rewritten by *a day beside
the thing*, and both times in the same direction: **less motion**. The
measurement below took a few hours; this one took a day; both found that
what reads well at a glance wears badly by the afternoon.

An earlier version of this section argued the opposite, and it is worth
recording why it was wrong. It said *movement is the alphabet* and gave
attention a hard 50% blink so it would outrank the slow green pulse. That
is correct for a glance and wrong for a day. The blink was confirmed on
hardware as reading "urgent" — and then confirmed in use as **stressful**,
which is a different measurement that only a few hours beside the thing can
take. A surface you sit next to all day has a constraint a surface you
glance at does not.

So the alphabet is colour. Breathing no longer says *something is
happening here* — it says **something is asking**, which is the one thing
worth interrupting for, and the only hue that was ever a request. Checked both ways
after the change, because gentler is only an improvement if it still works:
**calmer, and still catches the eye** from a screen.

**`lost` is deliberately not animated at all**, and this is the case that
proves the rule holds. It arrives in **bulk**: anchord restarts and every
running session becomes a question mark at once. Sixty-four moving pads is
noise, and a grid of *red* would say "twenty things failed" — false, and it
sends you debugging instead of purging. Dark red, static, is the truth: the
daemon lost track of these. It is the same colour the logo uses for an
unreachable anchord, so **dark red means the daemon had a problem, wherever
you see it**.

A muted slot ([04](04-signals.md#the-state-a-pad-renders)) renders the state
*underneath* its attention — green, which is now all that alive looks
like. It does not get a colour
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
| `▲` `▼` scroll, nothing that way | off | — |
| `▲` `▼` scroll, somewhere to go | 1 dark grey, static | `▲` held past 500 ms is `⌂` |
| `◄` back, no origin | off | — |
| `◄` back, origin recorded | 3 white, static | — |
| `►` next, queue empty | off | — |
| `►` next, something waiting | 9 amber, **pulsing** | — |
| `Session` map | 1 dark grey, static — always | 3 white while held ([20](20-the-overview.md)) |
| `Drums` arts | 1 dark grey, static — always | 3 white while a piece plays ([21](21-the-arts.md)) |
| `Keys`, `User` | off | — |
| `>` launcher, row has nothing to inherit or nowhere to put it | off | — |
| `>` launcher, ready | 1 dark grey, static | — |
| `CC19` corner, **Stop** | 5 red, static | 3 white while held |
| `CC19` corner, **Move** | 37 turquoise, static | 3 white while held |

⚠️ **This table was rewritten by [16](16-the-queue.md).** The top row used
to be eight page buttons, and the rule below — *exactly one page button is
bright, and it is always the one you are on* — was paid for in real
confusion. It is gone, deliberately, on the strength of one number: **pages
used, 70 : 2**. The argument, and what to revisit if pages ever become
load-bearing, is in [16](16-the-queue.md#what-is-given-up-stated-plainly).
What follows is kept because the lesson outlives the layout.

**Exactly one page button was bright, and it was always the one you were
on.** That was the rule, and it was learned the hard way: the row used to light
the current page white and occupied pages turquoise, which failed exactly
as the [three greens](#why-it-is-one-sentence) did. Both were perfectly
legible; neither said *which is which*. The report from the desk was *"I
am confused which one is selected, the white one or the turquoise one"* —
two bright colours, two meanings, and no way to tell them apart without
remembering a table.

So **brightness answers "where am I" before hue answers anything**. One
bright thing, some dim things, and the dim ones say "there is work over
there". Hue on this row is then spent only on the thing that is urgent —
amber, the same amber as everywhere else, meaning the same thing.

The kill button is **red**: the colour of what it does, and the colour
the pads it will touch are about to be. It shares red with a failed slot,
and that is survivable because it is not on the grid — it is the bottom
corner of the right column, where the silkscreen says *Stop*. Position
disambiguates before hue has to.

An amber `►` says **that something wants you**; the logo says **that it is
off this page**. Both, deliberately: the logo is visible from further away
and `►` is what you act on. You no longer navigate *to* attention — `►`
navigates attention to you, across pages ([16](16-the-queue.md#the-top-row)).
See [03-model.md](03-model.md#pages).

Both animations are the device's own
([02-surface.md](02-surface.md#colour)) — set once, no repaint loop. That is
not just an optimisation: a host-driven blink drifts, stutters under load,
and keeps blinking for one frame after the state changed. The hardware's
does not.

**How fast it breathes is a signal of its own** — the age of the oldest
thing waiting, carried by MIDI clock ([17](17-the-tempo.md)). The ramp
stays on the device, so none of the above is given back; only the beat
boundary becomes the host's.

Hue says *what*, motion says **something is asking**, and tempo says *how
long it has been*. The global clock stopped being a flaw the moment green
went still: it now modulates only the pads that are actually waiting,
which is what it always claimed to measure.

### The logo is the health light

`CC99` is not a slot. It is the status of the stack behind the surface:

| logo | means |
|---|---|
| off | tallyd running, anchord reachable, nothing to say |
| 7 dark red, static | **anchord unreachable** |
| 9 amber, pulsing | a session on a page you are not looking at needs an answer |

Unreachable wins. If anchord is down there is no session list, so there is
nothing that could be known to want you, and a logo doing both at once would
have to alternate — two meanings on the one light whose whole job is being
unambiguous.

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
| tap `▲` `▼` (top row) | scroll the grid one row |
| tap `◄` (top row) | **back** — return to where you were standing |
| tap `►` (top row) | **next** — jump to the longest-waiting amber |
| hold `▲` past 500 ms | **home** — scroll all the way back to row 1 ([20](20-the-overview.md)) |
| hold `Session` | the grid becomes a **map** of all 64 rows; tap a pad to go there ([20](20-the-overview.md)) |
| tap `Drums` | the grid becomes an **art**; `►` is the only way back ([21](21-the-arts.md)) |
| tap `>` (right column, rows 1-7) | **launch** into that row ([18](18-the-launch-column.md)) |
| hold `CC19` + tap slot | **Stop** — terminate what is alive, forget what has ended |
| double-tap `CC19` | swap the corner between **Stop** and **Move** ([22](22-the-mode-button.md)) |
| hold `CC19` in Move + tap, tap | **move** a session from one pad to another |

⚠️ **`CC19` used to be kill and only kill, and this table is the record of
that changing.** The corner carries two verbs now because
[18](18-the-launch-column.md) spent every other button on `>` and left
nothing spare — so purge joined kill under one hold, and Move went behind a
double-tap. Both arguments, and the cost of each, are in
[22](22-the-mode-button.md). The grammar below is unchanged: it is still
hold-then-tap, and letting go is still how you change your mind.

**Mute is not on this list, and it is not gone.** It has no button —
`tally mute <addr>` is the whole of it ([11](11-cli.md)) — because it
scored zero presses in a day and the grid had no button to spare. It is
still written to `tl.mute` on the session
([03](03-model.md#where-the-binding-lives)), so it survives tallyd
restarting and anything reading anchor's labels can see it, and it still
dies with the session it was set on: it was that agent you stopped
believing, not that pad. It stays because it is the starvation hatch for
`►` and for the tempo — a prompt you will never answer otherwise sits at
the head of the queue forever ([16](16-the-queue.md), [17](17-the-tempo.md)).

While an action button is held, the grid **recolours to show what the
gesture would do**. The surface answers "what happens if I press this"
before you press it, which is the only honest way to put a destructive
action on a button with no label.

### The held preview

Three levels, and the middle one is the one that earns its keep:

| pad | shows |
|---|---|
| the gesture would act on it | the gesture's colour, static |
| holds a session the gesture would **not** touch | 1 dark grey — dim |
| nothing there | off |

Dim rather than off, because "that session exists and this gesture will not
touch it" is the answer to the question you are actually asking. Hold Stop
and the dead rows light while the running agents go dim: you learn *why* a
pad is not a target, instead of watching two thirds of the grid vanish and
guessing.

**Move with a pad in hand is the one place the levels invert.** Everywhere
else, bright is the target and dim is what will be spared. There, *most of
the grid* is a legal destination — so room is drawn at the dimmest level
the language has, and the only bright things left are the two that are not
room: what is occupied, and what is in your hand
([22](22-the-mode-button.md#picking-something-up-flips-the-question-not-the-board)).

| held | pad | colour | what the tap does |
|---|---|---|---|
| `CC19` **Stop** | alive — working, quiet, wants-you | 5 red | terminate it |
| `CC19` **Stop** | hiding ended rows | 7 dark red | forget **all** of them |
| `CC19` **Move**, hand empty | holds a live session | 37 turquoise | pick it up |
| `CC19` **Move**, hand full | in hand | 37 turquoise, flashing against dark | — |
| `CC19` **Move**, hand full | holds a live session | 37 turquoise | nothing — no swap |
| `CC19` **Move**, hand full | free, on the visible grid | 1 dark grey | put it down there |
| either | a session this gesture will not touch | 1 dark grey | nothing |
| either | nothing there | off | nothing |

⚠️ **This table had four rows and four modifiers.** Respawn, purge and mute
left when the right column became launchers
([18](18-the-launch-column.md)); purge then came back *onto the same hold
as kill*, and Move arrived behind a double-tap, because
[22](22-the-mode-button.md) found the surface had no spare buttons left.
Two things about that are worth stating here rather than there:

- **Kill and purge can share a hold because their targets are disjoint.**
  Alive and ended are different sets, so every pad under the thumb has
  exactly one outcome and exactly one colour. The cost is real and named:
  purge's targets include pads that draw dark, so the dark two-thirds of
  the grid used to be a free miss and is not any more. What makes it
  affordable is that **the hold is what reveals them** — they stop being
  invisible at the moment they become targets.
- **Each colour is the colour of the consequence, not of the button.** That
  is why `preview` takes no `Action` at all: one button now carries two
  verbs, so a colour derived from the button would have to be wrong about
  one of them.

One rule survived every one of those changes and is the reason previews are
trustworthy at all: **the preview and the press read the same function.**
They did not always — a renamed spec once lit a pad green for a respawn
that then did nothing, because the preview counted a label and the press
resolved it. A preview that is wrong about one pad is a preview you stop
believing about all of them, which costs more than the gesture it was wrong
about.

The top row and the logo **do not** take part. Navigation and health keep
meaning what they always mean — a preview is a question about the grid, and
the health light is not allowed to answer a different one.

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
3. **How long you hold a slot means nothing.** This rule used to carve out
   the 400ms hold-to-scroll; [that feature is gone](#the-title-scroll--tried-and-abandoned)
   and the rule got simpler rather than disappearing. A slow press and a
   fast one do the same thing, so there is no way to be holding a pad
   "wrong", and a kill you are being slow about is still just a kill.
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

**A killed pad goes dark, not red**, and that needs a label to be true.
anchord records a terminate as an ordinary non-zero exit — there is no
field naming who asked for it, and there should not be: that is a client's
opinion, not the daemon's fact ([07](07-seams.md#what-we-must-not-ask-anchor-for)).
So tally writes `tl.killed` on the session immediately *before* the
terminate, and a terminal state carrying it renders as nothing.

That keeps the rule one sentence: **red means it ended in a way you did
not ask for.** Without it, the pad you just deliberately killed is
indistinguishable from an agent that crashed, and red stops meaning *go
and look* — which is the only thing red is for.

Two consequences, both deliberate:

- `anchor kill <id>` from a shell still goes red, because nothing told
  us. That is the honest answer: tally knows what tally did.
- Dark is not forgotten. The row and its scrollback stay — you often kill
  an agent *because* it went wrong and want to read why — and the purge
  chord is still how you clear them.

### The title scroll — tried, and abandoned

**Built, run on hardware, and deleted the same hour.** The reasoning that
led to it was sound and the result was unusable, which is worth more
written down than the feature was.

The idea: a pad cannot show what an agent needs — 64 pads are 64 pixels —
but a *held* pad could scroll `edit tests/pty.rs?` across the grid at 7
pads/second, enough to decide whether to jump now or later. The hardware
supports it generously: the scroll overlays the lighting without
destroying it, leaves the top row and logo alone, and runs on the
device's own clock ([02](02-surface.md#text-scrolling)).

**The grid is eight pads wide.** At the device's character cell that is
about **two legible characters at a time**, and a title read through a
two-character window moving past you is not reading, it is guessing. The
verdict after one hold on real hardware: *"it is impossible to read
anything."*

This is the same class of finding as the
[three greens](#why-it-is-one-sentence) and the
[hard amber flash](#nothing-blinks-and-only-attention-moves), and it is
the third time the project
has learned it: **a surface fact only shows up on the surface.** The byte
budget was fine, the sanitiser was correct, the tests were green, and the
feature was worthless. Nothing short of holding it down would have said
so.

There is no smaller version that works. Slower scrolling shows the same
two characters for longer; a shorter title still arrives two characters
at a time. The constraint is the width of the grid, and the grid is the
product.

So the honest answer to *"can the button show what it needs?"* is
**no**, and tally does not pretend otherwise. The pad says *that*
something wants you — which is the one bit it can carry from across the
room, and the bit the whole design is built on. *What* it wants is in the
window, one press away, rendered by a terminal that has eighty columns
and a font.

What survives: a hold is **not** a gesture. There is no timing on the
grid at all now, which makes the rules simpler than they were — one tap,
one action, decided at press-down.

## What a press does *not* do

**No key repeat, no double-tap, no long-press-to-activate, and no timing of
any kind.** One tap, one action, resolved at press-down. The surface had
exactly one threshold — the 400ms hold that started a title scroll — and
removing that feature removed the last clock from the input path. A press
is a press however long you lean on it.

**No input.** You cannot answer the agent from the grid. The grid tells you
which window to look at; the window is where you type
([01-scope.md](01-scope.md)).
