# 23 — Every gesture we have, and the three we do not

**Brainstorm, not design.** Nothing here is settled and nothing here is
scheduled.

> ⛔ **Part 4's wall was wrong, and [25](25-the-squad.md) measured it
> down.** *"Broadcasting a goal to four already-running agents is out of
> scope by construction"* assumed typing into a session needs anchord's
> write lease. It does not: the bytes go to a **tmux pane**, which
> forwards them to the attach client a keyboard would have used, and
> `tallyd` is not in the path at all. `25` also spends `Top(8)` — the
> button Part 5 expected to stay free — on `User + >`, a squad in a row.
> The inventory below is otherwise current. It exists to answer three questions that arrived together —
*fork a session from a session*, *sync two sessions*, *spawn four and give
them a goal* — and the only honest way to answer them is to first write
down what the surface already spends, what it has left, and which of the
three the architecture forbids outright.

## Part 1 — The inventory

### The physical budget

81 addressable positions, and **that is the whole input device**. No
velocity, no aftertouch, no pressure: the Mini is unweighted, press is
press ([02](02-surface.md)). Every gesture below is built from *which
button*, *down or up*, and *time* — and time is the one the surface has
spent a document rejecting.

### What each position does today

| position | resting | tap | held |
|---|---|---|---|
| grid pad, unbound | off | spawn its spec | — |
| grid pad, bound | green / amber / red / dark red | jump | — |
| `Top(1)` `▲` | dark grey if somewhere to go | scroll up one row | **500 ms → `⌂` home** |
| `Top(2)` `▼` | dark grey if somewhere to go | scroll down one row | — |
| `Top(3)` `◄` | white if an origin is recorded | back to where you were standing | — |
| `Top(4)` `►` | amber pulsing if anything waits | jump to the longest-waiting amber | — |
| `Top(5)` `Session` | dark grey, always | — | **the overview**: grid becomes 64 rows, tap = goto ([20](20-the-overview.md)) |
| `Top(6)` `Drums` | dark, white while playing | next art / back to the grid ([21](21-the-arts.md)) | — |
| `Top(7)` `Keys` | **off** | **nothing** | **nothing** |
| `Top(8)` `User` | ~~off~~ | ~~nothing~~ | **squad in a row, with `>`** ([25](25-the-squad.md)) |
| `Action(1..7)` `>` | dark grey if the row can inherit | launch into that row ([18](18-the-launch-column.md)) | — |
| `Action(8)` `Stop` | red | — (double-tap reserved for Move, [22](22-the-mode-button.md)) | **kill alive / purge ended** |
| `Logo` | health light | — | — |

Off the surface, in the shell: `tally spawn`, `tally purge`, `tally mute`,
`tally attention` ([11](11-cli.md)).

### The grammar, as primitives

Five, and only five, exist today:

1. **tap** — resolved at press-down, no timing, no repeat, no double-tap.
2. **hold-then-tap** — a right-column modifier down *before* the pad; fires
   on pad-down; multiple pads per hold; release without tapping is a no-op.
3. **hold-with-timer** — `▲` past 500 ms. Lives in `tallyd`'s 16 ms loop;
   `chord.rs` stays pure and clockless.
4. **hold-as-mode** — `Session` down repaints the grid into a different
   address space. Momentary, never latched.
5. **pick-up-and-drop** — two taps inside one hold, the first selecting and
   the second landing. **Designed in [22](22-the-mode-button.md) for Move
   and not yet built.** This is the most important line in the table: it is
   the only primitive that can express a gesture with *two operands*, and
   all three requests below have two operands.

Plus two rules that are really one: **two modifiers down fires nothing**,
and **releasing a modifier without tapping fires nothing**.

### The rules a new gesture has to survive

- The four chord rules ([05](05-interaction.md#the-exact-rules-of-a-chord)).
- **No timing on the grid.** The last threshold there was deleted with the
  title scroll. Timers are allowed on *buttons*, not on pads.
- **Colour budget is full.** Green, amber, red, dark red, white, dark grey,
  turquoise. A new verb that needs a new hue is a new language to remember,
  and the three-greens failure is the record of what that costs.
- **tally is an observer.** It never holds the control lease
  ([01](01-scope.md)). It cannot type into a running session. This kills
  one of the three requests outright — see C below.
- **No input.** The grid cannot carry text.

### The capacity ledger, stated plainly

| free | notes |
|---|---|
| `Top(7)` `Keys` | a whole button, tap or hold |
| `Top(8)` `User` | ~~a whole button~~ — **spent** by [25](25-the-squad.md) |
| `Action(8)` third mode | [22](22-the-mode-button.md) refuses it: a cycle needs an order to remember |
| pick-up-and-drop | one grammar, reusable by any verb that has a source and a destination |

Two buttons. That is the budget for everything below.

---

## Part 2 — A. A session from a session

### First, which "from"

Three different products hide in one word, and they cost wildly different
things.

| reading | what it inherits | what it needs |
|---|---|---|
| **A1 same place** | `cwd` + `command` of the source | nothing new — `>` already does this per *row* |
| **A2 same worktree, new branch** | git state | git knowledge in tally, which [01](01-scope.md) rejects: it belongs in a spawn preset |
| **A3 same conversation** | the agent's session id, resumed or forked | a label only the agent can write (`tl.resume`), then it is A1 with different argv |

**A1 is nearly free and A3 is A1 plus one label.** That is the cheap,
honest version of this feature: *spawn a sibling of that pad*.

### Gesture options

**A-i — Fork is a pick-up-and-drop verb.** `Top(7)` holds; every live pad
goes some colour; tap the source (it goes white, it is in your hand); tap a
dark pad; a new session lands there with the source's `cwd` and `command`.

- Costs one free button. Reuses the Move grammar exactly — same escape
  hatch, same preview, same two taps.
- Conflicts with Move: Move and Fork are the *same gesture shape* with
  different outcomes, and that is dangerous. Picking a button up and
  putting it down should not mean two things depending on which modifier is
  held. Unless — see A-iv.

**A-ii — `>` learns a source.** Hold a row's `>`, tap any pad: launch into
that row inheriting from *that pad* instead of from the row.

- Costs zero buttons. The launch column is already the spawn verb.
- Breaks a stated property: `>` is a *tap* and not a chord, deliberately,
  "because it destroys nothing" — and `chord.rs` currently has a held
  modifier *suppress* `>`. Making it a modifier means a slip while
  mid-launch spawns something.

**A-iii — double-tap a live pad spawns a sibling in its row.** Zero
buttons, no modifier.

- Reject. It puts a clock back on the grid, which [05](05-interaction.md)
  spent a section removing, and the failure mode is *an accidental extra
  agent in your repo*.

**A-iv — one modifier, verbs chosen by destination.** The interesting one.
Hold `Top(7)`, tap a live pad — it is in your hand. Then:

```text
drop on a dark pad      → move it there        (Move, from 22)
drop on `>`             → fork a sibling into that row
drop on the pad it came from → nothing
drop on an occupied pad → nothing (dim, as 22 already specifies)
```

- One button, one grammar, no mode toggle, and `Action(8)` keeps Stop
  alone — which is what [22](22-the-mode-button.md) actually wanted before
  it ran out of buttons.
- The cost is real: the destination is now load-bearing, so *what the
  gesture does* is decided by where your second finger lands. The preview
  has to carry it — the `>` column lighting a *spawn* colour while the dark
  pads light a *move* colour, under the same hold. That is two
  consequence-colours in one preview, which the held-preview section has
  never had to do.

**A-v — the shell.** `tally fork <addr>` — source's `cwd`, new pad, zero
surface cost. Consistent with where purge and mute went. Weakest claim:
forking is an *at-the-desk, looking-at-the-grid* act, which is the exact
argument that brought the launch column back onto the surface.

---

## Part 3 — B. Two sessions synced

**This is the one where the word is doing all the work.** There are at
least three products under it and they do not share a gesture, a cost, or
a layer.

| reading | what it actually is | where it lives |
|---|---|---|
| **B1 one view** | both sessions in one tmux window, side by side | tmux. Real, cheap, and already half-built: a view is a pane ([15](15-views.md)) |
| **B2 one group** | amber on either shows both; `►` ranks them as one; they move and die together | tally — needs a group label and a filter through the queue, the tempo, and the logo |
| **B3 one context** | they see each other's output or share a scratchpad | **not tally.** That is an orchestration product; tally never holds the write lease |

**B1 is the honest reading of "tap them together".** Two pads, one
`tmux join-pane`, and the jump target for both becomes that window. It
changes nothing about the sessions, is reversible, and the grid can show
it with nothing new: two pads that jump to the same place.

**B2 is the expensive reading** and it is the same machinery as C's
selection. Note what [22](22-the-mode-button.md) already decided about this
class of feature when it rejected Solo: *arrangement beats state*. Putting
two sessions next to each other on row 1 answers "these belong together"
with nothing to remember and nothing to clear — which is an argument that
**Move is the sync feature**, and B2 is Move with bookkeeping.

### Gesture options

**B-i — the literal one: press both pads at once.** Physically it is the
gesture you described, and it is the only one that would feel like tapping
two things together.

- Needs a time window on the *grid* — two downs within ~150 ms — which is
  the one thing the input path has none of, and the fat-finger blast radius
  of an accidental pair is a question nobody has asked yet.
- Could be made survivable by requiring a held modifier anyway, at which
  point it is pick-up-and-drop with extra risk and no extra expressiveness.

**B-ii — pick up and drop onto another live pad.** In A-iv's grammar, an
occupied destination currently does nothing. It could mean *pair these*.

- Zero new buttons on top of A-iv. The grammar stays one sentence: pick a
  pad up, and where you drop it says what happens.
- [22](22-the-mode-button.md) rejected drop-on-occupied as *swap* because
  it needs two label writes and an atomic op anchord does not have. Pairing
  is not a swap — it is one write, or a tmux call, or both — so the
  objection does not transfer.

**B-iii — do nothing, and let Move be the answer.** Put them side by side.
No code, no button, no new word.

---

## Part 4 — C. Spawn four, select them, give them a goal

Three separate problems wearing one sentence. Taken in order, they get
harder, and the last one hits a wall that is not a design preference.

### C1 — spawning four

The surface spawns one at a time, and the row is already the unit: `>`
fires into a row and takes the leftmost free column. Options:

- **hold `>` and it keeps firing** — a repeat, on a button, not a pad. Four
  presses is also four presses; this saves nothing and adds a clock.
- **`>` under `Session`** — the overview addresses rows; "fill this row"
  has a natural home there. But [20](20-the-overview.md) deliberately makes
  `>` **dark and inert** under `Session`, because a launcher that fires
  into the wrong row is the one mistake that column must not make.
- **the shell.** `tally spawn 1:1-1:4 -- claude`, or a count. The folder
  comes from the shell ([11](11-cli.md)), and four agents on one goal is a
  thing you set up while sitting in the repo, not while glancing at the
  desk. **This is the strong option.**

### C2 — selecting a set

The genuinely new primitive. Nothing on this surface has ever had a
multi-operand selection.

| option | shape | cost |
|---|---|---|
| **accumulate under a hold** | hold a modifier, tap pads, each toggles; selected pads go white | a selection that survives the release is latched invisible state — unless it does not survive, in which case the goal has to be delivered *inside the hold*, with your other hand |
| **rubber band** | tap corner, tap corner, rectangle selected | two taps, exactly pick-up-and-drop's shape, and "four in a row" is a rectangle |
| **the row is the set** | no selection at all: the unit is a row, and `>` is already row-scoped | zero new machinery, and it matches how the board is actually laid out |
| **a page is the set** | the overview already addresses rows as single pads | same, coarser |

**The row is the set** is the one worth arguing for. Every other option
builds a selection model to express something the layout already says — and
Move exists precisely so the layout can be made to say it.

### C3 — giving them a goal

**Here is the wall.** A goal is text, and:

- the grid has no input ([01](01-scope.md)),
- tally attaches as an **observer** and never holds the control lease, so
  it cannot type into a running session even if it had the text.

~~So **broadcasting a goal to four already-running agents is out of scope
by construction**, not by preference. It would need anchord to grow a
write path and tally to take a lease, and that is the invariant the whole
stack is built on.~~

⛔ **False, and [25](25-the-squad.md) measured it through
`anchor attach`.** The second bullet is the bad one: *tally* never types
into a session, and nothing proposed that it should. A **master agent**
types, by driving tmux — the same privilege any shell on this machine
has. anchord grows no write path, `tallyd` takes no lease, and the
observer invariant is untouched because `tallyd` is not in the picture.

What survives, and is the half worth keeping: **the goal never comes from
the surface.** No clipboard button, no staged prompt, no invisible state.
The human says it to a master in words; the master writes files.

What *is* free: **argv**. A goal handed to a session at spawn time is just
a command line. That splits the request cleanly:

| want | verdict | measured |
|---|---|---|
| four *new* sessions each starting on the same goal | **free** — argv | free, though [25](25-the-squad.md) does not use argv |
| four *running* sessions redirected onto a goal | ~~**forbidden**~~ | **free** — tmux, not anchord |
| a goal typed on the grid | out of scope | **still out of scope**, and right |

And if the text has to come from somewhere other than a shell, the
candidates are a file (`goals/x.md`), the clipboard (`pbpaste`), or a
staged prompt — all of which are *invisible state*, which is the thing the
surface has rejected three times now. A button that sends whatever happens
to be on your clipboard to four agents is the worst version of this
product.

---

## Part 5 — What this says, if anything

1. **The pick-up-and-drop grammar is the unlock.** It is already designed
   ([22](22-the-mode-button.md)), unbuilt, and it is the only primitive
   that fits a two-operand verb. Move, Fork and Pair are all the same
   gesture with different destinations. Building Move first is the way to
   find out whether the grammar survives a thumb.
2. ~~**Two free buttons, and A-iv spends only one.**~~ **One free button
   now.** [25](25-the-squad.md) spends `Top(8)` on `User + >` — a squad in
   a row — so A-iv's modifier has `Top(7)` and the surface has nothing
   spare. The point stands in the form it was made: if one modifier can
   carry move / fork / pair by destination, `Top(7)` is enough for all
   three and the `Action(8)` mode toggle is never needed.
3. **"Sync" has to be pinned down before it can be built**, and the cheap
   reading (one tmux view) and the expensive one (a group that the queue,
   the tempo and the logo all have to know about) share nothing but a word.
4. ~~**Half of request C is a shell command we could write this
   afternoon**, and the other half is forbidden by the observer
   invariant.~~ **The forbidden half was not forbidden** — see the banner
   at the top. The lesson is the one this project keeps relearning in the
   other direction: an invariant quoted from memory is a claim, and this
   one was never measured until [25](25-the-squad.md) measured it.
5. **Nothing here has been on hardware**, which is the only instrument that
   has ever settled an argument in this project. Five times now a surface
   fact has only shown up on the surface.
