# 25 — The squad is a row

[24](24-the-tally-server.md) gave tally its own tmux server and settled the
layout: **a row is a session, a column is a pane.** This document spends
that, and the thing it spends it on is not a feature — it is a unit of
work.

> **A squad is a row.** Column 1 is the master, columns 2…8 are members,
> and the Launchpad row you are looking at *is* the roster, lit.

No new address space, no registry, no identity scheme. The grid already
knows who is alive; a squad is what you get when you agree that eight pads
side by side mean something together.

## The feature already shipped, and it is not this one

Claude Code shipped **agent teams** in February 2026
([docs](https://code.claude.com/docs/en/agent-teams)). A lead spawns
teammates; each teammate is a full session with its own context window;
each gets a tmux split pane; they coordinate through a shared task list and
per-agent JSON inboxes under `~/.claude/teams/`, and they message each
other directly rather than relaying through the lead. Teammates do not
inherit the lead's history.

That is *exactly* the sketch this document started as — spawn N, hand out
identities, keep every context tight — and re-deriving it would be work
with a negative return. So the honest table:

| | agent teams | a tally squad |
|---|---|---|
| N isolated contexts | ✅ | ✅ |
| peer messaging | ✅ | ✅ |
| a pane each | ✅ | ✅ |
| **the lead operates a member's lifecycle** — compact, kill, respawn | ❌ | ✅ |
| **a member outlives its lead** | ❌ `/resume` does not restore in-process teammates | ✅ |
| **a member is addressable by a coordinate on a physical object** | ❌ | ✅ |
| **a member has its own working tree** | ❌ | ✅ |

**The claim is lifecycle, not orchestration.** A teammate is a child of a
session and dies with it. A squad member is a pane in `r07` on tally's
socket, created by `anchor`, outliving every process that ever spoke to it
— including its master. That is the only part worth building, and it is
built out of pieces that already exist: `anchor create`, `tl.slot`,
`remain-on-exit`, respawn-in-place ([19](19-the-dead-pane.md)).

If the answer to *why not just use agent teams* is ever "because ours
spawns more agents", stop and use agent teams.

## What was measured

Against live tmux **3.6b**, same machine as [24](24-the-tally-server.md),
on socket `squadspec`, 2026-10-17. The question was how a master puts text
into a member's pane, because every claim in this document rests on that
one syscall path.

The received wisdom is that `send-keys` mangles text, that
`load-buffer` / `paste-buffer` delivers it in one safe chunk, and that
this is what stops a leading `/` from being eaten by the completion menu.
**All three are false**, and the thing that actually works is a fourth
command nobody mentions.

✅ **`send-keys -l` is byte-exact.** Payload
`brief; then "quote" & $PATH ✓` — a tmux command separator, shell
metacharacters, a dollar sign, and a multibyte glyph — arrived at a raw
reader byte-identical to the file it came from. The `;` did not split the
tmux command because the shell passed it as one argument. **The escaping
hazard is shell quoting, not tmux.**

✅ **`paste-buffer` is byte-exact too**, over a nastier payload: a leading
`/`, backticks, an em-dash, `日本語`, and an embedded newline. 98 bytes in,
98 bytes out, `cmp` clean.

❌ **`paste-buffer` buys no atomicity.** A 9000-byte payload reached the
pty in **nine reads, max 1022 bytes, identical chunk-for-chunk** under both
methods. "One chunk instead of a keystroke stream" is not a thing that
happens. tmux drains its input the same way either way.

✅ **The only difference between the two is `-p`.** Only `paste-buffer -p`
wraps the run in the bracketed-paste markers `ESC[200~` … `ESC[201~`.
Plain `paste-buffer` is byte-identical to `send-keys -l`, so every
argument for one over the other comes down to a flag almost nobody
quotes.

✅ **Claude Code's TUI requests bracketed paste.** Sniffed through a pty
with no prompt sent and the process killed at eight seconds — zero tokens
spent: `ESC[?2004h` appears within the first 1332 bytes of startup.

So `-p` is the only flag that makes a paste look like a paste. **It is
also not enough**, which the spike below found out.

### The spike, run

Against Claude Code **v2.1.236** in a tmux pane on socket `squadspec`,
2026-10-17, in a scratch directory. The question
[cmux#1](https://github.com/wildware-uk/cmux/issues/1) raises is whether a
leading `/` opens the completion menu and whether `Enter` then picks from
it instead of submitting. Both halves were tested directly.

❌ **`-p` does not suppress the completion menu.** `/compact` pasted inside
`ESC[200~`…`ESC[201~` opened the menu anyway: six entries, `/compact` at
the top. Bracketed paste changes how the bytes arrive; it does not change
what the composer does with them.

❌ **`Enter` with the menu open selects the highlighted entry, not the
typed text.** `/sta` pasted under `-p`, then one `send-keys Enter`, and
`/status` ran — the full command, from a three-character prefix. A master
pasting a brief that happens to start with `/` would silently send
something it did not write. This is the failure cmux reported, and it
survives `-p` intact.

✅ **An embedded newline does not submit — under either method.** The
payload `line one of the brief\nline two must not submit` produced **two
lines in the composer and no submission**, both via `paste-buffer -p` and
via `send-keys -l`. The other half of the folklore is false too.

✅ **`Escape` dismisses the menu and keeps the text.** After `/sta` + menu
+ `Escape`, the composer still read `❯ /sta` with no menu.

✅ **`Escape` with no menu open is harmless.** `plain prose brief with no
slash` pasted, `Escape` sent, text intact. Its only effect is leaving the
composer's INSERT mode, which does not stop the next step.

✅ **`Escape` then `Enter` submits verbatim.** `/sta` came back as
`Unknown command: /sta. Did you mean /btw?` — proof the literal text was
submitted rather than a menu entry. A prose payload through the same path
submitted and was answered normally, **from normal mode**, so the lost
INSERT mode costs nothing.

**The mechanism is `Escape`, not `-p`.** The document said otherwise one
revision ago, on the strength of a plausible chain — the TUI requests
bracketed paste, so bracketed paste must be what tames it — and the chain
was wrong at the last link. `-p` stays because it is free and it is what
the TUI asked for; it is not what makes this work.

### The injection rule

```sh
tmux -L tally load-buffer  -b sq <brief-file>
tmux -L tally paste-buffer -p -d -b sq -t "$pane"
tmux -L tally send-keys    -t "$pane" Escape   # dismiss the completion menu
tmux -L tally send-keys    -t "$pane" Enter    # submit is always separate
```

**Four commands, and `Escape` is not optional.** It is unconditional on
purpose: a master cannot know whether the brief it is pasting will open a
menu — any leading `/` will, and a brief is a file somebody else wrote —
and `Escape` with no menu was measured to cost nothing. A conditional
`Escape` would need the master to read the pane back and decide, which is
a race against a TUI redraw.

**The payload never ends in a newline.** Not because a newline submits —
it does not — but because the submit must be the step that is visibly
separate, so that a brief which goes wrong goes wrong *in the composer*,
where `capture-pane` can still see it.

Reading back is `capture-pane -p -t "$pane"`, which is what
[04](04-signals.md) already does for the title.

## The spec

Everything is named by address, the rule from
[24](24-the-tally-server.md#the-naming-spec), and nothing here invents a
second one.

| thing | what it is |
|---|---|
| squad | an absolute row, `r01`…`r64` — the session [24](24-the-tally-server.md) already creates |
| master | column 1 of that row, by convention and nothing stronger |
| member | columns 2…8, lazily created panes, same as any view |
| roster | `anchor ls --label tl.slot` filtered to the row — **not a file** |
| brief | a file on disk, one per member, path handed in at spawn |
| ledger | one file per squad: slot → brief path → status |

**Master is column 1 by convention, deliberately.** Nothing enforces it,
nothing breaks without it, and a row with no master is just a row of
agents — which is what every row is today. A rule that needs enforcement
is a rule that needs a registry, and [08](08-lifecycle.md) already spent
its budget saying no to those.

### Four, not eight

A row holds eight. **Four is the default.** Reported experience across
several independent write-ups converges on three-to-five as the point past
which teammates scatter rather than parallelise, with token cost scaling
roughly linearly — three focused agents beating five unfocused ones. Eight
is the capacity of the furniture, not a target.

```sh
tally squad <row> [-n 4] [--brief <file>]
```

The remaining columns stay dark, which is the grid doing what it does:
**dark means nothing is there**, the same rule as everywhere else.

### The roster is the grid, not a broadcast

The tempting design — master spawns everyone, then immediately tells every
member who everyone else is — is wrong twice. It is N² text through N
context windows, and it is **stale the moment one member dies**, which is
the one event a squad must survive.

**Identity comes from the grid, on demand.** A member already knows its
own address: `tl.slot` is on its session and has been since
[slot.rs](../src/slot.rs). When it needs a peer, it asks. What it asks is
the same `anchor ls` that `tallyd` reconciles against, so a roster read at
10:04 is true at 10:04 and nobody is carrying a copy from 09:30.

This is the same correction as [24](24-the-tally-server.md#the-rule-underneath-it):
*match on the identity that survives the boundary.* A broadcast roster is
a derived copy. The grid is the thing.

## The MCP question, and doc 08's answer to it

The obvious next move is an MCP server, so a master can read the grid and
drive panes as tool calls instead of shelling out. The obvious next move is
also **a new socket and a new protocol**, and
[08](08-lifecycle.md#one-process-no-socket) rejected exactly that:

> *The thing that already has a socket gets to keep being the only one.*

That rule was written about a `tallyd` control socket and it applies here
without amendment. An MCP server survives it on **one** condition: it holds
no state. A read is `anchor ls` and nothing else; a write shells out to
`tally spawn` / `tally purge` / `tally mute` or to the three tmux commands
above. Four tools, no cache, no registry, no second source of truth:

```text
tally_roster()           -> [{slot, cwd, command, state, attention}]
tally_send(slot, text)   -> load-buffer / paste-buffer -p / Enter
tally_compact(slot)      -> the same path, payload "/compact"
tally_spawn(slot, …)     -> tally spawn
```

**The moment it caches a roster it is a control socket wearing a hat**, and
08 says kill it.

It is also not the first thing to build. [11](11-cli.md) opens by crediting
a deferral — *"defer until something is actually awkward"* — and the
deferral paid. The master drives panes with Bash first; the MCP exists only
if that is genuinely awkward, and the awkwardness gets written down before
the server does.

## Worktrees are not optional

Four agents editing one checkout is the failure this repository has already
paid for, three broken builds in one night, and a squad multiplies it by
`n`. So it is a property of spawning, not advice:

```text
~/worktrees/<repo>/<squad>-c<n>
```

**Per-member worktree at spawn time or the squad does not spawn.** It
cannot be retrofitted once members are writing, which is the whole reason
it is a spawn-time flag and not a convention.

This is the one place a squad adds a step `tally spawn` does not have
today. `spawn` takes the cwd from the shell it ran in
([24](24-the-tally-server.md#what-this-costs)); `squad` takes the *repo*
from the shell it ran in and gives each member a different directory
underneath it. The shell is still the thing that knows.

⚠️ **Permissions are inherited and that is a blast radius.** Teammates
start with their lead's permission settings — which, for four unattended
agents at a desk you are not sitting at, is a different proposition than
one agent you are watching. `squad` takes a narrower mode than the master
runs in, and the default is the narrow one.

## Compaction is the blunt instrument

`/compact` into a member is the gesture that started this document, and it
should be the second choice. It is lossy and you cannot see what it
dropped.

**Purge and respawn from the written brief is deterministic.**
[18](18-the-launch-column.md#what-they-were-and-what-that-was-worth)
found respawn unreachable only because there was no spec to resolve;
anchord now keeps `cwd`, `command`, `args` and `login_shell` per row, dead
rows included. A member with a brief on disk has a spec by construction.
So the squad's answer to a full context is: kill the pane, respawn in
place, paste the brief — same `%id`, same `pane_left`, same column, which
[24](24-the-tally-server.md) measured.

That is also why the master holds a **ledger of paths and not of work**.
Members write results to files; the master reads filenames. A master whose
own context fills with what its members produced is a master that needs
compacting, and then nobody is holding the thread.

## What this costs

**A second notion of "what a row means."** [16](16-the-queue.md) killed
`row = project` and this does not revive it — a squad is something you
*make* on a row, not something a row *is*, and a row with four unrelated
agents on it is still legal and still correct. But the eye will read four
lit pads in a line as a squad whether or not it is one, and that is a real
cost of putting the roster on the furniture.

**`>` and the squad disagree about arguments.**
[18](18-the-launch-column.md) drops `args` when spawning into a row, on
purpose, so a scene-launch cannot clone a conversation. A squad spawn must
pass a brief, which is an argument. These are not in conflict — `>` is
*another one like this*, `squad` is *this specific job* — but the two paths
into a row now behave differently and that is a thing to remember rather
than feel.

## What does not change

- the slot model, the addresses, the colours, the queue, the tempo
- `@tally_session` as the identity, `@tally_slot` as the address
- the kill chord, the launch column, purge, mute
- `anchor ls` as the only roster, and the reconcile tick as the only clock
- **no IPC between our own binaries** — the MCP server, if it is ever
  built, is a translator and not a participant

## Open questions

1. ~~**The bracketed-paste spike.**~~ **Run.** `-p` does not suppress the
   menu and `Enter` picks from it; `Escape` is the fix and it is in the
   rule above. What is still unmeasured is whether `Escape` keeps that
   behaviour across Claude Code versions — it is TUI behaviour, not a
   protocol, and nothing promises it.
2. **Does the master need a pane at all?** Column 1 is a convention here,
   but the master is the one agent you actually talk to — it may belong in
   your own terminal on your own socket, with the squad on tally's. That
   would mean a jump that crosses sockets, which
   [24](24-the-tally-server.md#what-this-costs) explicitly refused.
3. **What ends a squad?** Kill is per pad. There is no gesture that means
   *this row is finished*, and `purge` is about ended rows rather than live
   ones.
4. **Whether four is four.** The number is borrowed from other people's
   write-ups, not from `attention.jsonl`. The instrument that measured the
   top row ([16](16-the-queue.md#the-instrument)) can measure this, and
   until it does the default is a guess with a citation.
