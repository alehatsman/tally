# 12 — Layout proposal: rows are projects

**Status: proposal, not design.** Nothing in `01`–`11` is amended by this
file. It proposes replacing one shipped feature ([pages](03-model.md#pages),
[M4](../ROADMAP.md#m4--pages)) and reopening one stated non-goal
([no input from the grid](01-scope.md)), and until that argument is settled
the rest of `docs/` is the design and this is a document about it.

It exists because of a use report: **eight pages of sixty-four is capacity
for a workload of about twenty**, and the top row is spending four
silkscreened arrows on a problem nobody has.

## The surface says more than we are using

Checked against Novation's user guide, *Hardware Overview*, p.8 — what is
physically printed on the device:

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

Top row is `CC91`–`CC98`, logo `CC99`, right column `CC89` down to `CC19`
— the addressing in [02-surface.md](02-surface.md#addressing), unchanged.

What those legends mean to the device, which is what a user who has ever
opened Ableton already knows:

| printed | native behaviour |
|---|---|
| ▲ ▼ | move the session outline across **scenes** (rows) |
| ◄ ► | move it across **tracks** (columns) |
| `>` | **launch a scene** — the whole row fires at once |
| Stop/Solo/Mute | **cycles** the bottom row's verb: Clips → Stop → Solo → Mute |
| Session | return to the overview |
| Drums / Keys / User | the three **Custom modes** — layouts the user defines |

The current design uses the top row for pages and the right column for four
unrelated hold-modifiers. Neither assignment has any relationship to what is
written on the keys. **This proposal spends every legend on something it
already means.**

## What the field is doing

Three observations from how people actually run herds of agents, and what
each one implies here.

**The state language is converging on four states.** herdr shows per-pane
running / waiting-on-input / error / done. [04-signals.md](04-signals.md)
derives the same four independently. Nothing to change — it is corroboration,
and worth recording as such.

**The expensive problem is triage, not launching.** The DHH-sixteen-agents
workflow is: start a pile of them, then use the sidebar to find *which one
needs a decision*. Spawning is one command; finding the blocked one is the
cost all day. A surface with sixty-four launch buttons has over-solved the
cheap half.

**Approval is never automatic, by policy.** herdr's blocked detection is
deliberately strict and it refuses to auto-approve anything. So the human
answering is not a step that can be optimised away — it is the bottleneck,
and it currently has zero buttons on this device.

## Pages, and why they should go

Four arguments, in descending order of how much they should count.

1. **A page is hidden modal state.** [05](05-interaction.md#the-two-rows-that-are-not-slots)
   already paid for this lesson once on this exact row: two bright things
   meaning two different things were unreadable, and the rule that came out
   of it was *exactly one thing is bright and it is where you are*. A mode
   you can be in and forget is the same bug one level up — it is why the
   page row needs a cross-page attention tell and a logo tell to stay
   honest. Both of those are mitigations for the concept, not features of it.
2. **Grouping belongs on an axis you can see all of at once.** A row is
   visible. A page is a thing you remember. Pages were introduced as
   *grouping, not capacity* ([03](03-model.md#pages)) — rows do grouping
   better by being present.
3. **512 slots for twenty sessions.** Stated plainly in `03` and true.
4. **It frees eight buttons**, four of which are arrows.

Overflow is real and the device already has the answer: **▲▼ scroll the
window over the project list.** That is scene-outline movement, literally
what the arrows do natively. It is not a mode — it is a scroll, with
on-screen evidence (see [the logo](#the-logo)).

What is lost, stated without softening: **cross-page attention was an M4 exit
criterion and it was confirmed working.** Deleting pages deletes a verified
feature. The replacement claim is that it was solving a problem pages
created.

## The layout

### Grid — row is a project, column is a lane

```text
slot = project:lane             e.g. "tally:3"

row 1..8   the project under that row, from slots.toml order
col 1..8   lanes within that project, left to right
```

Convention, not enforcement: lane 1 is the long-lived session for that repo,
lanes 2+ are the throwaway swarm. A row reads left to right as *how deep am
I into this repo right now*.

Colours are unchanged from [05](05-interaction.md#the-colour-language) —
green alive, motion means producing, amber wants you, red died badly, dark
red means the daemon lost it.

The payoff is visible from the door: **amber anywhere in row 4 means the
fourth project is blocked.** Project identity becomes spatial instead of
modal, which is the whole argument against pages expressed as a positive.

The address format in [03](03-model.md#the-address-pinned) changes shape —
`page:row:col` becomes `project:lane`, still two halves, still typed into
shell scripts, still flipped in exactly one place in the surface layer.

### Right column — scene launch, used as scene launch

| button | tap | hold + tap a pad in that row |
|---|---|---|
| `>` rows 1–7 (`CC89`…`CC29`) | **spawn into the next free lane of this project** | row-scoped preview, then act |
| Stop/Solo/Mute (`CC19`) | cycle the verb — see below | — |

Pressing `>` beside a row and getting a new agent in that repo *is* scene
launch; the translation costs nothing because there is nothing to translate.

It also removes the config requirement from spawning. Today an empty pad
spawns only if `slots.toml` named it; here the **row** names it, so the
file shrinks to a list of projects and a default spec. That matches the
observed habit of starting from scratch and spawning ad hoc, and
`tally spawn` ([11-cli.md](11-cli.md)) stays the escape hatch for a
directory that is not on the surface.

**Row 8 has no scene button**, because `CC19` is the verb key. Either row 8
is a scratch row spawned from the CLI, or the surface shows seven projects.
Recommend seven projects and a scratch row; 56 slots is still more than the
workload.

### The verb key

Steal the cycle wholesale. Tap `CC19` to cycle; its own colour is the
indicator.

| verb | key colour | `>` does, to the whole row | hold + tap, to one pad |
|---|---|---|---|
| **Launch** (default) | 45 blue | spawn into the next free lane | — grid is normal |
| **Stop** | 5 red | kill every session in the project | kill |
| **Solo** | 3 white | jump to this project's blocked-or-newest lane, dim every other row | jump, dim the rest |
| **Mute** | 9 amber, dim | this project never raises attention | mute |

One modifier, one verb, two granularities: row via the scene button, single
session via hold-and-tap. Kill, mute and jump each burn a dedicated
right-column button today; they collapse onto the key with those three words
printed on it, and `tl.mute` / `tl.killed`
([03](03-model.md#where-the-binding-lives)) are untouched.

**The objection is real and it is the same objection as pages:** a cycling
verb is persistent modal state. Three mitigations, and they have to hold
together or this part falls:

- it reverts to Launch after one use, and after about five seconds idle —
  the only clock on the input path, and on a *control*, not on the grid,
  so [05's no-timing rule](05-interaction.md#what-a-press-does-not-do)
  survives intact for presses;
- it is the only non-blue thing on the surface while armed;
- hold-and-tap still previews before acting
  ([05](05-interaction.md#the-held-preview)), so nothing destructive
  happens without the grid saying so first.

If that is still too clever, the fallback is `CC19` as a plain hold-modifier
for Stop alone, with Solo and Mute moved onto Drums and Keys. **This is a
judgment call with a blast radius — Stop kills running agents — so it is
named here and not decided here.**

### Top row — the arrows answer

| key | printed | normal | while a slot is focused |
|---|---|---|---|
| `CC91` | ▲ | scroll the project window up | send `Up` |
| `CC92` | ▼ | scroll the project window down | send `Down` |
| `CC93` | ◄ | previous lane | send `Esc` — *no, don't* |
| `CC94` | ► | next lane | send `Enter` — *yes, do it* |
| `CC95` | Session | back to the plain grid; clears focus and any armed verb | — |
| `CC96` | Drums | one-shot mode: the 8×8 becomes canned prompts fired at the focused slot | — |
| `CC97` | Keys | reserved | — |
| `CC98` | User | user-defined | — |

**Focus is not a mode you select — a blocked agent puts you in it.** Tap an
amber-pulsing pad and, instead of jumping, that slot becomes focused: its
row dims, the four arrows light white, and ▲▼◄► drive the prompt in place.
Answer, the pad goes green, the arrows go dark, and you never left the
window you were working in.

A yes/no on one `Bash(rm …)` prompt costs one press instead of a tmux
switch, a read, a keystroke and a switch back. That is the extension of the
project's own thesis — *not a list you have to go look at* — by one step:
**not a window you have to go switch to.**

Jump survives and must: `Solo` + `>`, or a second tap on an already-focused
pad. The press that jumps stays available on every bound slot; what changes
is that it stops being the *only* thing a blocked pad can do.

Drums is the same trick one level up. A drum pad is a one-shot sample
trigger, so Drums mode makes the grid sixty-four canned prompts — *continue*,
*run the tests*, *commit and push* — fired at whatever is focused. Launch
with scenes, triage with arrows, drive with drums.

### The logo

Unchanged as the health light ([05](05-interaction.md#the-logo-is-the-health-light)).
It loses *off-page attention* along with pages and gains a smaller version
of the same sentence: **amber when a project scrolled off the window wants
you.** Same colour, same meaning, one fewer concept.

## The scorecard

| printed | native | proposed | honest? |
|---|---|---|---|
| ▲ ▼ | across scenes | scroll projects / answer up-down | yes |
| ◄ ► | across tracks | across lanes / Esc and Enter | yes |
| `>` | launch a scene | launch an agent into this project | nearly literal |
| Stop/Solo/Mute | cycle the verb | cycle the fleet verb | literal |
| Session | return to the overview | return to the plain grid | yes |
| Drums | one-shot pads | one-shot prompts | yes |
| Keys | chromatic custom mode | reserved | thin — the weakest cell here |
| User | custom mode | user-defined | yes |
| ◈ logo | brand | health light | neutral |

Nothing means something it does not say, which is a direct consequence of
deleting pages: **pages were the one assignment with no legend behind it.**

## What this contradicts, and what it costs

Four, and the first is not a detail.

**1. Input from the grid is currently a stated non-goal, with a reason.**
[01-scope.md](01-scope.md) rejects *"let it hold the control lease so I can
type from the pads"* because one writer per PTY is anchor's invariant and
the writer is the terminal you jumped to; [05](05-interaction.md#what-a-press-does-not-do)
closes with **"No input."** Answer mode asks for that back.

It can be had *without* breaking the invariant, and this is the part that
makes the proposal buildable rather than merely nice:

> **Send the keys to the tmux pane, not to the PTY.** The jump target is
> already a tmux pane running `anchor attach` ([06-jump.md](06-jump.md)),
> and that attach is the writer. `tmux send-keys -t <pane> Down` injects
> into *the terminal's* input — exactly the path your keyboard takes.
> tallyd stays an observer, takes no lease, and anchor needs no new call.

The limit is honest and worth stating: it only works for a slot that has a
live view window. A blocked agent you have never jumped to has no pane, so
the first press still has to be a jump. Whether that is acceptable, or
whether anchor should grow a real `send` seam
([07-seams.md](07-seams.md#what-we-must-not-ask-anchor-for) is the place
that argument belongs), is **open**.

**2. It deletes a shipped, verified feature.** Pages and cross-page
attention met their exit criteria on hardware
([M4](../ROADMAP.md#m4--pages)). This is a reversal, not an addition.

**3. Lane allocation becomes tallyd's policy.** "Next free lane in this row"
is a decision `slots.toml` currently forces the human to make. Low risk,
but it is new behaviour in a component whose whole claim is that it holds
nothing ([08-lifecycle.md](08-lifecycle.md)) — so the rule has to be
derivable from a `list`, which "leftmost column with no live binding" is.

**4. The verb key is modal.** Argued above; not settled.

## Open, before any of this is design

- Does answering from the grid survive the `tmux send-keys` route, or does
  it want a seam from anchor? **This decides whether the proposal is cheap
  or expensive.**
- Verb cycling, or Stop-only hold with Solo and Mute on the custom-mode
  keys?
- Seven projects and a scratch row, or eight projects and no scene button
  on row 8?
- What is Keys for? It is the one legend with no job, and an unassigned
  button is better than a strained one.
- Does `project:lane` replace `page:row:col` in the label, or coexist for
  one milestone? A label format change is visible to anything else reading
  anchor's labels.
