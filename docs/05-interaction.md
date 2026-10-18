# 05 — Interaction

What the eighty-one buttons mean, what they say when you are not touching
them, and what happens when you are.

## The colour language

The constraint that decides everything: **this must be readable from across
the room, without focusing on it.** That rules out subtle hue differences,
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

Four colours and off. The rule is one sentence: **green means alive, and
anything not green is gone.**

### Why the colour language is one sentence

**A colour language is a thing you recall, not a thing you resolve.** That
is stricter than legibility: eight states and three greens — working, quiet,
exited-clean — were perfectly legible side by side on real hardware, and
nobody could remember which was which ([D1](10-decisions.md#d1--three-greens)).

So hue answers the only question worth asking first — **can I talk to it, or
must I restart it?** — and nothing is spent on a finer distinction inside
that answer.

Four states earn no colour of their own:

- **"bound, not running"** — a slot with a spec and nothing in it looks the
  same as an empty one. Which pads spawn what is *your layout*, held in
  muscle memory, which is the entire reason a slot is a durable place.
  Lighting it tells you something you already know.
- **"starting"** — it lasts about a second, and off-to-green is already the
  feedback that the press landed.
- **"exited, code 0"** — it needs nothing and its slot is free again, so it
  stops being tally's business. The row stays in anchor for `anchor logs`;
  the pad goes dark. **Failures stay visible precisely because they are the
  ones you have to go and look at** — that asymmetry is the design, not an
  oversight.
- **"working" against "quiet"** — both green, both still. The distinction is
  known, because it is what the attention signal is derived from; it is
  simply not shown. Motion is spent entirely on amber.

Two costs, stated plainly. A finished agent looks like one that never ran —
tolerable because a *failed* one does not, and because an interactive agent
rarely exits at all: Claude Code sits at a prompt when it is done, so "alive
and idle" is the common resting state. And you cannot see which agents are
producing output, which turns out to be information you watch rather than
act on. The pad you act on is amber.

**There is no "focused" colour**, and it is refused twice over: tallyd would
have to poll tmux on every tick to know, and white on the focused pad would
overwrite the state of the one pad whose state you most want to see. The
window you are focused on is already in front of your face. The grid is for
the sixty-three you cannot see.

A muted slot renders the state *underneath* its attention — green. It gets
no colour of its own: another thing competing for the eye is exactly what
this section refuses, and you muted it to stop looking at it.

### Nothing blinks, and only attention moves

**One thing moves**, and it breathes — the device's two-beat ramp. Amber is
reserved, so nothing else on the surface is ever that colour, and nothing
else on the resting surface ever moves.

**The alphabet is colour; motion is one bit on top of it.** Breathing does
not say *something is happening here*, it says **something is asking** — the
one thing worth interrupting for, and the only hue that was ever a request.

Two rules decide how much of the surface may move, and both were set by a
day beside the thing rather than by a glance at it:

**Motion is spent only on amber.** Green pulsing for *producing output* put
five agents in motion whenever five were working — the surface busiest
exactly when nothing needed anything — and because the tempo is one global
clock, a single pad waiting fifteen minutes drove every unrelated green
pulse to 140 bpm. **19% of a 10-hour day** sat in that 120–140 band against
61% at the calm floor: urgency broadcast onto pads that had none, reported
from the desk as *tiring* ([D4](10-decisions.md#d4--the-green-working-pulse)).

**It breathes rather than blinks.** A hard 50% flash reads as urgent on
hardware and as **stressful** after a few hours. The gentler version was
checked both ways, because calmer is only an improvement if it still works:
calmer, and still catches the eye ([D2](10-decisions.md#d2--the-hard-amber-flash)).

Both point the same way: **what reads well at a glance can wear badly by the
afternoon.** A surface you sit next to all day has a constraint a surface
you glance at does not, and no amount of looking at it for ten seconds will
find that constraint.

**`lost` is deliberately not animated at all**, and this is the case that
proves the rule holds. It arrives in **bulk**: anchord restarts and every
running session becomes a question mark at once. Sixty-four moving pads is
noise, and a grid of *red* would say "twenty things failed" — false, and it
sends you debugging instead of purging. Dark red, static, is the truth: the
daemon lost track of these. It is the same colour the logo uses for an
unreachable anchord, so **dark red means the daemon had a problem, wherever
you see it**.

**There is exactly one exemption**, and it is a preview rather than a state:
under a held Move with a pad in hand, that one pad **flashes**. It breaks
both halves of this heading at once and is allowed to, because both were
measured on the surface at **rest** — a pad in your hand is not. It is one
pad, it lasts as long as your thumb, and the grid it is on has no amber to
be confused with, because a held modifier repaints all 64.

It flashes rather than pulses for a reason that is not taste: both figures
run on the one clock, so speeding this pad up by the clock would speed amber
up too and lie about how long something has been waiting. A flash is one
beat where a pulse is two — the only way to be faster without touching the
tempo.

### The tempo says how long

The grid says *something wants you*. Without this it never says *and it has
been waiting eleven minutes*, which is the one thing you need in order to
decide whether to get up. Forty-nine answered waits, from the journal:

```text
p10      7s     you were already looking
p25     50s
p50    187s
p75    335s
p90  1,076s     eighteen minutes
max  2,403s     forty minutes
```

**Two orders of magnitude, and the pad looks identical at both ends.** The
surface has one amber and it is spent; there is no second colour to give
this. So it goes on the free dimension: the device syncs its LED animations
to MIDI clock ([03](03-surface.md#midi-clock-drives-the-breathing)), so the
host owns the beat.

**Tempo is the age of the oldest thing waiting.** Not the count, not the
newest, not the average — the same quantity `►` sorts by, so the button and
the breathing cannot disagree.

```text
oldest wait   0s ────────────── 900s ──────────▶
       bpm   60 ────────────── 140 ───────────── 140 (clamped)
      cycle 2.0s ───────────── 0.86s
```

Linear in age, clamped at both ends. 900 s because the observed spread puts
p75 at 335 s and p90 at 1,076 s — the knee of the real distribution, not a
round number chosen first.

**The range is deliberately narrow and centred near today.** The device's
unsynced 120 bpm sits inside it. The slow end is *slower* than the grid has
ever been and the fast end barely faster, because the failure this design
has already paid for once is a surface that is correct and **stressful**. A
tempo that sprints is that mistake with a different knob.

**It is global, and that is the right scope.** One clock drives every
animation; there is no per-pad tempo and there must not be one. The surface
acquires one aggregate mood — calm while nothing is ageing, quickening as
the oldest thing ages — which you read without looking at any particular
pad. That is the only kind of reading that happens at six metres. Now that
green is still, the clock modulates only the pads that are actually waiting,
which is what it always claimed to measure.

**Frozen breath is a feature.** If `tallyd` wedges the clock stops, and the
device stops breathing mid-ramp rather than carrying on at its own 120. That
is a liveness indicator the grid has never had: a wedged daemon otherwise
leaves a perfectly normal-looking surface showing a frozen snapshot, which
is the worst failure a status display can have. **It must not be mistaken
for calm**, which is why the floor is 60 bpm and not 30: a very slow breath
and a stopped one have to be tellable apart.

Rejected: **per-pad tempo** (the clock is global; eight pads at eight rates
is the sixty-four-moving-pads problem again). **Tempo for the count instead
of the age** (five fresh ambers are not more urgent than one forgotten one,
and the count is already readable — they are all lit). **Flashing instead of
pulsing as things age** (that is the hard amber flash, already abandoned).
**Brightness instead of tempo** (the wrong axis: the colour language is
built on *bright means where you are*).

### The logo is the health light

`CC99` is not a slot. It is the status of the stack behind the surface:

| logo | means |
|---|---|
| off | tallyd running, anchord reachable, nothing to say |
| 7 dark red, static | **anchord unreachable** |
| 9 amber, pulsing | a session off the visible window needs an answer |

Unreachable wins. If anchord is down there is no session list, so there is
nothing that could be known to want you, and a logo doing both at once would
have to alternate — two meanings on the one light whose whole job is being
unambiguous.

An unreachable anchord lighting the logo rather than clearing the grid is
the point. Sixty-four dark pads mean "nothing is running". They must never
mean "I cannot tell".

## The layout

Checked against Novation's user guide, *Hardware Overview*, p.8 — what is
physically printed on the device, and what tally does with it:

```text
 ▲    ▼    ◄    ►   │ Session │  Drums   Keys   User │ ◈
                    │         │  └──── "Custom" ────┘ │
┌──────────────────────────────────────────────────┐  ┌──────┐
│                                                  │  │  >   │  ×7
│                     8 × 8                        │  │  >   │
│                                                  │  │  >   │
└──────────────────────────────────────────────────┘  ├──────┤
                                                      │ Stop │
                                                      │ Solo │
                                                      │ Mute │
```

| printed | what it means to the device | what it means here |
|---|---|---|
| ▲ ▼ | move the session outline across **scenes** (rows) | scroll the window one row |
| ◄ ► | move it across **tracks** (columns) | back where you were standing / go to the longest wait |
| Session | return to the overview | **held**: the grid becomes the whole 64-row map |
| Drums | one-shot sample pads | the arts: 64 pads doing something that is not work |
| Keys, User | custom modes | nothing. Two buttons, deliberately unspent |
| `>` | **launch a scene** — the whole row fires | spawn into that row, inheriting its directory |
| Stop / Solo / Mute | **cycles** the bottom row's verb | the corner's two verbs, swapped by a double-tap |
| ◈ logo | brand | the health light |

**Every legend is doing something close to what it says**, and that is not
decoration — it is the cheapest form of learnability this device offers. The
two exceptions are honest ones: `Keys` and `User` do nothing at all, because
an inert button that lights up like a live one is an invitation to press it.

### Every position

| position | resting | tap | held |
|---|---|---|---|
| grid pad, nothing there | off | spawn its spec, if `slots.toml` names one | — |
| grid pad, bound | green / amber / red / dark red | **jump** ([06](06-views.md)) | — |
| `Top(1)` ▲ | 1 dark grey if there is a row above | scroll up one | past 500 ms: **home**, back to row 1 |
| `Top(2)` ▼ | 1 dark grey if there is a row below | scroll down one | — |
| `Top(3)` ◄ | 3 white if an origin is recorded | **back** | — |
| `Top(4)` ► | 9 amber pulsing if anything waits | **next** — the longest wait | — |
| `Top(5)` Session | 1 dark grey, always | — | **the map**: 64 rows, tap to go |
| `Top(6)` Drums | 1 dark grey; 3 white while playing | **arts** | — |
| `Top(7)` Keys | off | nothing | nothing |
| `Top(8)` User | off | nothing | nothing |
| `Action(1..7)` `>` | 1 dark grey if the row can inherit | **launch** into that row | — |
| `Action(8)` `CC19` | 5 red in Stop, 37 turquoise in Move | double-tap **swaps the verb** | the modifier |
| `CC99` logo | the health light | — | — |

**Dark does nothing, lit does exactly one thing, and there is no state to
remember.** That is the rule the whole top row runs on, and the one
exception proves it: `Session` and `Drums` are always lit, because they
always have something to show.

**Brightness answers "where am I" before hue answers anything.** One bright
thing, some dim things, and the dim ones say *there is something there*. Hue
on these rows is then spent only on the thing that is urgent — amber, the
same amber as everywhere else, meaning the same thing.

That ordering is a rule, not a preference. **Two bright colours on one row
cannot be told apart without remembering a table**, which is the three
greens failure in a different place: a row lighting the current page white
and occupied pages turquoise was perfectly legible, and the report from the
desk was *"I am confused which one is selected, the white one or the
turquoise one."* Dim is how a row says *there is work over there* without
competing for the answer to *where am I*.

The corner is **red**: the colour of what it does, and the colour the pads
it will touch are about to be. It shares red with a failed slot, and that is
survivable because it is not on the grid — it is the bottom corner of the
right column, where the silkscreen says *Stop*. Position disambiguates
before hue has to.

An amber `►` says **that something wants you**; the logo says **that it is
off the window you are looking at**. Both, deliberately: the logo is visible
from further away and `►` is what you act on.

Both animations are the device's own — set once, no repaint loop. A
host-driven blink drifts, stutters under load, and keeps blinking for one
frame after the state changed. The hardware's does not.

Hue says *what*, motion says **something is asking**, and tempo says *how
long it has been*.

## Gestures

The surface has one modifier scheme, and it is **hold-then-tap**, not
prefix-then-key.

tmux's prefix is right for a keyboard — `C-t` then `x` — because a keyboard
has no notion of "still holding". A grid does. Holding a right-column action
button and tapping a pad has a property prefix mode lacks: **you can change
your mind by letting go.** For a surface where one of the gestures kills a
running agent, that matters more than the keystroke saving.

| gesture | does |
|---|---|
| tap unbound slot | spawn its spec |
| tap bound slot | jump |
| tap `▲` `▼` | scroll the grid one row |
| tap `◄` | **back** — return to where you were standing |
| tap `►` | **next** — jump to the longest-waiting amber |
| hold `▲` past 500 ms | **home** — scroll all the way back to row 1 |
| hold `Session` | the grid becomes a **map** of all 64 rows; tap a pad to go there |
| tap `Drums` | the grid becomes an **art**; `►` is the only way back |
| tap `>` (rows 1–7) | **launch** into that row |
| hold `CC19` + tap slot | **Stop** — terminate what is alive, forget what has ended |
| double-tap `CC19` | swap the corner between **Stop** and **Move** |
| hold `CC19` in Move + tap, tap | **move** a session from one pad to another |

**Mute is not on this list, and it is not gone.** It has no button —
`tally mute <addr>` is the whole of it — because it scored zero presses in a
day and the grid had no button to spare. It stays because it is the
starvation hatch for `►` and for the tempo: a prompt you will never answer
otherwise sits at the head of the queue forever, driving the breathing up
with it.

### The exact rules of a chord

Four sentences, all load-bearing, because between them they decide whether a
fat finger kills an agent.

1. **The modifier must be down before the slot goes down.** Tapping a slot
   and *then* grabbing the corner does nothing — the slot press already
   resolved as a jump. Order is not a detail here; it is the difference
   between a gesture and a race.
2. **The action fires on slot press-down, not release.** A chord is
   deliberate by construction (you are holding something with your other
   hand), so there is nothing left for a release to confirm, and firing on
   release would mean sliding off the pad silently cancels.
3. **How long you hold a *slot* means nothing.** A slow press and a fast one
   do the same thing, so there is no way to be holding a pad "wrong". The
   two timed gestures on this surface are both on **controls** — hold `▲`
   for home, double-tap `CC19` for the verb — and never on the grid.
4. **Releasing an action button without tapping is a no-op. Always.** This
   is the escape hatch that justifies hold-then-tap over a prefix in the
   first place.

Releasing the *slot* first, then the modifier, is the normal case and means
nothing extra. Pressing a second slot while still holding the modifier fires
the gesture again on that slot — the modifier is a mode for as long as it is
down, not a one-shot. **Two modifiers down fires nothing**, and the preview
disappearing says so.

**No key repeat, and on the grid no timing of any kind.** One tap, one
action, resolved at press-down.

### The held preview

While an action button is held, the grid **recolours to show what the
gesture would do**. The surface answers "what happens if I press this"
before you press it, which is the only honest way to put a destructive
action on a button with no label.

Three levels, and the middle one earns its keep:

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

| held | pad | colour | what the tap does |
|---|---|---|---|
| **Stop** | alive — working, quiet, wants-you | 5 red | terminate it |
| **Stop** | hiding ended rows | 7 dark red | forget **all** of them |
| **Move**, hand empty | holds a live session | 37 turquoise | pick it up |
| **Move**, hand full | in hand | 37 turquoise, flashing against dark | — |
| **Move**, hand full | holds a live session | 37 turquoise | nothing |
| **Move**, hand full | free, on the visible grid | 1 dark grey | put it down there |
| either | a session this gesture will not touch | 1 dark grey | nothing |
| either | nothing there | off | nothing |

Two rules hold that table together:

- **Each colour is the colour of the consequence, not of the button.** That
  is why `effect` takes no `Action` at all: one button carries two verbs, so
  a colour derived from the button would have to be wrong about one of them.
- **The preview and the press read the same function.** They did not always
  — a renamed spec once lit a pad green for a respawn that then did nothing,
  because the preview counted a label and the press resolved it. A preview
  that is wrong about one pad is a preview you stop believing about all of
  them, which costs more than the gesture it was wrong about.

The top row and the logo **do not** take part. Navigation and health keep
meaning what they always mean — a preview is a question about the grid, and
the health light is not allowed to answer a different one.

## The queue, and the two buttons that read it

The obvious question to build a layout around is *where is the blocked
agent*. The data says that is not the question: 61% of the time more than
one slot is asking, and when two or more are amber the gap between the
longest and shortest wait is a median of 386 s, max 1382 s. **Ordering is
not a tiebreak between similar things; it is the entire decision.**

```text
queue = slots whose state is NeedsAnswer
        minus muted
        minus the slot this client is already looking at
        ordered by the moment they started asking, oldest first
```

**Nothing is stored for this.** `Journal::asking` already maps a slot to the
moment it began asking and holds the *original* timestamp across a tallyd
restart ([04](04-signals.md#the-instrument)). No new label, no new file, no
new state in the daemon.

**Mute is the starvation hatch.** A prompt you are never going to answer
sits at the head forever. **The queue is per client**: which slot you are
already looking at differs between two attached terminals, so the head does
too.

`►` **crosses pages**, and scrolls the grid the smallest amount that brings
the pad it landed on into view — minimal, not centred, because `►` is
already moving your terminal and the grid lurching further than it must is a
second thing changing under you for no reason.

That is what makes losing the per-page amber indicator affordable: it told
you *which page* wants you so you could go there, and now attention
navigates to you. **Pages used, 70 : 2** — page 2 had been visited twice,
ever — so a rule about telling eight pages apart was a rule about a thing
that was not happening. If pages become load-bearing later, this is the
first decision to revisit.

### Back, and where you were

```text
Origin: client name -> pane id
recorded  when a client is about to be switched
          AND no trip is already open for it
          AND it is not already standing on the destination
one deep, first jump wins, cleared by `◄`
```

Three rules, each load-bearing:

1. **Per client.** A jump switches every attached client on purpose, so
   "where I was" is per client and never global. Two terminals jump to the
   same place from two different origins and both go home correctly.
2. **One deep, and the first jump of a trip wins.** `next → next → next →
   back` lands where you started, not three agents deep. **The queue is a
   trip; home is one press.** A stack would let you walk back through five
   agents, which is not a thing anybody has wanted while holding a grid.
3. **Staleness degrades to nothing.** If the origin pane is gone, `◄` goes
   dark and the press does nothing. It must not pick a nearby pane, the
   window the pane was in, or the last session: landing somewhere arbitrary
   is worse than not moving, because not moving is something you can
   understand from across the room.

**A view is a valid origin**, and that is the half that was got wrong. The
first rule said "and its current pane is not a view pane", assuming you jump
*from* nvim. Every pane this grid is used from is an anchor session, so
every candidate origin was a view, nothing was ever recorded, and `◄` was
dark forever. Caught within an hour of shipping by the `kind:"nav"` lines
added for exactly that purpose: two `{"via":"back","ok":false}` in a row,
which is what pressing a dead button twice looks like in a log. What that
rule was protecting is held by *first jump wins* instead, which does not
care what kind of pane you started in.

The origin lives in tallyd's memory, keyed by client name. A tallyd restart
loses it and the first press after jumps instead of returning. The
alternative — a global tmux user option per client — survives a restart and
is inspectable, and is rejected *for now* on cost: client names are paths
(`/dev/ttys000`) needing sanitising into option names, and the state leaks
past the process that owns it.

### What a navigation row costs the alternatives

**`►` cannot also be *commit the safe default*.** Approve-without-reading
has no button here, and narrowing it to the one answer safe to give unread
does not rescue it. Spending `►` on navigation retires the idea without a
further argument, because **navigation commits nothing**.

**There is no focus mode.** `▲▼` scroll, `◄` is back, `►` is next; none is
conditional on anything, so there is no mode to enter and nothing to escape.

**A tap on a pad keeps its one meaning**: jump. A second tap is not a return
and not a confirm — `◄` is the return gesture, on its own button, visible,
with an affordance.

## The overview

Every row the journal has ever seen a session in: rows 1, 2, 3, 4, 6, 8 and
**13**. Seven of sixty-four, and already discontiguous. Reaching row 13 from
row 1 is five `▼` presses across four rows of nothing, with no way to know
it is there.

**Sparsity is the argument for a map, not against it.** A dense 64-row
overview would be sixty-four lit pads and unreadable. Seven lit pads out of
sixty-four is instantly legible, and every lit pad is a destination.

**Hold `Session` and the grid becomes the whole address space: one pad per
absolute row, in reading order.**

```text
pad (r, c)  ⟶  absolute row (r - 1) × 8 + c

 ┌────┬────┬────┬────┬────┬────┬────┬────┐
 │  1 │  2 │  3 │  4 │  5 │  6 │  7 │  8 │   ← page 1
 │  9 │ 10 │ 11 │ 12 │ 13 │ 14 │ 15 │ 16 │   ← page 2
 │ …                                      │
 │ 57 │ 58 │ 59 │ 60 │ 61 │ 62 │ 63 │ 64 │   ← page 8
 └────┴────┴────┴────┴────┴────┴────┴────┘
```

**Tap a pad and the window lands there**: that row becomes the top of the
eight-row window, clamped at the bottom of the space.

One pad stands for eight slots, so seven-eighths of the information is
thrown away. What survives is **the worst thing in the row**: asking →
amber pulsing, producing output → green pulsing, alive and quiet → green
static, exited non-zero → red, lost → dark red, nothing → off. Same colours,
same meanings, no second language. *Which column* is deliberately lost: you
are choosing a row to stand in, and the eight pads of that row answer the
rest a half-second later.

Mute needs no rule here. A muted slot never resolves to `NeedsAnswer` at
all, so it cannot contribute amber to a row — for free, and for the same
reason it cannot reach the queue.

**Where you are standing** is marked by the only thing that costs nothing: a
pad inside the window that would be dark is dark grey instead. No new
colour, no new meaning. A pad inside the window that has content keeps its
true colour, because state outranks position. The limit, stated rather than
hidden: a window whose eight rows are all occupied shows no marker at all.
Survivable because it is rare — and because in that case every pad you are
looking at is lit, which is its own kind of answer.

**Momentary. Press to show, release to hide, tap a pad to go.** Chosen over
a latched mode for one reason: a latched overview is a mode, so the grid
would mean two different things depending on something invisible.

While it is held: `▲▼◄►` **stay live** (scrolling under the map slides the
grey marker across it, which is a free demonstration of what the marker
means); `>` ×7 go **dark and inert**, because grid row 3 means *page 3*
right now, not *row 3*, and a launcher that fires into the wrong row is the
one mistake that column must not make; holding the corner too is ambiguous,
so **neither fires** and the overview disappears.

Rejected: **a compacted list** — occupied rows only, packed from the top.
Fewer dark pads, shorter reach, and it is *the pads move while you reach for
them* wearing the sparsity argument as a hat. A map whose addresses are
stable is worth more than a map that is short. **A per-page map** — pages
stopped being a thing you navigate, and the measured usage crosses a page
boundary inside one region of interest.

### Home is hold-`▲`

`Session` cannot be two things, so `⌂` left it — and it did **not** get
folded into the overview, because the one jump the data describes, row 13
back to row 1, is worth a single press.

**`▲` scrolls one row on press. Held past 500 ms, it snaps to row 1.** There
is no tap-versus-hold ambiguity to resolve, because a tap is not withheld
waiting to find out: the scroll happens immediately and keeping the finger
down does more. Nothing is deferred, nothing is guessed, and you feel the
arrival because `▲` goes dark the moment you are there. It fires **once**
per press — autorepeat would arrive at the same row by a longer road and
need a rate nobody has measured.

`▼` gets no hold. The bottom of a 64-row space where seven rows are occupied
is not a destination, and the overview reaches the deep rows in one gesture.

**The clock does not go in `chord.rs`.** That module is pure — presses and
releases in, gestures out, no `now` — and that purity is what lets the four
chord rules be tested without a device or a timeline. The timer lives where
the clock already is: `tallyd`'s 16 ms loop. The consequence, stated so
nobody looks for it: `Gesture::Home` stays in the enum and `navigate` keeps
handling it, but **`chord.rs` never emits it**.

## The right column launches

The column used to be four hold-modifiers — kill, respawn, purge, mute — and
four dark. After a day of real use:

```text
jump           341      respawn   0
rows (scroll)   24      purge     0
next             4      mute      0
back             4
kill             4
```

**Respawn was not unused, it was unreachable**: it requires a resolvable
spec and there is no `slots.toml` on this machine
([D8](10-decisions.md#d8--slotstoml-as-the-spawn-path)). So the column was
one button doing work, one that could not, and two nobody wanted.

**The fact that changed the menu: the grid knows the directory now.** anchord
keeps `cwd`, `command`, `args` and `login_shell` per row, on dead rows too.
The reason spawning lived in a shell — *the shell knows the folder and a
config file cannot* — stops being true for any row that has ever held a
session. The row itself knows.

Seven scene-launch buttons, each beside a grid row, each a **tap**:

| | |
|---|---|
| **where** | the leftmost free column of that row |
| **directory** | inherited from the newest session in that row, whatever state it is in |
| **command** | that session's `command` and `login_shell` |
| **arguments** | **dropped** |
| **lit** | 1 dark grey when it would act; off otherwise |

**Arguments are dropped on purpose.** A row running `claude --resume
d37ee7e5…` describes one conversation; copying that would put a *second pad*
on the *same conversation*, which is the one thing a new agent must not be.
Keeping `command` keeps the choice that matters — a row running `isayes`
gets another `isayes`.

**It is a tap, not a hold.** The hold-then-tap grammar exists so you can
change your mind before something is destroyed. `>` destroys nothing; the
worst case is one `claude` you did not want, which is a kill away.

**Row 8 is shell-only, by rule.** Eight buttons, eight rows, and one must be
the modifier. Moving the modifier anywhere else means moving the one button
that is 8 mm from the wrong pad, and it has muscle memory in the corner the
silkscreen labels *Stop*. A destructive button that moves is how you get a
kill you did not mean. The cost is honest — a grid where one row behaves
differently is a grid you remember rather than feel — and it is affordable
only because the window scrolls: row 8 is one `▼` away from being row 7.

**Respawn is deliberately not here, in any form.** *Put back what was here*
is a real want, and `>` is not it: `>` gives you a **new** agent in the same
directory, deliberately dropping the `--resume` that would make it the old
one. Restoring a specific conversation stays `tally spawn <addr> -- claude
--resume <id>` — exact, visible, and already working. A button that guesses
which conversation you meant is a button that is wrong at the worst moment.

## The corner carries two verbs

There are no spare buttons. Two things want one and neither can have it, so
`CC19` carries both and **a double-tap toggles which**. That is not a trick:
on a Mini MK3 the bottom-right button is silkscreened **Stop Solo Mute**
because in Ableton it cycles modes by repeated presses. Novation put a mode
switch there and printed its menu on it.

| mode | at rest | what the hold means |
|---|---|---|
| **Stop** | 5 red | end what is running, forget what has ended |
| **Move** | 37 turquoise | put this pad somewhere else |

Stop is where it rests, because it has the muscle memory and the silkscreen.

### Stop does kill and purge

**The two consequences can never share a pad.** `alive` is working, quiet or
asking; `purgeable` is exited or lost. Disjoint sets, so every pad under the
hold has exactly one possible outcome and exactly one colour.

A tap on an ended pad forgets **every** ended row behind that address, not
the newest one — one tap, N deletions, and no way to delete one of them from
the grid. The grid does the common thing; `anchor` does the surgical one. It
takes the row's launch button with it: purge the last ended row in an empty
row and that row's `>` goes dark, because nothing left knows the directory.

**The cost, stated.** Purge's targets include pads that render dark: a clean
exit keeps its `tl.slot` and its log but draws as nothing, so a dark pad can
have a dozen rows behind it. Under the old kill-only chord the dark
two-thirds of the grid was a free miss. It is not any more. What makes that
affordable is that **the hold is what reveals them** — they stop being
invisible at the exact moment they become targets. A thumb one pad off
forgets a dead row's directory and log, which is a real regression priced at
one order below killing a live agent, and it is why purge gets the dark-red
*consequence* colour rather than kill's red.

### Kill is the one that needs the care

An agent mid-task is minutes of work. A fat finger is 8 mm from the wrong
pad. So kill is: hold a *different* button, on a *different* row, with the
target highlighted in red while you hold, and `anchor kill` underneath —
which is a `terminate` anchor already makes synchronous-to-terminal-state,
so the pad goes red at the moment it is actually dead rather than when we
asked.

No "are you sure" blink-and-confirm. A confirmation you give by pressing the
same pad twice is a confirmation you learn to double-tap through.

**A killed pad goes dark, not red**, and that needs a label to be true.
anchord records a terminate as an ordinary non-zero exit — there is no field
naming who asked for it, and there should not be: that is a client's
opinion, not the daemon's fact. So tally writes `tl.killed` on the session
immediately *before* the terminate, and a terminal state carrying it renders
as nothing.

That keeps the rule one sentence: **red means it ended in a way you did not
ask for.** Without it, the pad you just deliberately killed is
indistinguishable from an agent that crashed, and red stops meaning *go and
look*. Two consequences, both deliberate: `anchor kill <id>` from a shell
still goes red, because nothing told us — tally knows what tally did; and
dark is not forgotten — the row and its scrollback stay, because you often
kill an agent *because* it went wrong and want to read why.

### Move is the missing verb

Nothing else on this surface rearranges it. A shell spawn lands where you
typed, `>` takes the leftmost free column, and after a day the board is a
**log of where things happened to land** rather than a layout you chose.

**A move is one label write.** A pad's address is `tl.slot`, and `label()`
sets it atomically in one call. No process moves; the agent never learns it
happened. The view follows, from `tallyd`
([06](06-views.md#a-moved-session-takes-its-view-with-it)).

```text
hold Move          every pad holding a live session goes turquoise
tap a session      it starts flashing — it is in your hand
                   every pad it can land on comes up to a dim glow
                   the occupied pads stay turquoise, and do not move
tap a dim pad      it lands there
release early      nothing happened
```

Two taps inside one hold. No new grammar: `chord.rs` already emits one
`Chord` per tap and the pairing lives above it. The escape hatch survives
both halves — let go before picking up, or let go with something in hand,
and the board is unchanged.

| | |
|---|---|
| **what moves** | the drawn session, and only it |
| **what stays** | every ended row at the old address |
| **legal target** | a pad with no live session, on the visible grid |
| **across pages** | `▲▼` under a held modifier already works; the pad in hand survives the scroll |

**The history does not follow.** The ended rows behind the old pad stay
where they are, so the old row still knows its directory and keeps its `>`.
Dragging a session out of a row is not a claim that the row never existed.

#### Picking something up flips the question, not the board

The first version coloured the second half by *consequence* alone:
destinations turquoise, occupied pads dim. It read wrong, because it
repainted the board under the thumb that had just acted.

So **turquoise is not the verb, it is the fact**: a session is here. It
means that before the pick-up and after it, and the occupied board is the
one thing that does not move when your hand closes. What changes is the
*question*. Hand empty, you are reading for sessions, and the sessions are
the lit thing. Hand full, you are reading for room — and room comes up to a
**dim glow** on pads that were dark a moment ago.

**Dim, not bright**, and that is the second correction. Room was drawn white
first, on the reasoning that the thing you are looking for should be the
bright thing. It is not, because of a count: with a hand full, *most of the
grid* is a legal destination, and a board where two thirds is bright has
told you nothing. So the levels invert here, and only here: dim is the
target, and the only bright things are the two that are **not** room — what
is occupied, and what is in your hand.

The pad in your hand keeps the turquoise it already had and **flashes
against dark**. Hue says it is one of the occupied; motion says it is the
one you are carrying. It flashes against dark rather than a second colour
because dark is the one value on this grid that cannot be misread in this
moment: an occupied pad is turquoise and a destination is a dim grey that
never goes fully out, so the gap says *nothing*, which is exactly what a pad
that is no longer anywhere should say half the time.

### Changing the mode

**A double-tap on `CC19` toggles it.** Red ↔ turquoise, on the button, at
rest, all the time.

A *single* tap will not do, and the reason is worth writing down because it
was the first design and it was wrong: a bare tap on the corner is a press
and a release with no pad between them, which is **exactly the shape of an
aborted hold**. Chord rule 4 is the escape hatch the entire grammar rests
on; cycling on it would mean you abort a kill and silently arm Move. Two
taps cannot be confused with changing your mind.

| | | why |
|---|---|---|
| a tap | press and release inside 250 ms, no chord fired | a deliberate tap is ~100 ms; a hold long enough to read a preview is not |
| a double-tap | two taps inside 400 ms | long enough to be comfortable, short enough that two unrelated presses do not pair |

**It latches, and that is allowed here.** Nothing is invisible: the state is
the colour of the button that set it, and **the grid does not change until
you hold** — at which point it repaints entirely in the mode's colour under
your thumb, before anything can fire. You cannot act in the wrong mode
without first seeing the wrong colour.

**The clock stays out of `chord.rs`**, same rule and same place as hold-`▲`.
`tallyd`'s 16 ms loop owns both thresholds.

> **What the first press found.** The mode toggle froze the surface,
> immediately. A double-tap completes on the **release**, and `tallyd` flips
> the mode before `chord.rs` sees that release — so the button went down
> under one verb and came up under another, and a release matched by *verb*
> stranded the old one in `held` forever. `modifier()` then answered `Some`
> with nothing under the thumb: the whole grid sat in a held preview, in red
> or turquoise, and no sequence of presses recovered it. Fixed by matching a
> release on the **button** rather than the verb, which makes the pairing
> true by construction. No unit test would have found it — every test fed
> presses through `Chords` alone, where the mode never moves
> ([D9](10-decisions.md#d9--two-correct-modules-in-the-wrong-order)).

### What the corner deliberately does not carry

**Swap, when the destination is occupied.** Two label writes are two anchord
calls, and a failure between them leaves two sessions claiming one address.
An occupied pad stays turquoise and does nothing instead — it is *there*,
which is exactly why you cannot land on it. (This is about the *gesture*.
The tmux views **do** swap, which is a different layer:
[06](06-views.md#when-the-destination-is-live).)

**A third mode.** Two fit on a toggle. A third needs a cycle, a cycle needs
you to remember the order, and the next idea after that is a menu.

**Peek** — *what is this pad?* A real hole: the surface cannot name a pad,
and some share of 341 daily jumps are identification rather than work. It
needs a sink to print into and is the only candidate that needs new
machinery. Deferred, not dead; Stop's preview partly answers it for free by
showing every pad hiding history.

**Mute and respawn**, for the reasons above.

### The map does not learn about forgotten rows

The overview summarises what `resolve` draws, and `resolve` drops clean
exits — so a dark-but-purgeable pad contributes **nothing** to the map. Now
that Stop forgets dead stacks, the map understates exactly where those
stacks are. Left alone, deliberately: you can never see both (two modifiers
disarm the surface), and a dead row is not a destination — the overview
exists to put the window somewhere, and lighting a row whose only content is
forgettable history would be the map answering a question it was not asked.

## The arts

Sixty-four pads doing something that is not work. Two reasons, and the
second is the real one.

**It is a demo.** The pitch is "the agents you are running are a physical
object on your desk", and that pitch is made by a photograph. A grid of
eleven lit pads is a true picture of a working day and a bad picture of what
the hardware can do.

**It is a pixel test nobody has to remember to run.** Every piece is a
*total* function of the surface — all 64 pads, every frame. A dead pad, a
mis-flipped row, a diff that stopped converging, a palette entry that is not
the colour the manual says: all visible inside two seconds of plasma, and
none visible on a grid where eleven pads are lit and fifty-three are
correctly dark.

One button gets you in. After that the surface is a **menu**, not a grid:
`Drums` holds the piece you arrived on and the eight launchers hold the
rest, one each, in reel order.

```text
  Drums  ─ Bestiary        > 1 ─ Claude      > 5 ─ Fire
                           > 2 ─ Plasma      > 6 ─ Tetris
  ►      ─ back to work    > 3 ─ Life        > 7 ─ Pacman
                           > 4 ─ Rain        > 8 ─ Uprising
```

**The lit one is the one playing.** White on the piece you are watching, dim
on the eight you are not. The button you pressed to get in is the button
that is lit when you arrive — a mode that opened by highlighting some other
button would be lying about itself in its first frame.

**`►` is the only way out.** It is lit white for as long as an art is
playing, and it still goes where it was always going: if something is
waiting, it pulses amber instead and one press both ends the art and takes
you to the pad that wants you. Making that two presses would be the toy
charging rent on the one gesture this product exists to provide.

**Everything else is swallowed.** A pad press lands on a sine wave, not a
session, so it is eaten rather than acted on. You left art mode by *reaching
for the thing you actually wanted*, and the pad under your finger a quarter
second ago was a sine wave. Acting on it would be acting on a grid the eye
never saw.

*Press*, not *gesture*, and the distinction is load-bearing. It is read from
the raw button before both the corner's double-tap clock and `chords.feed`,
because `feed` answers `None` for a held modifier and for the release half
of a double-tap. Routing on the gesture instead left `CC19` able to reach
that clock and toggle Stop/Move behind a plasma field — a mode change with
nothing on the surface able to report it, discovered only on coming out
([D9](10-decisions.md#d9--two-correct-modules-in-the-wrong-order)).

**What the art is not allowed to hide: the amber.** It is the signal the
whole thing is a bet on, and a toy that can swallow it is a defect. So the
logo tells the truth, unchanged, at full brightness — and while the grid is
a plasma field, *all* attention is attention you cannot see — and `►` stays
lit when something is queued. Everything else goes dark. The art owns the 64
pads; it does not own the three lights that report.

### The nine pieces

| | |
|---|---|
| **Bestiary** | cat, dog, fish, bird, rabbit — three seconds each, two poses apiece, because a sprite that does not move is a logo and the grid already has one. The eyes are a second sprite drawn over the first, which is why `draw` paints set bits and leaves everything else alone |
| **Claude** | the mark, blooming open from a lit core and then turning — orthogonal, mark, diagonal, mark. Nothing on 8×8 can rotate by less than 45°, so the full sunburst between the two positions is the frame that sells the turn. **The diagonals stop short of the corners**: that taper is the entire difference between the mark and a `*` |
| **Plasma** | four sines summed per pad onto the bright-hue column. No state; frame *n* depends on nothing but *n*, so it cannot drift, stall or need reseeding. Integer arithmetic against a 64-entry table, which makes it a pure function the tests assert exact frames from |
| **Life** | Conway on an 8×8 torus. **Heat, not occupancy** — a dying cell cools through the ramp, so what you see is the glider's wake. **It reseeds itself**: a small torus falls into a still life within a few hundred generations, and a frozen grid is indistinguishable from a crashed daemon |
| **Rain** | eight drops, one per column, each leaving a tail that outlives it. A drop moves at most fifteen sixteenths of a pad per frame so it can never skip one — which is what lets the head light a single pad instead of interpolating a streak |
| **Fire** | Doom's fire, eight pads wide: *each cell is the one below it, minus a little, blown sideways a little*. The floor is held at full heat with seven rows above it, each losing up to two, so a flame reaches the ceiling only when the dice are with it. It propagates into a fresh board — reading from the buffer you are writing carries the floor to the ceiling in one frame |
| **Tetris** | self-placing: a piece picks rotation and column at spawn by the smallest scoring function that plays recognisably, then falls straight in. **It does not steer on the way down** — a piece sliding sideways mid-air reads as a bug on a grid this small, and the thing worth watching is the stack |
| **Pacman** | four mouth positions, because the mouth goes back the way it came and that is what a chomp is; two foot positions for the ghost, which is the whole reason it reads as a ghost rather than a bell. **A different ghost each time round** — Blinky, Pinky, Inky, Clyde, in the arcade's order. One modulo, and it is the detail that makes somebody look twice |
| **Uprising** | the joke, and the only piece *about* tally: one green pad, then eight, then forty; one turns amber, then all of them; one turns red, then all of them; a beat of dark; a yellow smile. The colour language telling a story about itself — in its current dialect, with the green **static**, because motion belongs to the amber now. It ends on the one palette entry the product never uses |

**Sprites are `[u8; 8]`, one byte a row, bit 7 is column 1.** That is the
only reason this is not a `u64`: written as binary literals, the source *is*
the picture. A sprite you cannot see in the source is a sprite nobody will
ever fix.

```rust
0b0100_0010,   //  . # . . . . # .    ears
0b0110_0110,   //  . # # . . # # .
0b0111_1110,   //  . # # # # # # .    head
```

The Uprising's timing is asserted in **seconds**, not frames, and that is a
test because a human found the bug: the first cut spent 17.4 s getting to
the smile, every test was green, the renderer drew the face correctly at
frame 759 — and the first person to press the button never saw it, because
nobody holds a button they have just discovered for eighteen seconds.
Seconds are the unit the complaint was made in.

**Where it lives.** `src/art/`, pure: a `Piece` and a frame counter in, a
`Frame` out. It does not know what a session is, that anchord exists, or
that the pads it is painting usually mean something. One file per piece,
plus `rng`, `colour` and `sprite` for what they share. Nothing in
`src/render.rs` changes — the colour language has one owner and the art is
not it.

The art ramp is **not eyeballed, and not verified either**: the twelve
entries in `surface::palette` each carry *confirmed against a real device*;
the fifteen bright hues the art uses are read off the manual's table and
nothing more. For a colour that means something that would be unacceptable;
for a plasma field it is fine, and no test asserts what colour it *looks*.

**The frame rate** is ~31 fps — the loop already wakes every 16 ms and art
advances every other wake. At most 200 bytes a frame, on a wire that took a
hundred back-to-back 328-byte worst-case frames without choking. The diff
does the rest: plasma changes most pads and pays for them, Life's steady
state changes a handful, and the Uprising's holds send nothing at all.

**What it does not do.** It does not run by itself — no screensaver, no idle
timeout, no art after N quiet minutes. A grid that starts animating on its
own is a grid you have to check before you trust, and the whole product is
the claim that you do not. It is not interactive: no paint mode, no Pong;
every press exits, and that rule is worth more than a toy. And it does not
touch anchord — art mode spawns nothing, kills nothing, and reads no session
list.
