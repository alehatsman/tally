# 25 — The squad is a file, the row is its face

**Status: spec, not built.** Nothing in `01`–`24` changes if this is never
implemented.

[24](24-the-tally-server.md) gave tally its own tmux server and settled the
layout: **a row is a session, a column is a window.** This document spends
that, and the thing it spends it on is not a feature — it is a unit of
work.

> **A squad is a ledger file.** The row is where it is displayed, the
> members are the panes it names, and the master is the pad on the left
> that you talk to. It is finished when every result it names exists.

This document has been wrong twice about the same thing, in opposite
directions, and both corrections are kept because the middle is where it
landed.

**Draft one said *a squad is a row*, with column 1 as the master** — the
furniture deciding the design. Three of its four open questions turned
out to be one question, *where does the squad actually live*, and the
answer was *in a file*.

**Draft two over-corrected** and put the master outside the grid
entirely, on the grounds that bytes cross sockets so it could live
anywhere. True, and beside the point: you **walk with** the master, and
walking with something here means pressing its pad. A participant you
cannot jump to is the one part of a squad that is not on the desk.

So: **the squad lives in a file, and the master lives in column 1.**
The grid is still an attention surface rather than an orchestration one —
it shows you the squad and gets you to the master, and the orchestration
happens in a conversation, which is where it was always going to happen.

No new address space, no registry, no identity scheme. The grid already
knows who is alive; a squad is what you get when a file says which of them
belong together.

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
                                               # settle
tmux -L tally send-keys    -t "$pane" Escape   # dismiss the completion menu
                                               # settle
tmux -L tally send-keys    -t "$pane" Enter    # submit is always separate
```

**Four commands, and `Escape` is not optional.** It is unconditional on
purpose: a master cannot know whether the brief it is pasting will open a
menu — any leading `/` will, and a brief is a file somebody else wrote —
and `Escape` with no menu was measured to cost nothing.

**The payload never ends in a newline.** Not because a newline submits —
it does not — but because the submit must stay a step of its own, so that
a brief which goes wrong stalls *in the composer* where a human can still
be shown what happened.

⚠️ **The gaps are not decoration.** Fired back to back with no delay, the
whole sequence silently does nothing: measured, the brief landed in the
composer and **sat there for thirty seconds across fifty-two polls, every
one `idle`.** The TUI needs time to finish handling a paste before the
next key means what it should. Roughly a second each was enough here; the
number is not the point and should not be tuned — **the point is that the
sequence has a timing dependency, so it cannot be fire-and-forget and must
be confirmed.**

### The defect is the silence, not the fragility

`Escape` is TUI behaviour and nothing promises it across a version bump.
Asking *will it keep working* is unanswerable and therefore the wrong
question. The thing that actually hurts is that **a break is silent**: the
day `Escape` stops dismissing the menu, nothing errors — a member receives
a command nobody wrote, and the first symptom is a wrong result some hours
later.

### The receipt is a status change, not a screenshot

The obvious check is to read the composer back with `capture-pane` before
submitting. That is screen-scraping, which is the same fragility as
`Escape` wearing a different hat — it breaks on the same version bump, for
the same reason.

**There is a supported channel, and the spike found it.** `/status` reports
`Peer address: uds:/tmp/cc-socks/<pid>.sock`; the sockets are real, one per
live session, owned by `claude` itself. The documented surface in front of
them needs no TTY:

```sh
claude agents --json
```

```json
{ "pid": 37659, "cwd": "…/futurumlab/tally", "kind": "interactive",
  "sessionId": "095a8f7b-…", "name": "tally-fd", "status": "busy" }
```

So the check becomes a **post-condition from an API rather than a
pre-condition parsed off a screen**: paste, `Escape`, `Enter`, then poll.
`Escape` is then allowed to break, because the receipt does not depend on
`Escape` being right.

**What it costs and how fast it answers**, measured on the same machine:

| | |
|---|---|
| one `claude agents --json` | **≈550 ms**, and it returns **every** session — poll once per squad, never per member |
| `Enter` → `busy` visible | within a single poll (±0.08 s, inside the sampling noise) |
| how long `busy` lasts | exactly as long as the work: **1.6 s** for a one-second reply, **6.8 s** for a six-second one |

⚠️ **The window can be shorter than the sample.** A reply that completes in
under half a second is never observed as `busy`. So the receipt is
one-sided, and that asymmetry is the rule:

> **`busy` proves the submit happened. Never seeing `busy` does not prove
> it failed — unless it persists.**

Which is how the thirty-second `idle` above was caught, and it is why
`capture-pane` comes back: **not as the check, as the tiebreaker.** When
the receipt never arrives, read the composer — text still sitting there is
a failed submit, an empty composer is a reply that was too fast to catch.

**Probe once per version anyway.** At squad spawn, paste `/sta` and assert
the composer reads `/sta` rather than `/status`, cached against
`claude --version`. It costs one paste and it converts *driven wrong* into
*refuses to drive*. The rule does not need to be stable; it needs to be
validated.

### `waitingFor` is the amber, and it reaches past this document

Forcing a real permission prompt — a `Write` outside cwd, session started
`--permission-mode manual` — moved the status to a third value with a
reason attached:

```json
{ "name": "spike3-92", "status": "waiting",
  "waitingFor": "permission prompt" }
```

Transitions were clean: `busy` at `Enter` +0.0 s, `waiting` at +1.15 s,
back to `idle` the moment the prompt was cancelled.

**This is the signal the README says never fired.** The amber is the bet
this product is built on; it never once lit for Claude Code because Claude
Code does not ring the bell, so [14](14-attention.md) moved attention onto
hooks. Here it is as a first-class field, with a reason, from a documented
command that needs no TTY.

**It is also exactly the kind of finding that gets too little scrutiny**,
because it confirms this document *and* solves somebody else's problem at
the same time. So, plainly, what is not known:

- **one version** (2.1.236), one machine, and **one** `waitingFor` value
  ever observed. The vocabulary is not documented.
- **a question may not count.** Permission prompts are part of the amber;
  an agent asking you something is the rest of it, and that was not tested.
- **`idle` is overloaded.** A fresh session and a finished turn are both
  `idle` — *done and waiting for you* is not distinguishable from *never
  started*, which is the distinction [16](16-the-queue.md) built a queue on.
- ~~the join key is wrong for tally~~ — **it is not, and this was checked
  rather than assumed.** `anchor get` already records `pid`, and it is the
  `claude` process itself: four running sessions matched
  `claude agents --json` pid-for-pid, `login_shell: true` and all, because
  the shell `exec`s. `pane_pid` *is* `anchor attach` in a tally pane, but
  the join never needed it. **The join is free.**
- **550 ms per poll is heavy for a 2 s tick.** A hook pushes; this polls.

**It was measured here and it is decided elsewhere.**
[28](28-the-level-and-the-edge.md) is that page: a hook is an edge and
`status` is a level, they fail in opposite directions, and the level gets
one job — vetoing an amber that the hook raised and never lowered. The
squad reads the same field for its own reasons; neither use makes
`claude agents` a roster.

### Through `anchor attach`, which is the configuration that matters

Everything above drove `claude` running **directly** in a pane. A tally
view is not that: it is `anchor attach <id>` in a pane, forwarding to a
PTY that anchord owns. Whether the injection rule survives that extra hop
was the one unmeasured link under this whole document, so it was measured
— same machine, 2026-10-17, socket `sqspike`, a real `anchor create`
session attached in a real tmux pane.

✅ **The whole chain works unchanged.** `load-buffer` → `paste-buffer -p`
→ `Escape` → `Enter` put `reply with exactly the word ACK` into the
composer, the session went **`busy` on the first poll**, and `ACK` came
back. Four tmux commands, one extra process in the path, no difference.

✅ **The join is confirmed a second time, in the right shape.** The three
pids, read off one pane:

| | |
|---|---|
| `anchor get <id>` → `pid` | **56015** |
| `claude agents --json` → `pid` | **56015** |
| `#{pane_pid}` | 58269 — `anchor attach`, as expected |

This document once asserted the join was impossible *because* `pane_pid`
is `anchor attach`. It is, and the join never wanted `pane_pid`. Measured
twice now, the second time through the exact topology a squad runs in.

⚠️ **Every member meets the trust dialog, and it is invisible while it
does.** A fresh worktree is a path Claude Code has never seen, and
`hasTrustDialogAccepted` is recorded **per project path** in
`~/.claude.json` — zero of the thirty projects on this machine are
worktrees, so this is not an edge case, it is every member of every
squad. Sitting on that dialog, the session is **absent from
`claude agents --json` entirely**, which is the row
[28](28-the-level-and-the-edge.md) measured and the reason that command
can never be a liveness source.

So spawning has a step the first draft did not know about:

> **A squad is not up until every member appears in the array.**

`squad new` answers the dialog — it is a fixed first screen and one
`Enter` — and then **waits for each member to become visible** before it
reports the squad as spawned. It answers it rather than leaving it
because the human just asked for a squad in a repo they are standing in,
and a worktree of your own repo is the least ambiguous trust decision
there is. It is recorded in the ledger that it did, because a tool that
clicks *yes, I trust this* on your behalf should say so in writing.

### What the spike did not find

**No supported way to write into a running interactive session.**
`claude agents` lists and `--bg` dispatches new ones; nothing sends a
message into a session that already exists. Reading a squad is solved and
official. Driving one is still typing at a TUI, which is why the injection
rule above is still the rule.


## The spec

Everything is named by address, the rule from
[24](24-the-tally-server.md#the-naming-spec), and nothing here invents a
second one.

| thing | what it is |
|---|---|
| squad | **the ledger file** — the slots, their briefs and their results |
| row | an absolute row `r01`…`r64`, where the ledger's members are shown |
| member | a pane in that row, same as any other view; columns 2…n are workers |
| master | **column 1** — a pad you jump to, holding the ledger and driving the rest |
| roster | `anchor ls --label tl.slot` filtered to the row — **not a file** |
| brief | a file on disk, written by the **master**, not by you — below |

**The ledger is the only new artefact, and it is the squad.** Everything
else in that table already exists. It is a file for the same reason
[08](08-lifecycle.md) made every other shared fact a file: so that nothing
has to be running for it to be true.

**It holds only what nothing else knows**, which is less than the first
draft assumed — `pid`, `cwd`, `status` and `sessionId` are all readable
live from `claude agents --json`, so none of them are stored:

| in the ledger | because |
|---|---|
| which slots are in this squad | nothing else groups them |
| brief path, per member | the ask |
| result path, per member | the deliverable |
| the paths a member owns | the only merge defence there is — below |
| the completion condition | **only when it is not the default** — below |

That is doc 08's rule applied to the new file: **a derivable fact is not
written down.**

### It is `attention.jsonl` again

The format is not a new decision, because this repository already made it
and shipped it. `src/journal.rs` is **append-only JSONL, replayed in
order, last line per key wins, unparsable lines skipped, missing file is
an empty one** — and every one of those properties is here for the same
reason it is there.

```jsonl
{"kind":"squad","squad":"auth","row":7}
{"kind":"assign","slot":"work:7:1","brief":"…/briefs/c1.md","result":"…/out/c1.md","owns":["src/anchor.rs"]}
{"kind":"assign","slot":"work:7:2","brief":"…/briefs/c2.md","result":"…/out/c2.md","owns":["src/jump.rs"]}
{"kind":"sent","slot":"work:7:1","ms":1760…}
```

`~/.local/state/tally/squads/<squad>.jsonl`, beside the journal, for the
reason [24](24-the-tally-server.md) names row sessions by absolute row:
**state that outlives a process does not live in config and does not live
in a worktree.** A ledger inside a member's checkout would be a file the
squad can commit to itself.

**Append-only is about a dead master, not about concurrency.** A master
killed mid-write leaves a torn last line, and `journal.rs` already decided
what to do with one: skip it. Losing the last assignment is a smaller
error than refusing to read the file.

**The `done` field is usually absent.** Once doneness is a result file
that exists, *every assignment has its result* is the default and is
derivable — so writing it down would break the rule two paragraphs up.
It is there for the squad whose finish line is narrower than its roster,
which is a real case and not the common one.

### One writer, full stop

The first draft said *one writer per field* — the member writes its own
result, the master writes only assignments — and congratulated itself for
needing no locking. It still needed N appenders on one file, which is a
concurrency claim dressed as a design rule.

It is not needed, because **a member never writes the ledger at all**:

> **Done is a result file that exists.**

The member writes its deliverable to the path it was assigned. Nothing
announces it. The master — or the next master — calls `stat`. Doneness
was always derivable from the filesystem, and [08](08-lifecycle.md)'s rule
says a derivable fact is not written down.

**Write to a temp path, `rename` into place.** The one way a file's
existence lies is a half-written one, and atomic rename is the boring
primitive that has fixed this since the 1980s. It is a line in the brief,
not a protocol.

What this buys is larger than it looks:

- **the ledger has exactly one writer**, so append atomicity across
  processes is not a property anything depends on
- **the member needs no tally awareness whatsoever.** It is told to write
  a file. There is no client, no hook, no handshake, and a squad member
  can be `codex`, `isayes`, or a person
- **a dead master loses nothing.** The assignments are on disk and the
  results are on disk; the master was only ever the thing holding the two
  against each other

**The master is the most replaceable part of a squad.** That inverts the
intuition this started from — a master that hands out identities and holds
the thread sounds like the irreplaceable centre. It is not: the thread is
in the ledger, so any new master reads the file and picks it up. The one
component you can kill freely is the one that was supposed to be in
charge.

### Control crosses sockets. Attention does not.

[24](24-the-tally-server.md) refused to let a jump cross sockets, and an
early draft let that refusal cover too much — it put the master in column
1 only so that master and members would share a server.

The refusal is narrower than that. `switch-client` **moves your eyes** and
must not cross. `send-keys`, `capture-pane` and `load-buffer` move
**bytes**, and cross freely — several measurements in this document were
taken by driving a pane on one socket from a session on another.

So *where the master can live* is unconstrained. That turned out to be
the wrong question.

### The master is column 1, because you walk with it

The draft that followed answered the unconstrained question with
"anywhere, so: your own shell." That is defensible and it is wrong, and
the thing that shows it is a sentence about how this gets used:

> *I spawn a squad of four, the leftmost is the master, and **I walk with
> it**.*

**Walking with something on this product means jumping to it.** That is
the gesture the whole device exists for ([06](06-jump.md)), and a master
in a shell somewhere is the one participant you cannot press. It would be
the only part of a squad that is not on the grid, in a product whose
entire claim is that the agents you are running are an object on your
desk.

So: **column 1 is the master, columns 2…n are workers**, and the master
is an ordinary view in an ordinary pane that you reach with an ordinary
press.

| | |
|---|---|
| where | the leftmost column of the squad's row |
| what it is | a pad, a view, a jump target — nothing special to `tallyd` |
| how it drives members | the injection rule, same socket, measured above |
| how it knows it is master | it reads the ledger that names its own `tl.slot` |
| what the grid enforces | **nothing** |

⚠️ **This strains [24](24-the-tally-server.md)'s naming rule and the strain
is worth naming.** *Everything is named by address, never by content* —
and "column 1 means master" is a role read off a position. The line it
stays on: **the ledger says which slot is master; the column is only
where `squad new` puts it.** Leftmost because that is where the eye starts
on a row, not because anything resolves a master by looking at a column.
Move the master to column 5 by hand and the squad still works, for the
same reason a pane dragged to another row still jumps correctly.

**The master discovers itself rather than being told.** It is a session
with a `tl.slot` like every other; it looks for the ledger naming that
slot. No argv handoff, no env var, no identity passed at spawn — the same
correction as [the roster](#the-roster-is-the-grid-not-a-broadcast), which
is that a derived copy goes stale and the grid does not.

### Nobody writes a brief before the squad exists

The earlier draft took briefs at creation: `squad new --brief c1.md
--brief c2.md`. That assumed the human arrives knowing the decomposition,
which inverts the point of having a master at all.

What actually happens:

```text
1.  press         a squad appears on row 7: master, worker, worker, worker
2.  press 7:1     you are looking at the master
3.  you say       "let's group the backlog and work out the streams"
4.  master        reads the backlog, groups it, comes back with:
                  a CLI stream, a desktop stream, a testing stream
5.  master        writes three briefs and sends them to 7:2, 7:3, 7:4
6.  you           keep talking to the master, and only to the master
```

**Step 4 is the work, and it is the reason the master is an agent and not
a script.** Grouping a backlog into streams is judgement. Nothing in this
document should try to specify it; what this document owes it is a way to
*act* on the answer — which is `squad send`, and which is measured.

Three consequences, and the third is the one that costs something:

- **`squad new` takes no briefs**, because none exist yet. It takes a row
  and a count. The repo comes from the shell or from the row, the way it
  already does for `>` ([18](18-the-launch-column.md)).
- **The master writes briefs into the ledger's brief paths** and sends
  them. It is the single writer, which is the rule
  [above](#one-writer-full-stop) already settled for a different reason.
- **`owns` is declared at assign time, not at creation.** The merge
  defence below is unchanged in substance and later in time: the check
  runs when the master assigns, because that is the first moment anyone
  knows what the paths are. An overlapping assignment is refused by
  `squad send`, which is a tool the master is calling — so it is told
  *no*, in a loop it can correct in, rather than a human discovering it
  at integration.

⚠️ **The master is now a thing that can be wrong at scale.** A human who
mis-scopes one agent wastes one agent. A master that mis-groups a backlog
wastes three, in parallel, in their own worktrees. That is not an argument
against the design — it is the argument for the brief being a **file you
can read** rather than a message you cannot, and for `squad status`
answering *what was each of them actually told* without jumping anywhere.

### Four, and four is a guess with a citation

A row holds eight. **Four is the default — one master and three
workers.** Reported experience across
several independent write-ups converges on three-to-five as the point past
which teammates scatter rather than parallelise, with token cost scaling
roughly linearly — three focused agents beating five unfocused ones. Eight
is the capacity of the furniture, not a target.

```sh
tally squad new <row> -n 4 [--brief <file>]…
```

**But `n` is the wrong variable and the ledger already measures the right
one.** The limit is not how many agents exist, it is **how many members one
master can service before the median wait gets long** — and the ledger
records who was sent what and when, which is that measurement for free.
[16](16-the-queue.md#the-instrument) took the same shape of reading off
`attention.jsonl` and it is the same instrument.

So the number is not defended here. It is a default with a citation,
recorded as such, and the week of data that
[`README`](../README.md) already refuses to hurry is what replaces it.

**And the row is a default, not a constraint.** Once the squad is the
ledger, the row is display — and the ledger names slots, which already
carry absolute rows. **A squad spanning rows is already representable and
nothing forbids it.** `tally squad new` puts members on one row because a
row is legible, not because anything requires it; the *gesture* is
row-bound because `>` is, which is a property of the chord and not of the
design. If master throughput turns
out to be the real limit, the eight-wide furniture has no claim on the
number and the design does not have to change to admit it.

The unused columns stay dark, which is the grid doing what it does:
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
no state. A read is `anchor ls` — joined, for members, with
`claude agents --json` — and nothing else; a write shells out to
`tally spawn` / `tally purge` / `tally mute` / `tally squad send`, which is
already the injection rule in one process. Four tools, no cache, no
registry, nothing it is the only copy of:

```text
tally_roster()           -> [{slot, cwd, command, state, status}]
tally_send(slot, text)   -> the injection rule, and it returns the receipt
tally_compact(slot)      -> the same path, payload "/compact"
tally_spawn(slot, …)     -> tally spawn
```

**`tally_send` returns whether the member went `busy`**, because a send
whose result the caller has to go and check separately is a send that will
not be checked.

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

### Isolation is half an answer, and the first draft stopped there

Four worktrees stop four agents corrupting each other's edits. They do
nothing about four sets of changes that have to become one, and a design
that hands you four branches and walks away has moved the problem rather
than solved it. The first draft said the ledger "probably should not grow
a notion of a merge." That is still right, and it is not an excuse to say
nothing.

**The cheap half is prevention, and it happens before any worker runs.**
Each assignment declares the paths that member owns. Two members claiming
the same path is an **assignment that `squad send` refuses** — caught
when the master is handing out work, when the fix is one more sentence of
planning, rather than at integration, when the fix is a human reading
four diffs.

```text
send 7:2  owns src/anchor.rs   ok
send 7:3  owns src/jump.rs     ok — disjoint
send 7:4  owns src/jump.rs     refused: already owned by 7:3
```

The master is the one that gets refused, which is the right audience: it
is mid-plan, it has the context, and it can re-cut the streams. A human
told this at integration time has no cheap move left.

This is the same move [24](24-the-tally-server.md) made with column
order: decide it where the decision is being made, as a rule on the
action, instead of maintaining an invariant something has to keep
checking.

**It does not catch everything, and the exceptions are known.** `Cargo.toml`,
a lockfile, a module list — files every member touches and nobody owns.
Declaring them owned by nobody is honest and is what the ledger records;
it is also the set a human should expect to resolve by hand, which is a
far smaller promise than *the squad merges itself*.

**The landing order is serial, because serial always works.** Members land
in ledger order, each rebasing on what is already in. Four rebases of a
focused diff is boring, and boring is the whole brief
([01](01-scope.md)). A squad that tried to land in parallel would be
building a merge engine, which is the thing this document just said it
would not do.

**`tally squad land <row>` is a CLI verb and will never be a pad.** It
rewrites history in a repository, and the Launchpad's claim is that it
cannot lose anything ([05](05-interaction.md)). The same rule that keeps
worktree reaping off the grid keeps this off it.

## The gesture: hold `User`, tap a row's `>`

A squad you have to open a terminal to start is a squad you will start
once. The surface already spawns one agent with a tap — `>` into a row,
inheriting that row's folder and command ([18](18-the-launch-column.md))
— and the ask is the obvious neighbour of that:

```text
>                     one agent in this row          (shipped)
User + >              a squad in this row            (this document)
Stop + >              end the squad in this row      (below)
```

**One primitive, no new one.** Hold-then-tap with a right-column target is
the grammar [05](05-interaction.md#the-exact-rules-of-a-chord) already
runs on, and the row-end chord below already proposed this exact shape.
`User` is `Top(8)` — **off, nothing, nothing** in
[23](23-gesture-brainstorm.md)'s inventory, and the button that document
expected to stay free.

**It spends the last free button, and that is the honest price.**
[18](18-the-launch-column.md) closed with *the bar for the next idea is
now a button taken from something that works*; `Top(8)` is the one
exception that was still spare, and after this the surface has nothing
left. If `squad` turns out not to earn it, the button comes back.

| | |
|---|---|
| **where** | the row of the `>` that was tapped, filling from the leftmost free column |
| **how many** | four — master plus three |
| **directory** | a worktree each, off the repo the row's newest session is in |
| **lit** | `User` held: rows that could take a squad light; the rest go dark |
| **row 8** | no `>`, so no squad by gesture — the asymmetry [18](18-the-launch-column.md#row-8-is-shell-only-by-rule) already declared |
| **a row with no session** | dark. There is no repo to branch from, the same reason `>` is dark there |

⚠️ **The grid gained a gesture that writes to a repository**, and that is
new. `>` spawns a process; `User + >` creates four worktrees and four
branches. It is still reversible — a worktree with no commits is removed
by `squad reap` and refuses when it is not — but the claim that *the
Launchpad cannot lose anything* now rests on `reap` refusing, rather than
on the gesture having nothing to lose. Stated here because it is the kind
of thing that gets noticed later.

### What this corrects in [23](23-gesture-brainstorm.md)

That document put *spawn four and give them a goal* behind a wall:

> *tally attaches as an **observer** and never holds the control lease, so
> it cannot type into a running session even if it had the text. So
> broadcasting a goal to four already-running agents is out of scope by
> construction, not by preference.*

**The wall is not there.** Measured above, end to end, through
`anchor attach`: the bytes go to a *tmux pane*, which forwards them to
the attach client the human's keyboard would have used. anchord grows no
write path and `tallyd` takes no lease — **`tallyd` is not in this
picture at all.** The thing typing is one agent driving tmux, which is
the same privilege any shell on this machine already has.

What `23` got right and this does not reopen: **the goal does not come
from the surface.** It never does. The human says it to the master in
words, the master writes files, and the clipboard-to-four-agents button
that document called the worst version of this product stays unbuilt.

So `23`'s verdict splits more finely than it could at the time:

| | `23` said | measured |
|---|---|---|
| four new sessions on one goal | free — argv | free, and argv is not how this does it |
| four *running* sessions redirected | **forbidden**, needs the lease | **free** — tmux, not anchord |
| a goal typed on the grid | out of scope | **still out of scope**, and good |

## The command surface

Five verbs, all of them shell. [11](11-cli.md) settled why: the folder
comes from the shell you are standing in, and a squad needs a repo the
same way `spawn` needs a directory.

```text
tally squad new   <row> [-n 4]        write the ledger, make the worktrees, spawn
tally squad send  <slot> <file>       the injection rule, with its receipt
tally squad status <row>              who is busy, waiting, stalled, done
tally squad land  <row>               rebase each member's branch in ledger order
tally squad reap  <row>               remove worktrees; refuses dirty or unmerged
```

**`new` is what `User + >` calls**, and the gesture is the expected way in
— the verb exists because a row with no session has no repo to infer, and
because a squad of three or six is a flag rather than a second chord.

**`send` is the master's verb, not yours.** It is the one in this list a
human will rarely type: the master calls it, gets told *no* on an
overlapping `owns`, and corrects. Everything the master does to a worker
goes through it.

**`new` is the only one that is not re-runnable**, and it is the one that
can refuse: a row with live pads in the way, a repo with no clean base to
branch from, a member that never appeared in the array. Everything else
reads the ledger and the grid, so running it twice is running it once.

**It is a verb rather than only an MCP tool** because the master drives
panes with Bash first, and a four-command rule with two timing
dependencies is exactly the thing not to re-type. It is one process that
does the paste, the `Escape`, the `Enter` and the poll, and exits
non-zero when the receipt never comes and the composer is not empty.

**There is no `tally squad kill`.** Ending a squad is the row-end chord
and `reap` — [below](#what-ends-a-squad) — because killing is reversible
and reaping is not, and they belong on different surfaces for that
reason.

## Compaction is the blunt instrument

`/compact` into a member is the gesture that started this document, and it
should be the second choice. It is lossy and you cannot see what it
dropped.

**Respawn restores the agent; `>` makes a new one.** The distinction
[18](18-the-launch-column.md) half-drew is now sharp, because
`claude agents --json` hands out `sessionId` and `--resume` takes it. `>`
drops `args` on purpose so a scene-launch cannot clone a conversation —
that is *another agent like this one*. Respawn-in-place is the opposite
case: the **same** member, same slot, same brief, and resuming its
`sessionId` restores what it had rather than replaying what it was told.

**Purge and respawn from the written brief is deterministic.**
[18](18-the-launch-column.md#what-they-were-and-what-that-was-worth)
found respawn unreachable only because there was no spec to resolve;
anchord now keeps `cwd`, `command`, `args` and `login_shell` per row, dead
rows included. A member with a brief on disk has a spec by construction.
So the squad's answer to a full context is: kill the pane, respawn in
place, paste the brief — same `%id`, same `pane_left`, same column, which
[24](24-the-tally-server.md) measured.

That is also why the ledger holds **paths and not work**. Members write
results to files; the master reads filenames. A master whose own context
fills with what its members produced is a master that needs compacting —
and because the ledger is on disk, the remedy for that is to kill it and
start another one, not to compact it.

## What ends a squad

Kill is per pad, and `purge` is about rows that have already ended. Nothing
says *this row is finished*, which is the gap a squad makes obvious.

**Hold kill, tap that row's `>`.** The scene-launch buttons are already
row-scoped — `CC89`…`CC29`, one per row, [18](18-the-launch-column.md) —
and kill is already a hold-modifier. The chord reads the way it acts:
`>` means *this row*, kill means *destroy*. It inherits the hold-then-tap
ceremony [05](05-interaction.md#the-exact-rules-of-a-chord) reserves for
destruction, and row 8 cannot be ended by gesture for the same reason it
has no `>` — an asymmetry [18](18-the-launch-column.md#row-8-is-shell-only-by-rule)
already declared and already pays for.

**Ending has two halves and only one of them is a button.**

| | how | why |
|---|---|---|
| kill the processes | hold kill + `>` | reversible — respawn in place exists ([19](19-the-dead-pane.md)) |
| reap the worktrees | CLI, and it refuses when one is dirty or unmerged | holds commits that are nowhere else |

**The Launchpad is fast because it cannot lose anything.** A pad that could
destroy an unmerged worktree is a pad you would hesitate over, and a pad
you hesitate over has already failed — that is the whole argument of
[05](05-interaction.md) applied to a new destructive gesture.

The deeper answer is that **a squad with a ledger already knows when it is
finished**: every brief has a result path or it does not. The gesture is
garbage collection, not a verdict.

### The stalled squad, and why it is not a colour

A row of four live agents that nobody is coordinating looks exactly like a
working squad. With the ledger it stops being ambiguous:

> **a result path that does not exist + every member `idle` = stalled**

Both halves are readable without a master running, and neither needs a
member's cooperation — the assignments replay from the ledger, the
results are a `stat`, and the statuses come from `claude agents --json`.
So *is anyone actually working* is a question with an answer, and the
answer survives the master that was supposed to know it.

**`idle` is the right word only because `waiting` is a different one.** The
three observed statuses separate the three cases a squad cares about, and
they separate them without a master:

| every member | means |
|---|---|
| `busy` | working — leave it alone |
| `waiting` | blocked on you — this is the amber |
| `idle`, briefs unfinished | **stalled** — nobody is working and nobody is asking |

⚠️ And `idle` is still overloaded within that last row: a member that
finished its brief and one that never received it both read `idle`. The
result file is what tells them apart — present means finished, absent
means never worked — which is the argument for
[the done rule](#one-writer-full-stop) restated.

**It surfaces in the CLI, not on the grid.** `tally squad status <row>`.
[16](16-the-queue.md) spent real effort keeping pad colour meaning exactly
one thing, and *your squad is stuck* would be a second meaning for a fact a
file already states. **The grid keeps telling you where to look; the file
tells you whether anyone is working.**

### A third source of truth, named out loud

`claude agents --json` is a reader that is not `anchor ls`, and
[08](08-lifecycle.md) is explicit that the thing which already has a socket
gets to keep being the only one. So this is a rule under pressure, and the
line it holds is:

**anchor stays the roster; `claude agents` is member enrichment that only
the squad reads.**

The two sets are not the same. anchor sees `isayes`, `codex` and bare
shells, and the grid is about all of them; `status` exists only for Claude
members, and a squad is the only thing that needs it.

⚠️ **That line did not survive the day.**
[28](28-the-level-and-the-edge.md) spends it: `waitingFor` is the amber,
the amber is `tallyd`'s business, and `tallyd` is given one read of
`claude agents` — to **clear** a stale `tl.attention`, never to raise one.
The roster is still `anchor ls` alone. The rule bent in exactly one
direction, on a page of its own, which is the only way it was ever going
to bend.

## What this costs

**A second notion of "what a row means."** [16](16-the-queue.md) killed
`row = project` and this does not revive it — a squad is something you
*make* on a row, not something a row *is*, and a row with four unrelated
agents on it is still legal and still correct. But the eye will read four
lit pads in a line as a squad whether or not it is one, and that is a real
cost of showing a squad on the furniture.

**The Launchpad starts a squad and gets you to its master. It does not
drive one.** The intuition this began from was eight pads you command a
fleet with, and two of those three verbs survived: the chord makes the
squad, the pad takes you to the master. The third — *telling them what to
do from the surface* — is not here and should not be. A goal is text, the
grid has no input ([01](01-scope.md)), and every way of smuggling text
onto it is invisible state the surface has rejected three times. The work
is a conversation, and it happens in the pane the press lands in.

**The last spare button is gone.** [18](18-the-launch-column.md) closed
with *the bar for the next idea is now a button taken from something that
works*, with `Top(7)` and `Top(8)` the two exceptions.
[23](23-gesture-brainstorm.md) spends `Top(7)` on the pick-up-and-drop
modifier, this spends `Top(8)`, and the surface is full. Every future
gesture is now a trade.

**The two ways into a row behave differently.**
[18](18-the-launch-column.md) drops `args` when `>` spawns, on purpose, so
a scene-launch cannot clone a conversation. `User + >` is not spawning one
of anything — it is four agents, four worktrees, and a file that groups
them. They are not in conflict, `>` being *another one like this* and the
chord being *a unit of work*, but one button and one button-plus-modifier
now do categorically different sizes of thing, and that is a thing to
remember rather than feel.

## What does not change

- the slot model, the addresses, the colours, the queue, the tempo
- `@tally_session` as the identity, `@tally_slot` as the address
- the kill chord's grammar, the launch column, purge, mute — the row-end
  chord adds a target to kill, not a new kind of gesture
- `anchor ls` as the only roster **`tallyd` reads** — today, and pending
  open question 1 — and the reconcile tick as the only clock
- **no IPC between our own binaries** — the MCP server, if it is ever
  built, is a translator and not a participant

## Open questions

1. ~~**Does `waitingFor` replace the hook, corroborate it, or neither?**~~
   **Answered, and it moved out** — [28](28-the-level-and-the-edge.md).
   Neither: it corroborates, and it is allowed to *clear* a stale amber and
   nothing else. It cannot replace the hooks, because a finished turn —
   the moment [14](14-attention.md) says the grid exists to report — reads
   `idle`, the same string as a session that was never given anything.
2. ~~**Does `waiting` cover a question, or only a permission prompt?**~~
   **Both**, measured: `waitingFor` is `input needed` for an
   `AskUserQuestion` and `permission prompt` for a tool dialog *and* a plan
   approval. A session on the startup trust dialog is absent from the array
   entirely, which is why none of this can be a liveness source.
3. ~~**How does a member tell the ledger it is done?**~~ **It does not** —
   [done is a result file that exists](#one-writer-full-stop). The question
   assumed a protocol, and the answer was to delete the protocol: the
   member writes its deliverable and the master calls `stat`. A hook would
   have bought promptness only, and the master already polls for its
   receipt.
4. ~~**What does a squad do when two members' worktrees conflict?**~~
   **Mostly it prevents them** — [assignments declare the paths a member
   owns](#isolation-is-half-an-answer-and-the-first-draft-stopped-there),
   overlap refuses at creation, and landing is serial. What survives is
   narrower and is the real open question: **the files nobody owns.**
   `Cargo.toml`, a lockfile, a module list — every member touches them, the
   ledger records them as unowned, and a human resolves them. Whether that
   set stays small enough to be a footnote or turns out to be where all the
   work is, is the thing only running a squad can say.
5. **What does `User` held look like?** Every other modifier repaints the
   grid to show what it would do ([05](05-interaction.md#the-held-preview)).
   `User` has seven candidate rows and a four-pad consequence each, which
   is more state than any held preview has had to show. If it cannot be
   drawn in one glance, the chord is wrong and the shell keeps the verb.
6. **Does a master need a brief of its own?** The ledger holds assignments
   and results; nothing holds *why this squad exists*. A master that dies
   is replaceable because the thread is on disk — but the thread is
   currently four tasks with no statement of what they add up to. The
   `done` condition in the squad line is a thin version of it and may not
   be enough.
7. **What happens on the second round?** Every verb here assumes one pass:
   assign, work, land, reap. A real squad discovers that c2's result
   changes what c3 should have been asked. Re-assigning into a live member
   is just `send` with a new brief — but the ledger grows a second
   `assign` line for the same slot, and *which result counts* becomes a
   question the replay rule answers by accident rather than by design.
