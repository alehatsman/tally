# 18 — The right column launches

Settled elsewhere and not reopened here: the grid is an 8×8 window onto a
64-row space ([16](16-the-queue.md)), the top row is navigation, and a pad
tap is a jump. This document is only about the eight buttons on the right
edge.

## What they were, and what that was worth

Four hold-modifiers — kill, respawn, purge, mute — and four dark. After a
day of real use:

```text
jump           341
rows (scroll)   24
next             4
back             4
kill             4
respawn          0
purge            0
mute             0
```

**Respawn was not unused, it was unreachable.** It requires a resolvable
spec, and there is no `slots.toml` on this machine — `tallyd` loads
*8 pages, 0 slots* — because spawning happens from a shell, where the
directory is known. `tally spawn` also deliberately writes no `tl.spec`
([11](11-cli.md#what-it-does-not-do)), so the preview could never light it
and the press could never fire. The same emptiness makes **tap-a-dark-pad**
inert: there is no spec to spawn.

So the column was one button doing work, one that could not, and two nobody
wanted.

## The fact that changes the menu

**The grid knows the directory now.** anchord keeps it per row, on dead
rows too:

```text
cwd        /Users/aatsman/projects/bright-frontend
command    isayes
args       ['--', '--resume', 'd37ee7e5-…']
login_shell true
```

The reason spawning lived in a shell — *the shell knows the folder and a
config file cannot* ([11](11-cli.md#the-folder-is-the-problem)) — stops
being true for any row that has ever held a session. The row itself knows.

That is what `slots.toml` was trying to be and failed at: a thing you have
to write in advance, about work you have not started yet.

## `>` spawns into its row

Seven scene-launch buttons, each beside a grid row, each a **tap**.

```text
CC89  >   row 1
CC79  >   row 2
CC69  >   row 3
CC59  >   row 4
CC49  >   row 5
CC39  >   row 6
CC29  >   row 7
CC19  ■   hold = kill        ← the button the silkscreen calls Stop
```

| | |
|---|---|
| **where** | the leftmost free column of that row |
| **directory** | inherited from the newest session in that row, whatever state it is in |
| **command** | that session's `command` and `login_shell` |
| **arguments** | **dropped** |
| **lit** | 1 dark grey when it would act; off otherwise |

**Arguments are dropped on purpose.** A row running
`claude --resume d37ee7e5…` describes one conversation; copying that would
put a *second pad* on the *same conversation*, which is the one thing a new
agent must not be. Keeping `command` keeps the choice that matters — a row
running `isayes` gets another `isayes` — and dropping `args` keeps it a new
agent rather than a clone.

**It is a tap, not a hold.** The hold-then-tap grammar exists so you can
change your mind before something is destroyed
([05](05-interaction.md#the-exact-rules-of-a-chord)). `>` destroys nothing;
the worst case is one `claude` you did not want, which is a kill away. A
hold here would be ceremony with nothing to protect.

**Dark means it would do nothing**, the same rule the top row runs on: a
row with no session to inherit from has no directory, and a full row has
nowhere to put one.

### Row 8 is shell-only, by rule

Eight buttons, eight rows, and one button must be the kill modifier — so
one row has no `>`. Rather than hide that, it is a rule: **`>` is a
seven-row gesture, and the bottom row of the window is spawned into from a
shell.**

Stated as a decision because the alternative was worse. Moving kill
anywhere else means moving the one button that is 8 mm from the wrong pad,
and it has four presses of muscle memory in the corner the silkscreen
labels *Stop*. A destructive button that moves is how you get a kill you
did not mean.

The cost is honest: a grid where one row behaves differently is a grid you
remember rather than feel. It is affordable only because the window
scrolls — row 8 is one `▼` away from being row 7.

## Kill keeps its corner

`CC19`, hold-then-tap, unchanged. White while held, the grid recolouring
to show what it would do. Nothing about
[05](05-interaction.md#kill-is-the-one-that-needs-the-care) changes; it is
now the only modifier, which makes it easier to find rather than harder.

⚠️ **Two details here did not survive the week.** The resting colour was
blue when this was written and went **red** the same evening — the colour
of its own consequence, which retired blue from the palette. And *the only
modifier* became *the only modifier button*: the corner carries two verbs
now, Stop and Move, swapped by a double-tap, because this document's own
closing rule — *the bar for the next idea is a button taken from something
that works* — turned out to have one more answer in it
([22](22-the-mode-button.md)).

## Purge and mute move to the shell

They lose their buttons and must not lose their meaning.

```text
tally purge <row:col | page:row:col>      forget every ended row behind a pad
tally mute  <addr> [--off]                stop this slot raising attention
```

**Purge has to keep existing** — `anchor` has no `purge` subcommand
([anchor#14](https://github.com/alehatsman/anchor/issues/14)), so deleting
tally's gesture without replacing it would leave rows accumulating with no
remedy anywhere. Twenty-five rows after a day, and nothing clears them.

**Mute has to keep existing for a reason that post-dates its button.** It
is the starvation hatch for the queue ([16](16-the-queue.md#the-queue)) and
now for the tempo ([17](17-the-tempo.md)): a prompt you are never going to
answer otherwise sits at the head of `►` forever and drives the breathing
up with it. Zero presses in a day is not an argument that it is unneeded —
it is an argument that the signal has not lied yet.

Both go to the shell on the same principle the row-8 rule runs on, and the
one the usage data states plainly: **the grid does the common things, the
shell does the rare ones.** 341 jumps against 0 purges is not a close call.

## What is deliberately not here

**Respawn, in any form.** *Put back what was here* is a real want — it is
what #9 cost five hand-written label edits — and `>` is not it: `>` gives
you a **new** agent in the same directory, deliberately dropping the
`--resume` that would make it the old one. Restoring a specific
conversation stays `tally spawn <addr> -- claude --resume <id>`, which is
exact, visible, and already works. A button that guesses which conversation
you meant is a button that is wrong at the worst moment.

**A merged kill/purge modifier.** Tempting — the held preview already
colours per pad by consequence, red for *will die* and dark red for *will
be forgotten*, so one button could do both and show it. Rejected: it widens
the blast radius of the single gesture [05](05-interaction.md) spends a
section protecting. Purge may lose its button; it may not share the
dangerous one.

**The four dark buttons.** There are none left — `>` takes them. That is
the first time this surface has had nothing spare, and the bar for the next
idea is now a button taken from something that works.

## Edge cases

| situation | behaviour |
|---|---|
| row has no session, ever | `>` dark; nothing to inherit |
| row is full (8 live pads) | `>` dark; nowhere to put it |
| row holds only dead sessions | `>` lit — a dead row still knows its directory |
| newest in the row is `lost` | lit, and inherits from it; `lost` still records a cwd |
| row's newest ran `isayes -- --resume x` | spawns `isayes`, no args |
| two rows share a directory | irrelevant; `>` asks its own row only |
| the window is scrolled | `>` follows the **display** row, not the address row |
| anchord unreachable | `>` dark — no session list means nothing to inherit |
| `>` pressed while kill is held | nothing; a held modifier owns the surface |
| row 8 | no `>`, by rule |

## Validation

**Unit-testable:**

- the target is the leftmost free column, and free uses the same predicate
  as `tally spawn` and the grid
- the directory and command come from the **newest** row, in any state
- arguments are dropped, and `login_shell` is not
- an empty row and a full row both produce no launch
- `>` follows the display row across a scroll
- `tally purge` and `tally mute` parse, and refuse an address with no page
- kill is still the only `Action`, and `Action::of` returns `None` for
  `CC29`–`CC89`

**Only the hardware says:**

- [ ] a column of seven dim buttons beside the rows reads as *launch here*
      and not as decoration
- [ ] the row-8 rule is learnable, or is a daily irritation
- [ ] a week in which `>` is used at all — if the shell still wins, the
      column has again been given work it does not want
