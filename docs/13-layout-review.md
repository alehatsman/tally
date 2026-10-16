# 13 — Review of the layout proposal, and a smaller one

**Status: review, and a narrower counter-proposal.** It reads
[12](12-layout-proposal.md) against `01`–`11` and the hardware notes, keeps
what survives, and cuts what does not. Decisions taken here are marked
**decided**; everything else is named and left open. Nothing in `01`–`12` is
amended by this file.

`12` asked the right question — *the top row spends four silkscreened arrows
on a problem nobody has* — and then answered it with more surface area than
the question needed.

## The verdict

Three ideas survive, four do not.

| from `12` | verdict |
|---|---|
| `>` spawns into this project's row | **keep** — adjacency is a real ergonomic win |
| answer the prompt from the grid | **keep**, narrowed — see [§5](#5-answering-from-the-grid-is-the-best-idea-here-and-the-most-dangerously-framed) |
| rows are projects, columns are lanes | **keep**, with four repairs — see [§4](#4-rows-what-they-buy-and-what-they-cost) |
| the cycling verb key | **cut** — it breaks a rule we already paid for |
| Drums as sixty-four canned prompts | **cut** — a closed non-goal, reopened weaker |
| Solo | **cut**, or momentary only |
| project-scoped Mute | **cut** — not implementable under `03`'s invariant |

## 1. The argument against pages defeats itself

`12`'s first and heaviest argument is that **a page is hidden modal state —
a mode you can be in and forget.** The argument is correct. The replacement
then ships four mode axes where pages had one.

| | mode axes | indicator |
|---|---|---|
| today | page index | eight dedicated buttons, exactly one bright, always visible |
| `12` | scroll offset, armed verb, focus, Drums | one small key's hue, a dimmed row, one top-row light |

Pages were *one* mode carrying **the best indicator on the device** — the
rule from [05](05-interaction.md#the-two-rows-that-are-not-slots) that cost
a rebuild to learn: *exactly one thing is bright and it is where you are.*
A design that objects to modality and then distributes four weaker-indicated
modes across the surface has lost on its own criterion.

`Session` existing to "clear focus and any armed verb" is the confession.
You do not need an unconditional escape when there is nothing to escape.

The scroll is not exempt either. A window showing projects 4–10 means row 1
is project 4, and you must *remember* the offset to read the grid. That is a
page index with a worse name. It feels lighter only because the design
quietly assumes you have seven projects or fewer — in which case ▲▼ are
spent on a path that almost never fires, and "every legend earns its keep"
has bought an honest idle button rather than a used one.

**What survives the argument:** modality is the right thing to count, and
the count is the test. The layout in [§8](#8-the-layout) has one mode.

## 2. The verb key — decided: four hold-modifiers

[05](05-interaction.md#kill-is-the-one-that-needs-the-care) already wrote
the rule this breaks:

> A confirmation you give by pressing the same pad twice is a confirmation
> you learn to double-tap through.

Killing one agent under `12` is: tap `CC19` to arm, release, hold `CC19`,
tap the pad. **Press the same button twice to kill** — the exact pattern
`05` rejected, and worse than what it rejected, because the second press
confirms *a mode* rather than *a target*. That is friction carrying no
information.

Three further failures, descending:

- **The timeout inverts the action.** Arm Stop, get pulled away for six
  seconds, come back, press `>` on row 3 — you do not kill that project,
  you **spawn into it**. A mode that reverts silently is a mode you learn
  about from the consequence, and you were looking at your screen, not at
  the one non-blue key, which is the premise of the whole device.
- **Bulk stop has the least protection of any gesture on the surface.**
  Stop armed, tap `>`, and up to eight running agents die with no held
  preview and no red target. [05](05-interaction.md#kill-is-the-one-that-needs-the-care)
  gives killing *one* agent a different button, a different row, and the
  target red while you hold. The gesture that kills eight gets none of it.
- **It puts a clock back on the input path.** [05](05-interaction.md#what-a-press-does-not-do)'s
  "no timing of any kind" was bought by deleting the title scroll. *"On a
  control, not on the grid"* is a lawyer's distinction — the rule exists so
  that nothing on this device behaves differently depending on **when** you
  touch it.

**Decided.** `12`'s own fallback gestures at the answer and stops one step
short. Go the whole way:

```text
CC19   kill      hold + tap        (unchanged — muscle memory, isolated corner)
Drums  respawn   hold + tap
Keys   purge     hold + tap
User   mute      hold + tap
```

No modality, no timeout, no clock, nothing deleted, and
[05](05-interaction.md#the-exact-rules-of-a-chord)'s four chord rules apply
unchanged — including the one that justifies the scheme: **releasing without
tapping is a no-op, always.**

The legend match is *better* than Stop/Solo/Mute, not worse. Drums, Keys and
User are the three **Custom mode** buttons: the thing silkscreened on them
is literally *the user defines these*. Nothing on the surface is more
honestly spent than a user-defined verb on a button labelled user-defined.

## 3. Two shipped gestures were deleted by accident

`12` states: *"Kill, mute and jump each burn a dedicated right-column button
today."* Jump is a bare tap, and the right column holds **four** modifiers
([05](05-interaction.md#gestures)): kill `CC19`, **respawn** `CC29`,
**purge** `CC39`, mute `CC89`. The proposal reassigns `CC29` and `CC39` to
scene launch and offers Launch/Stop/Solo/Mute in exchange — so respawn and
purge have no home, and the sentence that should have caught it miscounts.

Purge is not cosmetic. Failed and lost rows persist by design
([03](03-model.md#exited-rows-accumulate-and-purge-is-the-answer)), so
without it the rows fill with red — degrading exactly the *amber anywhere in
row 4* legibility that is the proposal's stated payoff.
[§2](#2-the-verb-key--decided-four-hold-modifiers) fixes this for free.

## 4. Rows: what they buy, and what they cost

**The benefit, stated accurately.** Today you see *full* state for one
project and *one bit* — the page button — for each of the other seven. Rows
give you full state for every project at once. That is real, and it is the
best thing in `12`.

**The claimed benefit is a wash.** *"Amber anywhere in row 4 means the
fourth project is blocked"* — counting to the fourth row of eight unlabelled
silicone pads is the same operation as finding the fourth amber-pulsing page
button in a row of eight. Rows 1, 2, 7 and 8 read off the physical edge; 3
through 6 are "middle" in both designs. This is a **perceptual claim**, and
this project has now been wrong about three of them — the three greens, the
hard amber flash, the title scroll. See
[§10](#10-the-experiment-that-gates-all-of-this).

Four repairs, and the layout does not work without them.

### 4.1 Rows are derived, not declared — decided

`12` has `slots.toml` name the project list, so a project only has a row if
it was predicted. That is the half
[M5](../ROADMAP.md#m5--tally-spawn) already found to be the wrong one:

> `slots.toml` wants the folder declared in advance, which is a prediction
> about work you have not done. **The only thing that knows the directory is
> a shell standing in it.**

The config is empty in practice and spawning is ad hoc, so a layout that
requires a declared project per row requires exactly the planning that does
not happen. The claim that the file "shrinks to a list of projects" is
relocation, not removal.

**Derive the rows instead:**

```text
project          = basename of the session's cwd, unless slots.toml pins a name
a project has a row if it has any session the grid draws
                   (live, failed or lost — the same predicate as model::resolve)
row order        = pinned projects first, in file order
                   then the rest, by the start time of each project's oldest
                   drawn session, oldest first
```

A project therefore **holds its row for as long as anything in it is alive**,
and the order is a pure function of one `anchor list` — so
[08](08-lifecycle.md)'s claim that tallyd holds nothing survives intact.
Config becomes optional pinning rather than a precondition.

The address stays a **name**, not a row index: `tl.slot = "tally:3"` is
`project:lane`, and the row the project occupies is a *rendering* decision
recomputed every tick. That is what lets the order drift without rewriting
labels, and it keeps the string typeable into a shell script, which is the
property [03](03-model.md#the-address-pinned) pinned it for.
Validation is unchanged in spirit: `name:1-8`, and anything else is
**ignored, not clamped and not repaired**.

### 4.2 Lane 1 is durable; the swarm allocates from lane 2 — decided

`12` makes lane 1 "the long-lived session for that repo" by convention and
then has `>` allocate the *leftmost free lane*. A clean exit renders dark
and frees its lane ([03](03-model.md#lifecycle-of-a-slot)), so the first
time the long-lived session finishes, the next `>` drops a throwaway into
lane 1. The convention and the allocator disagree on day two.

So `>` allocates **the leftmost free lane ≥ 2**. Lane 1 is claimed
deliberately — `tally spawn <project>:1` — or not at all. That also keeps
one durable place per project, which matters more than it looks:
[03](03-model.md#a-slot-is-not-a-session) opens by arguing *a grid you
cannot learn is a grid you look at instead of feel*, and auto-allocation
trades away learnability for seven-eighths of the surface. One durable
column per project is the smallest thing that keeps the sentence true.

### 4.3 A full row is reachable, so `>` has to say so

Measured peak is **five to eight agents in one repo**. Eight lanes is
therefore *exactly* big enough and has no slack — a full row is a normal
Tuesday, not an edge case. `12` has no answer for pressing `>` on a full
row.

Do not solve it by scrolling lanes. A second scroll offset is a second mode
([§1](#1-the-argument-against-pages-defeats-itself)) bought to handle a case
that a dark button can state for free. **The `>` column becomes the project
directory** — which also gives the rows the landmark they lack, because the
right column sits against the physical edge and is countable in a way the
middle of the grid is not:

| `>` | means |
|---|---|
| off | no project at this row |
| 1 dark grey | project here, **row is full** — `>` will refuse |
| 45 blue | project here, a lane is free |
| 9 amber, pulsing | this project wants you — wins over both of the above |
| 3 white | held |

Blue and white are [05](05-interaction.md#the-two-rows-that-are-not-slots)'s
control colours, unchanged. Amber on a control is not new either: the page
row already does it, and M4 recorded the reason — *a session reports itself
twice, deliberately*, because the thing you read from across the room and
the thing you press are not the same thing.

Dark grey for "exists, not a target" is lifted straight from
[the held preview](05-interaction.md#the-held-preview), where it already
means exactly that.

### 4.4 One impatient finger now produces the orphan race

[11](11-cli.md) accepts that two racing spawns both succeed and newest-wins
hides the loser, on the grounds that it takes *"you and a script in the same
second."* Auto-allocation removes that protection: `>` computes the free
lane from a list that is up to 2 s stale, so **three taps in 500 ms is three
sessions allocated the same lane**, two of which become running agents in a
real repo that no pad shows, no chord reaches and no amber can report.

tallyd must hold an ephemeral reservation — a lane is spoken for for a few
seconds after a spawn. This is allowed under
[08](08-lifecycle.md)'s rule precisely because **losing it is harmless**: a
restart forgets the reservations and the worst case is the status quo. That
is the opposite of the mute case, where state that evaporates is worse than
no state at all ([03](03-model.md#where-the-binding-lives)).

## 5. Answering from the grid is the best idea here, and the most dangerously framed

The diagnosis is right and it is the most valuable sentence in `12`: triage
and approval are the bottleneck, and they currently have zero buttons. The
mechanism is right too — `tmux send-keys` to the pane rather than the PTY,
so tallyd takes no lease and anchor needs no new seam.

**Its stated limitation is overstated.** `12` says a blocked agent you have
never jumped to has no pane, so the first press must still be a jump.
But [06](06-jump.md) has spawning create the view window *with* the
session, and tallyd can create a detached one — `new-window -d`, no
`switch-client` — without moving your eyes at all. The open question
*"cheap or expensive"* resolves to **cheap**.

**Its real problem is the one it does not state: this is
approve-without-reading, by construction.** The pad says *that* something
wants you and never *what* — [05](05-interaction.md#the-title-scroll--tried-and-abandoned)
proved it on hardware and [04](04-signals.md#what-we-deliberately-do-not-do)
states it as policy. So pressing ► sends Enter to a prompt you have not
read. `12`'s own worked example is a `Bash(rm …)` prompt, which is the worst
case it could have picked, and it cites herdr's *"approval is never
automatic, by policy"* as motivation two sections earlier. A button that
answers an amber light rather than a command has made approval automatic in
everything but name.

One detail is also simply wrong: ◄ → `Esc` is not *"no, don't."* Esc cancels
the operation; declining with a reason is a list selection plus typed text.
The honest claim is **one press answers a default-accept prompt, and
anything needing words still needs the keyboard** — not "a yes/no costs one
press."

**Decided — navigate, and commit only the safe default:**

| key | focused | why |
|---|---|---|
| ▲ ▼ | move the selection in the prompt | navigation commits nothing |
| ◄ | leave focus, back to the grid | *no* means "leave this alone", never `Esc` to the agent |
| ► | commit the selection — **the continue / no-op case only** | the one answer that is safe to give without reading |

Focus is entered by tapping an amber pad and left by `Session` or ◄, the
row dims, the four arrows light white, and a second tap still jumps
— so the press that jumps stays available on every bound slot, which is
`12`'s rule and a good one.

**Rejected while writing this: answer-and-advance.** Committing an answer
and auto-focusing the next amber pad is the obvious inbox move and it is the
hazard above with a multiplier — you press ►, focus moves, you press ►
again out of rhythm, and you have approved a prompt that was never on your
screen. Focus moves because you pressed a pad. Never because the grid
decided.

## 6. The three cuts

**Drums as sixty-four canned prompts.** It repeats the error `12` opens by
diagnosing — *capacity for a workload of about twenty* — at a factor of
roughly ten. Nobody has sixty-four canned prompts. More seriously, a per-pad
text string fired at a session is *"let me bind arbitrary shell commands to
pads"* with a different payload, and [01](01-scope.md#how-to-reject-a-feature)
rejected that with an argument that applies verbatim: **the configuration
surface it needs is larger than the rest of this program.** Four to six
prompts behind `User` later, if a week of use asks for them. Not sixty-four,
and not now.

**Solo.** It bundles a jump and a display filter under one word, and the
filter is anti-correlated with the product: *dim every other row* hides the
sixty-three pads [05](05-interaction.md#the-colour-language) says the grid
exists for. Held-Solo — see only this project while my finger is down — is
harmless and carries no mode, and is the only version worth keeping.

**Project-scoped Mute.** Not implementable under the current invariant.
`tl.mute` lives on the session and dies with it, deliberately
([05](05-interaction.md#gestures): *it was that agent you stopped believing,
not that pad*). A row mute either means "mute the sessions currently in this
row" — honest, and it will not cover the next spawn — or it needs state
inside tallyd, which breaks [08](08-lifecycle.md). Say which; do not ship
the ambiguity.

## 7. What the device still has that neither layout uses

`12` asks whether we are spending the surface well and answers by
rearranging what the pads mean. Three documented capabilities are spent on
nothing at all:

- **Global brightness** ([02](02-surface.md#other-messages-worth-knowing),
  `08h`). One message, no model change, and it does not compete with the
  colour language because it is orthogonal to it. Dim the whole surface
  while nothing wants you; full brightness the moment something does. A free
  signal dimension currently set once and forgotten.
- **LED sleep** (`09h`). The same idea overnight.
- **MIDI clock — the interesting one.** [02](02-surface.md#colour) records
  that both animations run at **120 bpm *when no MIDI clock is supplied***.
  Supply one and the host owns the breathing rate, still on the device's
  clock, still with no repaint loop and no drift. That answers a question
  neither layout can currently answer: **how long has it been waiting?**
  Breathe slower when everything is calm, faster as the oldest unanswered
  attention ages. No new colour, no new concept, no new state. The clock is
  global, so it expresses the aggregate — which is the right scope for the
  thing you read from across the room.

  ⚠️ **Unverified.** This is the manual's word, and
  [09](09-testing.md#what-a-green-suite-does-not-prove) is the standing
  reminder that a surface fact only shows up on the surface. It is one
  message to try.

That is a larger ergonomic gain than any rearrangement of what the grid
means, and it costs nothing the design has to agree about first.

## 8. The layout

```text
  ▲      ▼      ◄      ►    │ Session │ Drums  Keys   User │  ◈
  └─ scroll ─┘  └─ focus ─┘ │  top    │ resp.  purge  mute │ health
                            │         │ └─── hold+tap ───┘ │
┌──────────────────────────────────────────────────┐  ┌──────┐
│                                                  │  │  >   │ ×7  spawn into
│   row = project   (derived, not declared)        │  │  >   │     the leftmost
│   col = lane      (1 durable, 2+ the swarm)      │  │  >   │     free lane ≥2
│                                                  │  │  >   │
└──────────────────────────────────────────────────┘  ├──────┤
                                                      │ kill │ hold + tap
```

| control | no focus | focused |
|---|---|---|
| ▲ ▼ | scroll the project window — **dark unless there is something to scroll** | move the selection in the prompt |
| ◄ | dark | leave focus |
| ► | dark | commit the safe default |
| `Session` | return the project window to the top | leave focus — the one unconditional escape |
| `Drums` `Keys` `User` | respawn / purge / mute, hold + tap | unchanged |
| `>` × 7 | spawn into this project; colour per [§4.3](#43-a-full-row-is-reachable-so--has-to-say-so) | unchanged |
| `CC19` | kill, hold + tap | unchanged |
| `◈` | health, plus amber for an off-window project that wants you | unchanged |

Two buttons are dark much of the time, and that is the correct trade: `12`
is right that **an unassigned button is better than a strained one**, and
the rule applies to the arrows as much as to `Keys`.

**One mode** — focus — entered by a deliberate press, indicated by a dimmed
row and four white arrows, left by either of two buttons.

### The scorecard, honestly

| printed | native | here | honest? |
|---|---|---|---|
| ▲ ▼ | across scenes | scroll projects / move the selection | yes |
| ◄ ► | across tracks | leave / commit | **thin — the weakest pair here** |
| `>` | launch a scene | launch an agent into this project | nearly literal |
| Stop/Solo/Mute | cycle the verb | **kill, hold + tap** | partial — one of three words |
| Session | return to the overview | window to the top, leave focus | yes |
| Drums / Keys / User | *modes the user defines* | *verbs the user defined* | literal |
| ◈ logo | brand | health light | neutral |

It scores worse than `12` on two cells and better on one, which is the point
worth stating plainly: **the silkscreen is a discoverability argument, and
discoverability is a first-hour property.** Ergonomics is what the same hand
does on the four-hundredth press. Where the two disagree, the hand wins.

## 9. What this costs, and what it contradicts

1. **It still reopens [01](01-scope.md)'s "no input from the grid"**, and
   narrowing the feature does not close that argument, it only bounds the
   damage. `tmux send-keys` keeps anchor's one-writer invariant intact
   ([06](06-jump.md#who-holds-the-lease)), so the contradiction is with the
   scope doc, not with the architecture — but it is a contradiction and
   `01` has to be amended rather than quietly outvoted.
2. **It still deletes pages**, which met their exit criteria on hardware
   ([M4](../ROADMAP.md#m4--pages)). Cross-page attention goes with them and
   comes back smaller on the logo.
3. **It changes the address**, `page:row:col` → `project:lane`, which is
   visible to anything reading anchor's labels.
4. **It changes `tally spawn`.** The address becomes `[project:]lane` with
   the project defaulting to the basename of `$PWD` — which is more in the
   spirit of [11](11-cli.md) than what it has now, since the whole document
   exists because *the shell already knows*.
5. **Seven projects, not eight.** `CC19` is kill, so row 8 has no scene
   button. Recommend seven drawn projects and a scratch row, as `12` does.

## 10. The experiment that gates all of this

Not the open questions `12` lists. This one, and it costs ten minutes:

> **Light row 4 amber. Stand where you stand. Say "four" without counting.**

The entire rows-as-projects payoff rests on that being possible, it is a
perceptual claim, and three perceptual claims in this project have already
gone the other way after being obvious on paper
([05](05-interaction.md#why-it-is-one-sentence),
[05](05-interaction.md#nothing-blinks),
[05](05-interaction.md#the-title-scroll--tried-and-abandoned)). If it fails,
[§4.3](#43-a-full-row-is-reachable-so--has-to-say-so)'s `>` column is doing
all the work and the grid is only grouping, not signalling.

And one question behind it, which a week of `attention.jsonl` can answer for
free:

> **Do you need to know which project is blocked *before* you jump?**

You were going to press the amber pad either way, and you arrive in the
right window either way. The benefit is real only for **priority triage** —
answering the work repo before the side project. If the honest answer after
a week is *"I press the amber one and go"*, then pages cost nothing, the
grouping argument collapses, and the top row should be spent on
[§5](#5-answering-from-the-grid-is-the-best-idea-here-and-the-most-dangerously-framed)
and [§7](#7-what-the-device-still-has-that-neither-layout-uses) instead.

## Still open

- **▲▼◄► are thin.** Four arrows, two jobs, both conditional. Better than
  four arrows on a scroll nobody uses, not obviously better than pages.
- **Does `01`'s "no input" get amended, or does answer mode stay out?**
  [§9.1](#9-what-this-costs-and-what-it-contradicts). This is the decision
  the rest of the layout is least coupled to — everything above works with
  the arrows dark.
- **One-way address migration, or coexist for a milestone?** Recommend
  one-way: the labels are written by scripts you control, there are a few
  dozen of them, and a surface that reads two address formats is a surface
  with two ways to be wrong.
- **Derived row order when nothing is pinned.** "Oldest live session first"
  is stable while a project is alive and arbitrary across a quiet morning.
  Whether that is good enough, or whether a pin is required in practice, is
  a week-of-use question.

### Searched and rejected

Recorded because the design space is small enough to state, and two of these
look better than they are:

- **Row = urgency band** (top row wants you, middle is working, bottom is
  dead). Self-sorting, always one row to look at — and **the pads move while
  you reach for them.** You cannot build muscle memory against a target that
  relocates, and a mis-press is a kill.
- **No grouping at all**: sixty-four slots, flat, first-come. For twenty
  sessions this is nearly fine, and it is the honest baseline the grouping
  argument has to beat. It loses only on priority triage — which is exactly
  what [§10](#10-the-experiment-that-gates-all-of-this) asks you to measure
  before believing.
- **Column = project, row = lane.** Symmetric and worse: the right column is
  adjacent to *rows*, so `>` stops meaning anything and the one unambiguous
  win in `12` is lost.
