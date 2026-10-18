# 14 — Attention, measured

> **Amended by [26](26-the-dead-pane.md).** The hook this
> document designs now reports a second fact on the same call: `tl.claude`,
> the conversation the agent is running.

**Status: shipped** ([M6](../ROADMAP.md#m6--the-amber-exists)). It amends
[04-signals.md](04-signals.md) and
[11-cli.md](11-cli.md), and it exists because the signal the whole product
rests on does not fire against the agent this grid actually runs
([#4](https://github.com/alehatsman/tally/issues/4)).

[M1](../ROADMAP.md#m1--the-grid-tells-the-truth) named one risk and said to
stop if it came back wrong:

> If the attention signal is not trustworthy, stop — the rest of the program
> is a worse fuzzy finder.

It did not come back untrustworthy. It came back **absent**, which is a
different finding and has a different answer.

## What was measured

Three measurements, two of them taken after `04` was written and one of them
correcting the issue that prompted this document.

### 1. The agent never rings — confirmed

1012 `0x07` bytes across 6.6 MB of one real session. **Every one an OSC
title terminator; zero in GROUND state.** Same across four sessions,
including plain `claude` with no wrapper. Full table in
[#4](https://github.com/alehatsman/tally/issues/4).

`src/signal.rs` is correct and is the reason we can say *zero* rather than
guess: the OSC scanner runs before the bell check exactly as `04` requires,
and it rejected all 1012. A naive `bytes.contains(&0x07)` would have
reported 1012 phantom rings.

**Consequence:** `bell_pending` is never set, so rule 5 of
[the precedence table](04-signals.md#precedence) is unreachable. Derived
attention is not weak. It does not exist.

### 2. The agent *does* go quiet — `04` is right and #4 is wrong

#4 claims the stream never falls silent — ~2 emissions per second forever,
a software cursor blink and a footer repaint — and concludes that `Quiet` is
unreachable and the 2 s window unreachable with it.

That is true of the sample it was taken from and **not true of Claude
Code**. Measured 2026-10-16:

| what | idle window | bytes emitted |
|---|---|---|
| `claude`, trust prompt, nothing typed | 60 s | **0** |
| `claude`, main prompt, nothing typed | 92 s | **0** |
| `claude --resume`, live session on pad `1:1`, between turns | 42 s | **0** |

The third is the one that matters, because it is the real session on the
real grid rather than a probe.

The 202 cursor-blink toggles and 205 `auto-approve ON  2 approved  delay 0s`
footer repaints in #4 are **`isayes`'s** — the wrapper's own UI, painting on
its own timer. Claude Code emits nothing at an idle prompt.

**Consequence, and it is the good news in this document:** `Working` versus
`Quiet` is honest, rule 6 works, and motion means producing output — under
`claude`. Under a repainting wrapper the distinction is a lie, and that is
the wrapper's bug to fix, not a colour to delete here.

### 3. Nothing clears `tl.attention` — a gap, not a bug report

`04` promises:

> The clearing write is tallyd's, not the hook's — the hook fires when the
> agent starts waiting and gets no second callback when you answer.

**It was never built.** `LABEL_ATTENTION` appears in the codebase three
times: the constant (`src/anchor.rs`), a reader
(`Session::declared_attention`), and a test. Nothing removes it, anywhere.

So the declared path is half-built — it can be switched on and never off.
A `Notification` hook installed against today's binary sticks a pad amber
permanently. This has to be closed before any hook ships, which makes it a
prerequisite rather than a follow-up.

> ✅ **Closed, in the same milestone.** The three paragraphs above are the
> state of the tree *before* M6 and are kept because the gap is the reason
> the decision below came out the way it did. The falling edge shipped
> with the rising one: `tally attention off` removes the label
> (`src/bin/tally.rs`), on `UserPromptSubmit` and `PreToolUse`. Nothing is
> blocked on this, and a document that says otherwise is reading a
> measurement as a status.

## The decision

**Both edges are hooks.** Not because nothing else could work — measurement
2 says output-clearing *would* work under plain `claude` — but because it
breaks under exactly the wrapper the grid is driven through, and a clearing
rule that depends on which binary you launched is a rule you cannot reason
about from across the room.

```text
Stop, Notification   →  tally attention on  || true   (set    tl.attention=1)
UserPromptSubmit     →  tally attention off || true   (remove tl.attention)
PreToolUse           →  tally attention off || true
```

### `|| true`, and it is not superstition

Claude Code gives **exit 2 a meaning**, and the meaning differs per hook: a
`Stop` hook exiting 2 *blocks the agent from stopping* and feeds its stderr
back to the model; a `PreToolUse` hook exiting 2 *denies the tool call*.
Our exit 2 is a usage error — one mistyped word in a settings file — and
without the guard that typo does not produce a dead pad, it produces an
agent that cannot finish a turn or run a tool, on a machine where the grid
may not even be plugged in.

This is [04](04-signals.md)'s own rule pointed the other way. *No signal may
become load-bearing enough that losing it breaks the surface* — and nothing
on the surface may become load-bearing enough to break **the agent**. tally
is the disposable layer ([08](08-lifecycle.md)); a hook is the one place it
runs inside something that is not.

The honest cost: a real failure — anchord down — stops being visible in the
hook's exit status. It is still one line on stderr, and the grid is already
reporting the same outage as a dark red logo, which is the louder and
correct channel for it.

Three things follow from the labels, and they are the same three that made
a label beat a socket in `04`:

- **No new channel.** tallyd is already polling `list`.
- **It survives tallyd**, because it lives where the binding lives.
- **teleport sees it too.** Two surfaces, one fact.

### Why `Stop` and not only `Notification`

`~/.claude/settings.json` runs `defaultMode: auto` with
`skipDangerousModePermissionPrompt`. Permission dialogs — the thing
`Notification` fires for — are mostly answered without asking. The moment
this grid exists to report is **the turn finishing**: the agent said its
piece and is waiting for the next instruction. That is `Stop`.

`Notification` stays in the list because when it does fire, it is the
strongest possible statement of *wants you*, and it costs one line.

> ⚠️ **Both hooks write the same bit, and a second kind of reader now
> reads it.** `tally attention on` takes no argument, so *blocked on a
> dialog* and *turn finished* are one string by the time anything sees
> them. That is right for the reader this document was measured against —
> a human, for whom both mean *go look*. It is not obviously right for a
> program: [32](32-the-grid-in-text.md) is read by agents, and `tally ls`
> now says `wants you` rather than `waiting for an answer` because the
> latter was false in the common case. Whether the two states should be
> told apart at all is
> [#23](https://github.com/alehatsman/tally/issues/23). **The amber is
> unchanged either way** — nothing below is amended.

### Why `PreToolUse` on the falling edge as well

`UserPromptSubmit` covers answering with words. It does not cover answering
a permission dialog, which fires no prompt submission — so attention set by
`Notification` would outlive the answer and leave a pad amber while the
agent works. `PreToolUse` is *the agent is acting again*, which is the honest
falling edge for that case.

Both are idempotent: removing a label that is not there is not an error.

### Why not output-clearing as a floor

Rejected deliberately. Under `isayes` it clears the label within about half
a second of the hook setting it, so the floor does not back the hook up — it
**fights** it, and silently. A signal with two writers disagreeing twice a
second is worse than one writer that can be absent.

## `tally attention on|off`

[11-cli.md](11-cli.md) accepts that a second subcommand needs a measured
reason. This is one, and the measurement is that `$ANCHOR_SESSION` does not
exist.

```text
tally attention on          set    tl.attention=1 on the calling session
tally attention off         remove tl.attention  from the calling session
```

`04` writes the hook as `anchor label $ANCHOR_SESSION tl.attention=1`. That
variable is set by nothing — it appears nowhere in anchor's source and is
not in a live session's environment. The recipe expands to an empty
argument and has never worked.

### How it finds its session

**Walk its own process ancestry to a pid anchord recorded.**

```text
pid = getppid()
loop:
    if pid matches some session's `pid` in `anchor list`  ->  that session
    if pid == 1 or pid == 0                               ->  not under anchor
    pid = parent of pid
```

Verified: anchord records the agent's pid directly
(`anchor get` → `"pid": 31100`, whose parent is anchord), and a hook is a
descendant of it. No lookup by cwd — two sessions in one repo is the normal
case on this grid today, and cwd cannot tell them apart.

Rejected alternatives, both named in
[#6](https://github.com/alehatsman/tally/issues/6):

- **A lookup inside the hook.** Ships today and costs nothing to build, but
  it is several `anchor` calls on every turn, and the only key a shell
  one-liner has is cwd, which is ambiguous.
- **Ask anchor to inject the session id.** Cleanest, and `04`'s recipe would
  then be literally correct. It changes another repo and has to clear
  [07-seams.md](07-seams.md#what-we-must-not-ask-anchor-for) first. Not
  refused — **deferred**, and if anchor ever grows it, this subcommand
  becomes a one-line wrapper rather than wasted work.

### Interface

| | |
|---|---|
| exit `0` | the label was written, **or** the process is not under anchor |
| exit `1` | anchord was reachable and refused, or the socket is gone |
| exit `2` | usage — anything that is not exactly `on` or `off` |
| stdout | nothing, ever |
| stderr | one line, only on exit `1` |

**Not being under anchor is success, and that is the load-bearing cell.**
The hook runs on every turn of every Claude Code session on this machine,
and all but a handful are not on the grid. A non-zero exit there puts an
error in the user's agent on every single turn, for a surface that is not
even watching. Silence is the only acceptable behaviour.

The same argument covers anchord being down: it is reported, because that is
a real failure, but it is reported once in one line and never to stdout —
stdout belongs to the agent's own transcript.

## Edge cases

| case | behaviour |
|---|---|
| hook fires on a session with no `tl.slot` | label is written anyway; no pad draws it, which is harmless and keeps the subcommand ignorant of the grid |
| session exits with `tl.attention` set | terminal states outrank attention ([04](04-signals.md#precedence)), so the pad renders the exit. No cleanup needed |
| the slot is muted | unchanged — mute suppresses the rendering, not the label |
| `on` twice, `off` twice | idempotent; anchord takes a label write either way |
| two sessions in one repo | distinguished by pid, which is why ancestry and not cwd |
| hook fires faster than the 2 s reconcile | last write wins, which is the correct answer: the latest edge is the true one |
| anchord restarts between `on` and `off` | labels are anchord's state and survive it |

## The journal is miscounting, and it is restarts

[#7](https://github.com/alehatsman/tally/issues/7) reports three
`needs_answer` lines against one `answered` for slot `1:3:3`, and reads it
as either oscillation with a lost falling edge or a rise written when
nothing changed. It is **neither**.

`Journal::asking` is an in-memory `BTreeMap` (`src/journal.rs`) rebuilt
empty on every start. A slot that is still asking when tallyd restarts has
no record of having asked, so the next tick writes a fresh rising edge. The
evidence is in the file:

- `after_ms: 90347` is measured from the **third** rise exactly
  (`1792159699962 + 90347 = 1792159790309`), not the first — so the clock
  restarted with the map;
- the earliest line has no `title` field at all, while later ones carry
  `"title": null` — three rises spanning two binaries.

The two sessions sharing an `after_ms` is the benign case #7 guessed at:
both rose on the same tick and fell on the same tick. `after_ms` is computed
per slot from its own stored rise (`src/journal.rs`), not from a shared
timestamp. Worth stating because the sample cannot distinguish them.

**The fix: read the tail back at startup.** The file is transitions only, a
few lines a day; scanning it for slots whose last transition was
`needs_answer` restores `asking` with its original timestamps, and the
overcount and the truncated `after_ms` both go away.

This contradicts one sentence in `04` — *"the only piece of state tallyd
writes that is not rebuildable, which is acceptable precisely because
nothing reads it back"* — and the contradiction is the point: the file is
the instrument [M1](../ROADMAP.md#m1--the-grid-tells-the-truth) is measured
with, so it has to be right more than it has to be write-only.

A restart during an unanswered attention still loses nothing, because the
label does not live here. If the agent was answered while tallyd was down,
the next tick writes the falling edge with an `after_ms` that includes the
downtime — honest, and visible as such.

### The `title` shape

Absent on old lines, `null` on new ones, same meaning, in a file something
will eventually parse. **`null` is the shape**; it is what the current
binary writes and what the struct declares. Nothing is rewritten — the old
lines predate the field and a reader that treats missing as null is correct
for both.

## What `04` has to say instead

Four amendments, and most of the document survives.

1. **Replace the "Claude Code rings it" premise** with measurement 1, and
   state plainly that the derived floor is **zero** for this agent.
2. **Keep the quiet section unchanged**, and add measurement 2 — including
   that a repainting wrapper defeats it, and that this is the wrapper's bug.
3. **Rewrite [Clearing attention](04-signals.md#clearing-attention).** "Any
   non-whitespace output" is not what clears a declared attention and never
   was; a hook does.
4. **Delete the `$ANCHOR_SESSION` recipe** and point at
   `tally attention`.

What `04` got right and keeps, stated because it is most of the file: the
OSC-before-BEL rule; *no signal may become load-bearing enough that losing
it breaks the surface*, which held exactly as written — the bell is absent
and the grid is still truthful about alive and dead; and the
`Source { Derived | Declared }` split, which is why this is a match arm and
not a rewrite.

## Validation

Hardware, because [09](09-testing.md#what-a-green-suite-does-not-prove) is
this project's standing reminder that a surface fact only shows up on the
surface, and it has been right four times.

- [ ] `tally attention on` from a hook inside a slotted session turns that
      pad amber within one reconcile
- [ ] answering it turns the pad green again — both via `UserPromptSubmit`
      and via a permission dialog, which is the `PreToolUse` path
- [ ] `tally attention on` in a shell that is not under anchor exits `0`
      and prints nothing
- [ ] with anchord stopped, one line on stderr and exit `1` — and the
      agent's own transcript shows nothing
- [ ] a second agent in the same repo gets its own pad, not the other's
- [ ] restart tallyd with an attention outstanding: **one** rising edge in
      `attention.jsonl` total, and `after_ms` measured from the real rise
- [ ] a week of `attention.jsonl` with a number in it, which is the thing
      [M1](../ROADMAP.md#m1--the-grid-tells-the-truth) has been waiting for
      since it shipped

The last one is not a checkbox this milestone can tick, and that is the
point of building it.
