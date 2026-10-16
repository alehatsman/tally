# 04 — Signals

The hard part of this project is not lighting an LED. It is knowing *which*
LED to light amber, and being right often enough that you trust it.

## What anchor gives us, and what it does not

anchor reports `state` — `running`, `closing`, `exited`, `lost` — and
nothing about what the process inside is doing. That is deliberate and
correct: "this agent is waiting for you" is a *policy about what a UI
shows*, and anchor's scope doc keeps those in clients.

teleport already hit this and solved it. From
`teleport/daemon/src/anchor_watch.rs:1-14`:

> Four fields teleport's API serves have no anchor equivalent, by decision —
> `title`, `claude_resume_id`, `last_bell_ms`, `idle_since_ms`. […] teleport
> attaches to every session it owns and scans the stream itself. That is one
> observer connection per session, which is cheap: anchor is already
> producing those bytes for its log, and an observer never takes the lease
> or writes.

**Do the same thing.** The shape is proven, the failure modes are
documented, and the code is one directory over.

## The watcher

Per live session, one `attach` with `control: false`:

```text
tallyd ──attach(session, from, control=false)──▶ anchord
    ◀── output frames ───────────────────────
```

Rules, all of them inherited rather than invented
(`teleport/daemon/src/anchor_watch.rs:149-290`):

- **Observer only.** Never `control: true`. The lease belongs to the
  terminal you jumped to; a surface that steals it makes typing stop
  working, from across the room, for no visible reason.
- **A reconcile loop, not a hook on create.** Sessions appear because a
  shell script or teleport made one. Poll `list` every 2s and start a
  watcher for anything live that lacks one — `running` or `closing` only.
  An exited session keeps whatever was derived while it ran, and a `lost`
  one has nothing to read (teleport does the same,
  `anchor_watch.rs:169-172`).
- **Only slots get watchers.** A session with no `tl.slot` is not on the
  grid, so scanning its bytes buys nothing. This is the one place the
  client-side filter ([03](03-model.md#where-the-binding-lives)) has to be
  applied *before* doing work rather than just before rendering.
- **Start from offset 0 the first time.** The signals worth having are
  emitted at startup — an agent sets its terminal title and prints its
  resume link in its opening banner — and the supervisor cannot attach
  before that happened. Starting at the head reliably misses exactly the
  output that matters. teleport's comment on this is a bug report written
  before the bug: *"Starting at the head reliably misses exactly the output
  that matters, which is how this was first written and why the title never
  appeared."*
- **Resume from `scanned_to` after a drop.** A watcher dropped for
  backpressure must not re-read the whole log — worst on exactly the busy
  sessions that caused the drop.
- **A watcher ending is normal.** Exit frame, EOF, or dropped-for-falling-
  behind all end it; the supervisor starts a fresh one if the session is
  still live.

## Deriving state

Four inputs, in increasing order of how much we trust them.

### 1. BEL (`0x07`) — "it wants you"

The terminal bell is what an agent emits when it is done talking and wants
input. Most agents ring it; so does a shell with a failed completion, which
is the problem.

> ⚠️ **Claude Code does not ring it, and this is the floor falling to zero.**
> Measured across 6.6 MB of one real session: 1012 `0x07` bytes, **every one
> an OSC title terminator, zero in GROUND state** — and the same on plain
> `claude` with no wrapper ([14](14-attention.md#1-the-agent-never-rings--confirmed)).
> `bell_pending` is therefore never set and [rule 5](#precedence) is
> unreachable against the agent this grid actually runs.
>
> Everything below is correct and is why we can say *zero* rather than guess
> — the scanner rejected all 1012. It is a working detector with nothing to
> detect. Attention comes from [a hook](#4-a-hook--it-told-us) instead, on
> both edges.

```text
a 0x07 seen in GROUND state  →  last_bell_ms = now
```

**Not `bytes.contains(&0x07)`.** `BEL` is also an OSC *string
terminator*, so `ESC ] 0 ; some title BEL` carries a `0x07` that is not a
ring — and an agent that sets its terminal title, which Claude Code does
constantly, would look exactly like one that asked a question and went
quiet. The pad would flash amber for nothing, which is precisely the
"wrong amber" [M1 counts](#measuring-whether-any-of-this-works).

Telling the two apart needs the OSC state machine in
[signal 3](#3-osc-title--what-it-is-doing) running *before* the bell check,
which is why that scanner is in the attention path rather than only in the
title-scrolling one. teleport has the naive check and is not wrong to —
it only ever stores `last_bell_ms` as a field for a UI to show. tally is
the first consumer to **act** on it, and acting is where the distinction
starts to matter.

It is one byte with no structure, so it carries no reason and cannot be
attributed. It is a **hint that something changed**, and on its own it is
not enough to flash a pad amber for ten minutes.

### 2. Quiet — "it stopped working"

```text
now - last_output_ms >= IDLE_THRESHOLD  →  idle        (IDLE_THRESHOLD = 30s)
```

teleport uses 30s (`anchor_watch.rs:34`). An agent mid-task emits something
— a spinner, a tool call, a token — far more often than that, so quiet means
*it is not currently thinking*. Combined with a recent bell, quiet is a
strong signal: **bell, then silence, means it asked and is waiting.**

> ✅ **This half measured true, and it was doubted.** A `claude` at its main
> prompt with nothing typed emits **0 bytes in 92 s**; the live session on
> pad `1:1` held **42 s flat** between turns
> ([14](14-attention.md#2-the-agent-does-go-quiet--04-is-right-and-4-is-wrong)).
> Quiet is reachable, rule 6 is honest, and motion does mean producing
> output.
>
> **A wrapper can take that away.** `isayes` repaints a footer and blinks a
> cursor about twice a second, so under it `last_output_ms` never ages, the
> pad breathes forever and `Quiet` is unreachable. That is a bug in the
> wrapper's output, not a colour to delete here — but it is the reason the
> clearing rule below cannot lean on output.

### Stating that precisely, because the obvious phrasing is unimplementable

"Bell since the last output" cannot be written as a timestamp comparison.
**The BEL arrives inside an output chunk**, so the chunk that carries it
sets `last_bell_ms` and `last_output_ms` to the same instant — teleport does
exactly this (`anchor_watch.rs:225-283`) and never compares them, because it
only serves the two fields to a UI. tally is the first consumer that has to
decide, and `>=` versus `>` on two equal numbers flips the answer.

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
  Without it the pad would flash amber 2s later, every time, which is the
  single easiest way to make the signal untrustworthy.
- **Whitespace never cancels.** The bell is almost always followed by a
  newline before the prompt — sometimes in the same chunk, sometimes in the
  next one. Treating that newline as "it kept going" means the signal never
  fires at all, and which chunk it lands in is a buffering accident nobody
  should be able to observe.
- **`bell_pending` is cleared by output, not by a timer.** It stays true for
  as long as the agent stays silent, which is the desired behaviour: a pad
  that has been waiting for you for ten minutes should still be amber.
  Clearing it is [answering](#clearing-attention).

The 2s is **not** the 30s idle threshold, and the two serve different
masters. Attention wants to fire fast — the whole point is that you look up
— and "nothing since the bell" is a much cheaper claim to make than "this
session is idle".

### Clearing attention

What clears it depends on which signal raised it, and the two are not the
same rule. An earlier draft of this section gave one rule for both and it
was wrong in a way nothing caught, because **the clearing write it
describes was never implemented** — `tl.attention` had a reader and no
remover ([14](14-attention.md#3-nothing-clears-tlattention--a-gap-not-a-bug-report)).

A **derived** attention — a pending bell — is cleared by output, and that is
the flag rule above rather than a separate mechanism.

A **declared** attention is cleared by a hook, because output cannot be
trusted to mean anything: under a repainting wrapper it arrives twice a
second and would clear the label within half a second of the hook setting
it. Two writers disagreeing at 2 Hz is worse than one writer that can be
absent.

| what happens | what it clears |
|---|---|
| any non-whitespace output | a pending **bell** — it answered, or it started talking again |
| `UserPromptSubmit` / `PreToolUse` hook | a **declared** attention — see [below](#4-a-hook--it-told-us) |
| the session exits | both: terminal states outrank attention ([precedence](#precedence)) |
| `tally mute` ([11](11-cli.md)) | neither — mute suppresses the *rendering*, leaving the state underneath |

Notably **jumping to a slot does not clear it.** Looking at a window is not
answering the question in it, and a pad that goes quiet because your eyes
moved is a pad that has started guessing.

### 3. OSC title — "what it is doing"

Agents set the terminal title (OSC 0/2). teleport's
`daemon/src/session/osc.rs` is an incremental scanner for exactly this,
chunk-boundary-safe, with OSC 8 hyperlink extraction for Claude Code's
resume id riding along. Port it.

The title is what the [title scroll](05-interaction.md#the-title-scroll)
displays, and it is the one place a pad can say something in words.

Its own doc comment is the honest caveat, and it should be repeated here:

> both rest on Claude Code's own current terminal output format, which is
> not a documented or versioned contract. A future CLI release changing or
> dropping either sequence degrades silently back to "no title" / "no resume
> id" — never a parse error, never a broken session.

Same rule applies to everything in this document. **No signal may ever
become load-bearing enough that losing it breaks the surface.** Lose the
title, lose a word. Lose the bell, lose the amber — the pad is still green
and still jumps.

### 4. A hook — "it told us"

The heuristics above are scraping. There is an authoritative channel
available for Claude Code specifically: a `Notification` hook that fires
when the agent is waiting on the user.

**It writes a label, not a message to us.**

```text
~/.claude/settings.json
  Stop, Notification  →  tally attention on
  UserPromptSubmit    →  tally attention off
  PreToolUse          →  tally attention off
```

```text
tallyd, on the tick it already makes:
  tl.attention present  →  NeedsAnswer { source: Declared }
```

> **Both edges are hooks, and `$ANCHOR_SESSION` does not exist.** An earlier
> version of this block wrote the hook as
> `anchor label $ANCHOR_SESSION tl.attention=1`; that variable is set by
> nothing, appears nowhere in anchor's source, and the line expanded to an
> empty argument. `tally attention` resolves the session by walking its own
> process ancestry to a pid anchord recorded
> ([14](14-attention.md#tally-attention-onoff), [11](11-cli.md)).
>
> The falling edge is a hook too, for the reason given in
> [Clearing attention](#clearing-attention). `Stop` carries the common case
> — the turn finished — because permission dialogs are mostly auto-answered
> and `Notification` rarely fires; `PreToolUse` catches the answer to one
> that did, which submits no prompt.

The hook talks to **anchord**, which tallyd is already polling. That buys
more than it looks like:

- **No new channel.** No socket, no FIFO, no framing, no error codes, no
  reader task, no reopen-on-EOF — which is what
  [08-lifecycle.md](08-lifecycle.md#one-process-no-socket) refuses on
  principle and would otherwise have to make an exception for.
- **It survives tallyd.** Declared attention outlives a tallyd restart
  because it is in the same place as the binding
  ([03-model.md](03-model.md#where-the-binding-lives)). A FIFO's contents do
  not.
- **teleport can see it too.** Two surfaces, one fact, no integration.
- **The hook needs one subcommand of ours.** This *is* a reason for the
  CLI, and the second one it has ([11-cli.md](11-cli.md)): `anchor label`
  needs a session id, and nothing in a hook's environment carries it. The
  measured gap is [14](14-attention.md#how-it-finds-its-session); if anchor
  ever injects the id, `tally attention` becomes a one-line wrapper rather
  than wasted work.

The cost is latency: one reconcile tick, ≤2s. That is the same 2s the
derived signal waits anyway, so declared attention is never *slower* than
the floor it replaces.

The clearing write is **also a hook's**, and this is the sentence that
changed. It used to read that tallyd would clear on output, on the grounds
that it already watches the stream — a clearing write that was never
implemented, and that would have been defeated anyway by any agent whose UI
repaints on a timer. The hook does get a second callback when you answer:
`UserPromptSubmit`, or `PreToolUse` when the answer was a permission
dialog.

This is **strictly better where it applies and useless everywhere else** —
it does not exist for codex, for a plain shell, or for an agent somebody
installs next month. So:

- the heuristic is the floor, and always runs — **and against Claude Code
  that floor is zero**, which is the measurement that promoted the hook
  from *Later, unscheduled* to the only thing that makes the attention half
  of this product exist ([14](14-attention.md))
- a hook, when configured, **overrides** it for that session — `tl.attention`
  present means `NeedsAnswer` whatever the bytes say, and its absence never
  means "not waiting", only "nobody declared"
- nothing in the surface layer knows which one it got

Design the state as `Attention { source: Derived | Declared }` from the
start. Retrofitting an authoritative signal onto code that assumes scraping
is a rewrite; accepting one into code that already has the variant is a
match arm.

## The state a pad renders

```rust
enum Slot {
    Empty,                       // nothing here, or nothing left to do
    Working,                     // alive, producing output
    Quiet,                       // alive, idle
    NeedsAnswer { since: i64, source: Source },
    Failed { code: i32 },        // exited non-zero
    Lost,
}

enum Source { Derived, Declared }
```

Six variants, because **the grid shows what is alive or what is wrong**
([05](05-interaction.md#the-colour-language)). A session that exited cleanly
is `Empty`: it needs nothing, its slot is free, and the row stays in anchor
for `anchor logs` without occupying a light.

`Lost` is its own variant and not `Failed { code: -1 }`, and anchor is
precise about why: **`EXITED` means the daemon can say how the session
ended; `LOST` means it cannot** (`anchor/docs/02-model.md`). Two reasons
reach it — `daemon_restart`, where the PTY died with the daemon and whatever
the child did last was never seen, and `kill_timeout`, where it was killed
and never observed exiting, so the process may still be out there. Rendering
either as an ordinary failure tells a lie at exactly the moment the user
needs the truth, and it is a lie told *sixty-four times at once*, because
a daemon restart loses every running session together.

A terminal row carrying **no exit code** is therefore `Lost` rather than a
clean exit. "anchor has no account of how this ended" is the definition.

Mute is **not** a variant. It is an orthogonal bit read from `tl.mute`, and
it suppresses exactly one thing — the `NeedsAnswer` rendering — leaving the
slot to render as `Working` or `Quiet` underneath. A muted agent is still an
agent you can see is alive; mute means *stop shouting*, not *disappear*.

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
same either way ([05](05-interaction.md#why-it-is-one-sentence)); the spec
matters when you press, not when you look.

`closing` is deliberately **not** on that list: it falls through to the live
states at 7 and 8. A session anchor is terminating is still alive, and it
has **no exit code yet** — `exit_code` is `None` until the child is reaped —
so rendering it as `Exited { code }` would mean inventing a number. An
earlier draft of this table grouped it with `exited`; writing the code is
what surfaced that the variant could not be constructed honestly.

Three of those placements are decisions, not orderings:

- **Terminal states outrank attention.** An agent that rang the bell and
  then died is `Exited`, not amber. There is nothing to answer.
- **There is no `Starting`.** A freshly created session with no watcher yet
  falls to `Quiet` — alive and idle, which is true. The state existed to
  render "hold on" in yellow for about a second, and off-to-green already
  says the press landed.
- **Muting falls through rather than short-circuiting.** Dropping to
  `Working`/`Quiet` is what makes a muted slot still legible.

## What we deliberately do not do

**No VT emulation.** Not parsing the screen, not diffing a rendered buffer,
not pattern-matching prompts. anchor's own CLI refuses to emulate for the
same reason — the user's terminal already is one — and a surface that got
its state from screen-scraping a TUI would break on every agent release.

**No per-agent plugins.** A registry of "how to detect state for agent X"
is a maintenance treadmill (herdr ships 15 of them). The floor is generic;
the ceiling is a hook the agent itself calls.

**No guessing at reasons.** The pad says *needs you*. It does not say what
it needs. Finding that out is what the jump is for.

## Measuring whether any of this works

[M1's exit criterion](../ROADMAP.md#m1--the-grid-tells-the-truth) is "a week
of real use, counting every time the amber was wrong", and
[09-testing.md](09-testing.md#what-a-green-suite-does-not-prove) says to keep
that count. A count you keep by remembering is a count you do not keep.

So tallyd appends one line per attention transition to
`~/.local/state/tally/attention.jsonl`:

```json
{"ms":1760000000000,"session":"01J…","slot":"work:3:5","to":"needs_answer","source":"derived","title":"edit tests/pty.rs?"}
{"ms":1760000061000,"session":"01J…","slot":"work:3:5","to":"answered","after_ms":61000}
```

Transitions only, so it is a few lines a day, not a log. It exists to answer
one question at the end of the week — *how many of these did I look up for
nothing* — and it is the only piece of state tallyd writes that is not
rebuildable from `anchor list`. Delete it when M1 exits.

**tallyd reads it back at startup**, and that is a change from the first
draft, which said nothing did. An in-memory map of who is asking does not
survive a restart, so a slot still waiting got a second rising edge and an
`after_ms` measured from the wrong one — three rises against one fall in the
file as shipped. The instrument has to be right more than it has to be
write-only ([14](14-attention.md#the-journal-is-miscounting-and-it-is-restarts)).
