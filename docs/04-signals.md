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
  watcher for anything live that lacks one.
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
input. Claude Code rings it; so do most of the others; so does a shell with
a failed completion, which is the problem.

```text
bytes.contains(&0x07)  →  last_bell_ms = now
```

It is one byte with no structure, so it carries no reason and cannot be
attributed. It is a **hint that something changed**, and on its own it is
not enough to flash a pad amber for ten minutes.

### 2. Quiet — "it stopped working"

```text
now - last_output_ms >= IDLE_THRESHOLD  →  idle
```

teleport uses 30s. An agent mid-task emits something — a spinner, a tool
call, a token — far more often than that, so quiet means *it is not
currently thinking*. Combined with a recent bell, quiet is a strong signal:
**bell, then silence, means it asked and is waiting.**

```text
needs_answer  =  bell_since(last_output) && quiet_for(> 2s)
```

The 2s is not the 30s idle threshold. Attention wants to fire fast — the
whole point is that you look up — and "nothing since the bell" is a much
cheaper claim than "idle".

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
when the agent is waiting on the user, writing a line to a socket or
FIFO that tallyd reads.

```text
~/.claude/settings.json  →  hook: Notification → tally signal --session $X needs-input
```

This is **strictly better where it applies and useless everywhere else** —
it does not exist for codex, for a plain shell, or for an agent somebody
installs next month. So:

- the heuristic is the floor, and always runs
- a hook, when configured, **overrides** it for that session
- nothing in the surface layer knows which one it got

Design the state as `Attention { source: Derived | Declared }` from the
start. Retrofitting an authoritative signal onto code that assumes scraping
is a rewrite; accepting one into code that already has the variant is a
match arm.

## The state a pad renders

```rust
enum Slot {
    Unbound,
    Bound,                       // a spec, nothing running
    Starting,                    // created, no output yet
    Working,                     // output recently
    Quiet,                       // no output for a while, no bell
    NeedsAnswer { since: i64 },  // bell then silence, or declared
    Exited { code: i32 },
    Lost,
}
```

`Lost` is its own variant and not `Exited { code: -1 }`. anchord restarted;
the process may well still be running, orphaned. Rendering that as a clean
exit tells a lie at exactly the moment the user needs the truth.

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
