# 04 — Signals

The hard part of this project is not lighting an LED. It is knowing *which*
LED to light amber, and being right often enough that you trust it.

## What anchor gives us, and what it does not

anchor reports `state` — `running`, `closing`, `exited`, `lost` — and
nothing about what the process inside is doing. That is deliberate and
correct: "this agent is waiting for you" is a *policy about what a UI
shows*, and anchor's scope doc keeps those in clients
([01](01-scope.md#what-tally-asks-of-anchor-and-what-it-must-not)).

teleport already hit this and solved it by attaching to every session it
owns and scanning the stream itself. That is one observer connection per
session, which is cheap: anchor is already producing those bytes for its
log, and an observer never takes the lease or writes. **tally does the
same.**

## The watcher

Per live session, one `attach` with `control: false`:

```text
tallyd ──attach(session, from, control=false)──▶ anchord
    ◀── output frames ───────────────────────
```

Rules, all inherited from teleport rather than invented:

- **Observer only.** Never `control: true`. The lease belongs to the
  terminal you jumped to; a surface that steals it makes typing stop
  working, from across the room, for no visible reason.
- **A reconcile loop, not a hook on create.** Sessions appear because a
  shell script or teleport made one. Poll `list` every 2 s and start a
  watcher for anything live that lacks one — `running` or `closing` only. An
  exited session keeps whatever was derived while it ran; a `lost` one has
  nothing to read.
- **Only slots get watchers.** A session with no `tl.slot` is not on the
  grid, so scanning its bytes buys nothing. This is the one place the
  client-side filter has to be applied *before* doing work rather than just
  before rendering.
- **Start from offset 0 the first time.** The signals worth having are
  emitted at startup — an agent sets its terminal title and prints its
  resume link in its opening banner — and the supervisor cannot attach
  before that happened. teleport's comment on this is a bug report written
  before the bug: *"Starting at the head reliably misses exactly the output
  that matters, which is how this was first written and why the title never
  appeared."*
- **Resume from `scanned_to` after a drop.** A watcher dropped for
  backpressure must not re-read the whole log — worst on exactly the busy
  sessions that caused the drop.
- **A watcher ending is normal.** Exit frame, EOF, or dropped-for-falling-
  behind all end it; the supervisor starts a fresh one if the session is
  still live.

## Four inputs, in increasing order of trust

### 1. BEL (`0x07`) — "it wants you"

The terminal bell is what an agent emits when it is done talking and wants
input. Most agents ring it; so does a shell with a failed completion, which
is the problem.

```text
a 0x07 seen in GROUND state  →  last_bell_ms = now
```

**Not `bytes.contains(&0x07)`.** `BEL` is also an OSC *string terminator*,
so `ESC ] 0 ; some title BEL` carries a `0x07` that is not a ring — and an
agent that sets its terminal title, which Claude Code does constantly, would
look exactly like one that asked a question and went quiet. Telling the two
apart needs the OSC state machine running *before* the bell check, which is
why that scanner is in the attention path.

It is one byte with no structure, so it carries no reason and cannot be
attributed. It is a **hint that something changed**, and on its own it is
not enough to flash a pad amber for ten minutes.

> **Against Claude Code this floor is zero.** 1012 `0x07` bytes across
> 6.6 MB of one real session, **every one an OSC title terminator, zero in
> GROUND state** — and the same on plain `claude` with no wrapper.
> `bell_pending` is therefore never set and rule 5 of
> [the precedence table](#precedence) is unreachable against the agent this
> grid actually runs. The scanner is *correct*, which is why we can say
> *zero* rather than guess: a naive `contains` would have reported 1012
> phantom rings. It is a working detector with nothing to detect.
> ([D5](10-decisions.md#d5--the-bell-that-is-never-rung))

### 2. Quiet — "it stopped working"

```text
now - last_output_ms >= IDLE_THRESHOLD  →  idle        (IDLE_THRESHOLD = 30s)
```

teleport uses 30 s for the same job. An agent mid-task emits something — a
spinner, a tool call, a token — far more often than that, so quiet means *it
is not currently thinking*.

**This half measured true, and it was doubted.** A `claude` at its main
prompt with nothing typed emits **0 bytes in 92 s**; the live session on pad
`1:1` held **42 s flat** between turns. Quiet is reachable, rule 6 is
honest, and motion does mean producing output.

**A wrapper can take that away.** `isayes` repaints a footer and blinks a
cursor about twice a second, so under it `last_output_ms` never ages and
`Quiet` is unreachable. That is a bug in the wrapper's output, not a colour
to delete here — but it is why the clearing rule below cannot lean on
output.

#### Stating the bell precisely, because the obvious phrasing is unimplementable

"Bell since the last output" cannot be written as a timestamp comparison.
**The BEL arrives inside an output chunk**, so the chunk that carries it
sets `last_bell_ms` and `last_output_ms` to the same instant, and `>=`
versus `>` on two equal numbers flips the answer.

So the predicate is a **flag, not an arithmetic**:

```text
on each chunk:
    tail = bytes after the LAST BEL in the chunk, or the whole chunk if none

    if chunk contains BEL:
        last_bell_ms = now
        bell_pending = true

    if tail is not all whitespace:
        bell_pending = false        # it kept talking

    last_output_ms = now

needs_answer = bell_pending && (now - last_output_ms > ANSWER_QUIET)
                                                       ANSWER_QUIET = 2s
```

The `tail` is computed **once, before** the BEL branch, and the
not-all-whitespace test applies to every chunk rather than only the one
carrying the bell. That ordering is the whole trick: it makes one rule —
*non-whitespace output cancels a pending bell* — cover both the bell chunk
and everything after it, where the obvious two-branch version silently
treats a following `\r\n` as the agent resuming work.

Three things that look like detail and are not:

- **The tail check is the "it rang and kept going" case.** An agent that
  prints `\a` mid-stream and carries on talking has not asked you anything.
  Without it the pad would flash amber 2 s later, every time, which is the
  single easiest way to make the signal untrustworthy.
- **Whitespace never cancels.** The bell is almost always followed by a
  newline before the prompt — sometimes in the same chunk, sometimes in the
  next. Treating that newline as "it kept going" means the signal never
  fires at all, and which chunk it lands in is a buffering accident nobody
  should be able to observe.
- **`bell_pending` is cleared by output, not by a timer.** It stays true for
  as long as the agent stays silent, which is the desired behaviour: a pad
  that has been waiting for you for ten minutes should still be amber.

The 2 s is **not** the 30 s idle threshold. Attention wants to fire fast —
the whole point is that you look up — and "nothing since the bell" is a much
cheaper claim to make than "this session is idle".

### 3. OSC title — "what it is doing"

Agents set the terminal title (OSC 0/2). teleport's `session/osc.rs` is an
incremental scanner for exactly this, chunk-boundary-safe, with OSC 8
hyperlink extraction for Claude Code's resume id riding along. Ported.

Its own doc comment is the honest caveat and it generalises to this whole
document:

> both rest on Claude Code's own current terminal output format, which is
> not a documented or versioned contract. A future CLI release changing or
> dropping either sequence degrades silently back to "no title" / "no resume
> id" — never a parse error, never a broken session.

**No signal may ever become load-bearing enough that losing it breaks the
surface.** Lose the title, lose a word. Lose the bell, lose the amber — the
pad is still green and still jumps. That rule held exactly as written: the
bell is absent and the grid is still truthful about alive and dead.

### 4. A hook — "it told us"

The three above are scraping. There is an authoritative channel for Claude
Code specifically, and against that agent it is the *only* one that fires.

**It writes a label, not a message to us.**

```text
~/.claude/settings.json
  Stop, Notification  →  tally attention on  || true
  UserPromptSubmit    →  tally attention off || true
  PreToolUse          →  tally attention off || true
```

```text
tallyd, on the tick it already makes:
  tl.attention present  →  NeedsAnswer { source: Declared }
```

The hook talks to **anchord**, which tallyd is already polling. That buys
more than it looks like:

- **No new channel.** No socket, no FIFO, no framing, no error codes, no
  reader task, no reopen-on-EOF — which is what
  [08](08-daemon.md#one-process-no-socket) refuses on principle.
- **It survives tallyd.** Declared attention outlives a tallyd restart
  because it is in the same place as the binding. A FIFO's contents do not.
- **teleport can see it too.** Two surfaces, one fact, no integration.

The cost is latency: one reconcile tick, ≤ 2 s. That is the same 2 s the
derived signal waits anyway, so declared attention is never *slower* than
the floor it replaces.

#### Why `Stop` and not only `Notification`

`~/.claude/settings.json` runs `defaultMode: auto` with
`skipDangerousModePermissionPrompt`. Permission dialogs — the thing
`Notification` fires for — are mostly answered without asking. The moment
this grid exists to report is **the turn finishing**: the agent said its
piece and is waiting for the next instruction. That is `Stop`.
`Notification` stays in the list because when it does fire it is the
strongest possible statement of *wants you*, and it costs one line.

#### Why `PreToolUse` on the falling edge as well

`UserPromptSubmit` covers answering with words. It does not cover answering
a permission dialog, which fires no prompt submission — so attention set by
`Notification` would outlive the answer and leave a pad amber while the
agent works. `PreToolUse` is *the agent is acting again*, which is the
honest falling edge for that case. Both are idempotent: removing a label
that is not there is not an error.

#### `|| true`, and it is not superstition

Claude Code gives **exit 2 a meaning**, and the meaning differs per hook: a
`Stop` hook exiting 2 *blocks the agent from stopping* and feeds its stderr
back to the model; a `PreToolUse` hook exiting 2 *denies the tool call*. Our
exit 2 is a usage error — one mistyped word in a settings file — and without
the guard that typo does not produce a dead pad, it produces an agent that
cannot finish a turn or run a tool, on a machine where the grid may not even
be plugged in.

This is the rule above pointed the other way. *No signal may become
load-bearing enough that losing it breaks the surface* — and **nothing on
the surface may become load-bearing enough to break the agent.** tally is
the disposable layer; a hook is the one place it runs inside something that
is not.

The honest cost: a real failure — anchord down — stops being visible in the
hook's exit status. It is still one line on stderr, and the grid is already
reporting the same outage as a dark red logo, which is the louder and
correct channel.

#### One bit, two states

Both rising hooks write the same label and `tally attention on` takes no
argument, so *blocked on a dialog* and *turn finished* are one string by the
time anything sees them. That is right for the reader this was measured
against — a human, for whom both mean *go look*. It is not obviously right
for a program: `tally ls` is read by agents, and says `wants you` rather
than `waiting for an answer` because the latter was false in the common
case. Whether the two states should be told apart at all is
[#23](https://github.com/alehatsman/tally/issues/23). **The amber is
unchanged either way.**

## Clearing attention

What clears it depends on which signal raised it, and the two are not the
same rule.

A **derived** attention — a pending bell — is cleared by output, and that is
the flag rule above rather than a separate mechanism.

A **declared** attention is cleared by a hook, because output cannot be
trusted to mean anything: under a repainting wrapper it arrives twice a
second and would clear the label within half a second of the hook setting
it. **Two writers disagreeing at 2 Hz is worse than one writer that can be
absent**, which is why output-clearing was rejected as a floor rather than
kept as a backstop.

| what happens | what it clears |
|---|---|
| any non-whitespace output | a pending **bell** — it answered, or it started talking again |
| `UserPromptSubmit` / `PreToolUse` hook | a **declared** attention |
| the session exits | both: terminal states outrank attention |
| `tally mute` | neither — mute suppresses the *rendering*, leaving the state underneath |

Notably **jumping to a slot does not clear it.** Looking at a window is not
answering the question in it, and a pad that goes quiet because your eyes
moved is a pad that has started guessing.

## The state a pad renders

```rust
enum SlotState {
    Empty,                       // nothing here, or nothing left to do
    Working,                     // alive, producing output
    Quiet,                       // alive, idle
    NeedsAnswer { source: Source },
    Failed { code: i32 },        // exited non-zero
    Lost { reclaimable: bool },
}

enum Source { Derived, Declared }
```

Six variants, because **the grid shows what is alive or what is wrong**. A
session that exited cleanly is `Empty`: it needs nothing, its slot is free,
and the row stays in anchor for `anchor logs` without occupying a light.

Two fields are deliberately absent, for different reasons:

- **`NeedsAnswer` carries no `since`.** How long a slot has been asking is
  `Journal::asking`'s, because it has to survive a `tallyd` restart and a
  variant in memory cannot. That map turned out to be
  [the queue and the tempo](05-interaction.md#the-queue-and-the-two-buttons-that-read-it).
- **`Lost` carries `reclaimable`.** True only for
  `lost_reason=daemon_restart`, where anchord owning the PTYs means it took
  them with it and there is no orphan to collide with. It does not change
  the colour — the pad is still dark red, because the daemon still cannot
  account for what that session did — only whether the pad can be claimed
  again.

`Lost` is its own variant and not `Failed { code: -1 }`, and anchor is
precise about why: **`EXITED` means the daemon can say how the session
ended; `LOST` means it cannot.** Two reasons reach it — `daemon_restart`,
where the PTY died with the daemon and whatever the child did last was never
seen, and `kill_timeout`, where it was killed and never observed exiting, so
the process may still be out there. Rendering either as an ordinary failure
tells a lie at exactly the moment the user needs the truth, and it is a lie
told *sixty-four times at once*, because a daemon restart loses every
running session together. A terminal row carrying **no exit code** is
therefore `Lost` rather than a clean exit.

**Mute is not a variant.** It is an orthogonal bit read from `tl.mute`, and
it suppresses exactly one thing — the `NeedsAnswer` rendering — leaving the
slot to render as `Working` or `Quiet` underneath. A muted agent is still an
agent you can see is alive; mute means *stop shouting*, not *disappear*.

**`frees_the_pad`** is `Empty | Failed { .. } | Lost { reclaimable: true }`
— one predicate shared by the grid, the press and the shell
([07](07-cli.md)).

### Precedence

Several of these are true at once — a session can be `running`, have
produced nothing for a minute, and have a pending bell. So the derivation is
an **ordered** match, and the order is the whole opinion:

```text
1. no session                 -> Empty
2. state == lost              -> Lost
3. state == exited, code 0    -> Empty
   state == exited, code != 0 -> Failed { code }
   state == exited, no code   -> Lost
4. tl.attention present       -> NeedsAnswer { Declared }     unless muted
5. bell_pending && quiet 2s   -> NeedsAnswer { Derived }      unless muted
6. now - last_output < 30s    -> Working
7. otherwise                  -> Quiet
```

Whether a *spec* exists for the slot is not an input. An empty pad looks the
same either way; the spec matters when you press, not when you look.

`closing` is deliberately **not** on that list: it falls through to the live
states at 6 and 7. A session anchor is terminating is still alive, and it
has **no exit code yet** — `exit_code` is `None` until the child is reaped —
so rendering it as `Failed { code }` would mean inventing a number. An
earlier draft grouped it with `exited`; writing the code is what surfaced
that the variant could not be constructed honestly.

Three placements are decisions, not orderings:

- **Terminal states outrank attention.** An agent that rang the bell and
  then died is `Failed`, not amber. There is nothing to answer.
- **There is no `Starting`.** A freshly created session with no watcher yet
  falls to `Quiet` — alive and idle, which is true. The state existed to
  render "hold on" in yellow for about a second, and off-to-green already
  says the press landed.
- **Muting falls through rather than short-circuiting.** Dropping to
  `Working`/`Quiet` is what makes a muted slot still legible.

A hook, when configured, **overrides** the heuristic for that session —
`tl.attention` present means `NeedsAnswer` whatever the bytes say, and its
absence never means "not waiting", only "nobody declared". Nothing in the
surface layer knows which one it got. The
`Source { Derived | Declared }` split existed from the start, which is why
promoting the hook from a nice-to-have to the only working source was a
match arm and not a rewrite.

## What we deliberately do not do

**No VT emulation.** Not parsing the screen, not diffing a rendered buffer,
not pattern-matching prompts. anchor's own CLI refuses to emulate for the
same reason — the user's terminal already is one — and a surface that got
its state from screen-scraping a TUI would break on every agent release.

**No per-agent plugins.** A registry of "how to detect state for agent X" is
a maintenance treadmill. The floor is generic; the ceiling is a hook the
agent itself calls.

**No guessing at reasons.** The pad says *needs you*. It does not say what
it needs. Finding that out is what the jump is for.

## The instrument

Whether any of this works is the actual quality metric of this program, and
it is measurable only by using it and counting the times it was wrong. **A
count you keep by remembering is a count you do not keep.**

So tallyd appends one line per transition to
`~/.local/state/tally/attention.jsonl`:

```json
{"ms":1760000000000,"session":"01J…","slot":"work:3:5","to":"needs_answer","source":"declared","title":"edit tests/pty.rs?"}
{"ms":1760000061000,"session":"01J…","slot":"work:3:5","to":"answered","after_ms":61000}
{"ms":1760000090000,"kind":"nav","via":"next","slot":"1:2:1","waited_ms":219722,"queued":4}
{"ms":1760000120000,"kind":"nav","via":"pad","slot":"1:1:2","waited_ms":274173,"queued":0}
```

A fourth shape, `"via":"overview"`, is no longer written: the overview is
cut (#41) and the nine lines it produced are what cut it. They stay in the
file, and a reader has to keep tolerating them — a journal is a record, so
nothing in it is ever retracted.

Transitions only, so it is a few lines a day, not a log. `kind` absent means
`"state"`, so every line written before navigation was recorded still
parses.

**tallyd reads it back at startup.** `Journal::asking` is an in-memory map
rebuilt empty on every start, so a slot still asking when tallyd restarted
had no record of having asked, and the next tick wrote a fresh rising edge —
three rises against one fall in the file as shipped, with `after_ms`
measured from the third. Scanning the tail for slots whose last transition
was `needs_answer` restores the map with its original timestamps.

That contradicts the first draft's claim that nothing reads it back, and the
contradiction is the point: **the file is the instrument the whole product
is gated on, so it has to be right more than it has to be write-only.** It
also built [the queue](05-interaction.md#the-queue-and-the-two-buttons-that-read-it) by accident — a map from
slot to the moment it began asking, surviving a restart, is exactly what `►`
sorts on.

A restart during an unanswered attention still loses nothing, because the
label does not live here. If the agent was answered while tallyd was down,
the next tick writes the falling edge with an `after_ms` that includes the
downtime — honest, and visible as such.

### What the instrument cannot see, and what the signal now means

Two findings from reading 55 hours of it, and the second is the larger.

⚠️ **It did not record the gesture it is measuring. It does now.** There
was a nav line for `next`, `back` and the overview, and **none for a plain
pad tap** — the gesture the product exists for, running at a reported
341/day against 62 `next` presses in 55 hours. So the file held the ambers
that were raised and almost none of the answers, and the gate it exists to
settle could not be computed from it at any length
([#43](https://github.com/alehatsman/tally/issues/43)).

`via: "pad"` is written beside the other three now, and **the week starts
from the build that carries it** — the 2.3 days already in the file cannot
be repaired, because the presses they are missing have happened.

One field is not borrowed from `next`, and it is the one the line is for:

> `waited_ms` is **absent when the pad was not asking.**

`►` can only ever go to something that wants you. A tap can land on a pad
that wants nothing, and *going to look at something quiet* and *answering
an amber* are the two things the week has to tell apart. Present means an
amber was answered and says how long it had waited; absent means the trip
was yours, not the grid's. Both numbers are the ones that were true when
the finger landed — last tick's queue, like the grid the press was aimed
at.

**And the question it was built for describes a signal that no longer
exists.** The gate asks *how many ambers were wrong*, which was the right
instrument when amber meant *the detector believes it rang and went quiet*.
Since the hook replaced the bell, `Stop` fires on **every turn** — so amber
means *an agent finished a turn*, measured at **544 rises in 55 hours**,
about ten an hour.

A hook-raised amber is essentially never *wrong*; the hook is
authoritative. What it can be is **correct and ignored**. So the honest
question is not precision and recall, it is *how many did you go to, and
how many aged out* — and `after_ms` already carries the second half
(p50 117 s, p90 983 s, max 18.7 h, 10% answered inside ten seconds).

**This is worth stating plainly because it is a risk, not a bookkeeping
note.** Ten rises an hour is a lot of light for a surface whose whole claim
is that you can ignore it until it speaks. If the week says most of them
age out, the thing to change is `Stop` being a rising edge at all, and that
is upstream of every milestone built on top of it.

## Edge cases

| case | behaviour |
|---|---|
| hook fires on a session with no `tl.slot` | label written anyway; no pad draws it, which keeps the subcommand ignorant of the grid |
| session exits with `tl.attention` set | terminal states outrank attention, so the pad renders the exit. No cleanup needed |
| the slot is muted | unchanged — mute suppresses the rendering, not the label |
| `on` twice, `off` twice | idempotent |
| two sessions in one repo | distinguished by pid, which is why ancestry and not cwd ([07](07-cli.md#tally-attention-onoff)) |
| hook fires faster than the 2 s reconcile | last write wins, which is correct: the latest edge is the true one |
| anchord restarts between `on` and `off` | labels are anchord's state and survive it |
