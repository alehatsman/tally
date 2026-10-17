# 11 — The CLI

[10-open-questions.md](10-open-questions.md) deferred this one: *"`anchor ls
--label tl.slot=…` and `$EDITOR slots.toml` cover most of it. Defer until
something is actually awkward."* The deferral paid off — it is awkward now,
and we know exactly which part.

## The folder is the problem

Claiming a pad from a shell is this:

```sh
anchor create --cwd ~/projects/futurumlab/tally --login-shell \
  --label tl.slot=1:1:1 -- claude
```

Four facts, and **three of them the shell already knows.** The directory is
the one that stings, because it is the longest and the one that changes
every time.

The deeper reason is that **cwd belongs to where you were standing, not to
the pad.** `slots.toml` assumes otherwise: it wants the folder declared in
advance, which is a prediction about work you have not done. That is why
the spawn-from-a-dark-pad half of [M3](../ROADMAP.md#m3--spawn-and-the-destructive-chords)
sits unused — not because it is broken, but because the only context that
knows the folder is a shell sitting in it.

So the CLI is not a convenience wrapper. It is **the other half of
spawning**, for the case where the folder comes from you rather than from a
file.

## `tally spawn`

```text
tally spawn     [--detach] [--count N] <addr> [-- command [args…]]
tally fork      [--detach] [--count N] <src> <dst> [-- command [args…]]
tally purge     <addr>
tally mute      <addr> [--off]
tally attention <on | off>
```

An `addr` is `row:col` or `page:row:col`, everywhere.

`purge` and `mute` are here because the grid gave them up
([18](18-the-launch-column.md)), and they could not simply be deleted:
`anchor` has no `purge` subcommand to defer to (anchor#14), and mute is
the starvation hatch for `►` and the tempo ([16](16-the-queue.md),
[17](17-the-tempo.md)). **`mute` takes a flag rather than toggling**: a
shell cannot see the pad, so it says which way it meant.

```sh
$ cd ~/projects/futurumlab/anchor
$ tally spawn 1:1
spawned 01M53KQ… on 1:1:1

$ tally spawn work:3:4 -- cargo watch -x test
spawned 01M53KR… on work:3:4
```

| | | why |
|---|---|---|
| address | **required** | you choose the pad, so you know where to look without reading output |
| page | defaults to the first | two parts is `row:col`, three is `page:row:col` |
| command | defaults to `claude` | the thing this grid is for |
| cwd | **always `$PWD`** | the entire point; there is no `--cwd`, `cd` is the flag |
| login shell | always | you want your `PATH`, and a spawn that cannot find `claude` is not useful |
| view | opened on tally's tmux server, unless `--detach` | a pane on *your* server can never be a view ([24](24-the-tally-server.md)) |

⚠️ **`spawn` used to hand your shell over, and it no longer can.** The
pane you ran it in became the view, which stopped being possible when
views moved to tally's own tmux server — a pane on your personal server
cannot be one ([24](24-the-tally-server.md)). The command now creates the
session, opens the view over there, and **returns to your shell**. The
folder still comes from where you are standing, which was always the half
worth having.

So **`--detach` survives with a narrower meaning**: not *do not adopt my
pane*, which is no longer a thing that happens, but **open no view at
all**. The session runs, the pad lights, and the first press is what
opens it.

| | what |
|---|---|
| `tally spawn 1:1` | creates it, opens the view, prints, returns |
| `tally spawn --detach 1:1` | creates it, opens nothing, prints, returns |
| `--count N`, `N > 1` | creates N, opens nothing — one shell, N views, and picking one is a guess |

**Opening the view is not fatal.** The session exists by the time the
view is attempted, so a tmux that will not cooperate costs you a view the
next pad press makes anyway. It prints where the session went and exits
`0`.

**No `--cwd` is deliberate.** A flag would re-admit the thing we are
removing — a directory typed by hand — and the shell has a perfectly good
one already. Scripts that want a different folder can use a subshell, or
call `anchor create` directly, which stays supported.

## What `purge` and `mute` do, and do not

**`purge` does not kill.** It forgets the *ended* rows behind a pad:
every session anchord reports as exited or lost that carries the pad's
`tl.slot`, rows and logs together, and it closes their views
(`model::purgeable`, `src/bin/tally.rs`). A running session is never in
that set. With nothing ended behind the pad it prints `nothing to purge on
<addr>` and exits `0`, so a script reading the exit code learns nothing
about whether anything is still running there.

```sh
$ tally purge 1:3:1
purged 4 on 1:3:1
```

Freeing a pad that holds a **live** session is a different move, and it is
the one `tally spawn` prints when it refuses:

```sh
$ tally spawn 1:3:1
tally: 1:3:1 is held by 01M53KQ… (running)
tally: if it is really gone: anchor label 01M53KQ… tl.slot- tl.attention-

$ anchor label 01M53KQ… tl.slot- tl.attention-   # let go of the pad
$ anchor kill 01M53KQ…                           # end the session
$ tally spawn 1:3:1                              # the pad is free
```

An agent that reads `tally purge <addr>` as *kill whatever is on that
pad* runs it against a live session, gets a success exit, and the
session keeps running (#11).

**`mute` suppresses attention and nothing else.** It sets `tl.mute` on the
session currently drawn on the pad, so the pad shows the state underneath
(working or quiet) instead of amber, and the pad stops counting toward the
queue head and the tempo. The session is not touched. The label is on the
*session*, not the pad: a fresh spawn there starts unmuted. A dark pad has
no session to carry it, so `mute` refuses with `nothing is on <addr>`.

```sh
$ tally mute 1:3:1
muted 1:3:1
$ tally mute 1:3:1 --off
unmuted 1:3:1
```

## It is exactly a pad press, with a folder

The occupancy rule is not invented here. `tally spawn` **refuses an address
the grid is drawing and allows one it is not**, which is the same predicate
`model::resolve` already applies (`src/model.rs`), run with no watchers.

That gives the behaviour you would guess from looking at the device:

| pad | `tally spawn` |
|---|---|
| dark | spawns |
| green, amber | refuses — something is alive there |
| red, dark red | refuses — something wants reading |
| dark, with exited rows behind it | spawns, and the old rows stay for `anchor logs` |

The fourth row is the interesting one and it is not a special case:
a clean exit holds its `tl.slot` in anchor but renders as nothing
([03](03-model.md#exited-rows-accumulate-and-purge-is-the-answer)), so the
pad is dark and pressing it spawns a second session. The CLI does the same,
by asking the same function. **One predicate, read by a finger and by a
shell** — if they ever disagree, the grid is lying about what a press does.

A failed or lost pad refusing is the deliberate half: red has one job, which
is *go and look*, and spawning over it would erase the only sign. Purge it
from the grid, then spawn.

**The check is advisory, and knowing why matters.** It is a list, a
decision, then a create (`src/bin/tally.rs`), with nothing holding the
address in between, and anchor puts no uniqueness constraint on label
values — so two spawns racing for the same pad both succeed, and both
sessions carry the same `tl.slot`. The grid then draws the newest and only
the newest (`model::resolve`), which makes the loser a running `claude` in
a real repo that no pad shows, no chord reaches and no amber can report:
you find out when the work never arrives. That is not fixable from this
side. It is a property of the protocol, and the design already accepts
*newest wins* everywhere else ([03](03-model.md#limits)) rather than
pretend a pad can be held. What the check buys is the case you actually hit
— you forgot that pad was taken — not the case where you and a script reach
for `1:1` in the same second. When a session goes missing, `anchor ls
--label tl.slot=1:1:1` is the only thing that shows it, and killing it by
id is the only way to end it.

## `tally fork <src> <dst>`

`spawn` answers *claim a pad for what I am doing here*. `fork` answers the
other half of the same question: **another one of those**, where *those* is
a pad you can see and a directory you are not standing in.

```sh
$ tally fork 1:1 1:2
forked 01M53KS… from 1:1:1 onto 1:1:2
```

It is `spawn` with one substitution: **the directory comes from the source
pad instead of from `$PWD`**. Everything else — the occupancy check, the
`tl.slot` label, where the view opens, the absence of `tl.spec` — is the
same code and the same rules.

| | | |
|---|---|---|
| `src` | **required** | the pad to inherit from |
| `dst` | **required** | the pad to land on. There is no pad-picking here either |
| cwd | the source's | the whole point |
| command | the source's `argv[0]`, unless `--` says otherwise | |
| args | **dropped**, unless `--` gives new ones | see below |
| login shell | the source's | you get the environment that one got |

**It inherits from the newest session the source pad has ever held, dead
rows included.** That is the same rule `>` runs on
([18](18-the-launch-column.md)) and it is what makes fork work on a pad
whose agent has already exited: anchord keeps `cwd` and `command` on every
row it has, so a pad that has *ever* held a session can be forked from.

**The arguments are deliberately dropped**, and this is the rule that
makes fork safe rather than clever. A pad running `claude --resume <id>`
describes **one conversation**. Copying its argv would put a second agent
on the same conversation, which is the one thing a new agent must not be.
Same sentence, same reason, as `model::row_launch`.

So fork is *same place, fresh agent*. It is not "fork the conversation",
and the surface has no way to be — the session id a resume would need is
the agent's to know, not ours ([23](23-gesture-brainstorm.md)).

**`src` and `dst` may be the same pad** only if that pad is not drawn —
which is the ordinary re-spawn-a-dead-pad case and is allowed by exactly
the occupancy rule above, not by a carve-out.

## `--count N`

Both `spawn` and `fork` take `--count N`, `1..=8`. It claims **N
consecutive pads, rightwards, in the address's row.**

```sh
$ cd ~/projects/futurumlab/tally
$ tally spawn 2:1 --count 4 -- claude -p "port the parser to the new API"
spawned 01M5…A on 1:2:1
spawned 01M5…B on 1:2:2
spawned 01M5…C on 1:2:3
spawned 01M5…D on 1:2:4
```

That line is the whole of *"spawn four agents and give them a goal"*, and
the two halves it is made of are worth separating:

- **the four pads** are this flag,
- **the goal** is not a feature at all. It is `argv`, after `--`, exactly
  as it has always been.

**There is no `--goal`.** A goal is text handed to a *new* process, which
is what a command line already is. There is deliberately no way to send
text to a session that is **already running**: tally attaches as an
observer and never holds the control lease ([01](01-scope.md)), so it
cannot type into a live PTY and must not learn how. *Spawn four on a goal*
is free; *redirect four onto a goal* is out of scope by construction.

### The rules

**It refuses before it spawns anything.** Every target address is checked
— in the row, inside column 8, and not drawn — and a single bad one fails
the whole command with nothing created. The alternative is a partial fleet
you then have to clean up by hand, which is worse than retyping.

**It never leaves the row.** `--count 4` from `1:7` is a usage error, not
a wrap onto row 2. Same rule as every address here: refused, not clamped
([03](03-model.md#limits)).

**`--count N` for `N > 1` opens no view**, in tmux or out of it. There are
N sessions and one of them would have to be the one you are shown, which
is a guess. It prints every id and returns; the pads are where they are
read. `--count 1` is exactly the command without the flag.

**A failure part-way through stops and reports.** The check is advisory
for the same reason `spawn`'s is — list, decide, create, with nothing
holding the addresses in between — so the honest thing on the third create
failing is to say three landed and where, not to unwind two working
agents.

## What it does not do

- **No socket to tallyd.** [08-lifecycle.md](08-lifecycle.md#one-process-no-socket)
  argued tallyd needs no control socket and that still holds — the CLI talks
  to **anchord**, writes a label, and tallyd notices within one reconcile
  (≤2 s). The grid is downstream of anchor's state, never of this process.
- **No `tl.spec`.** A spec label is a *name looked up in `slots.toml`*
  (`Config::by_spec_name`, `src/config.rs`). Writing one that no
  `[[spec]]` declares would be a label that lies, and worse, a respawn that
  found a spec would spawn in the **spec's** cwd rather than the folder you
  spawned from — the exact bug this document exists to fix.
  **This used to mean the respawn chord did nothing on a pad claimed this
  way.** There is no respawn chord now, and `tl.spec` is read only by a
  tap on a dark pad ([18](18-the-launch-column.md)).
- **No pad-picking.** It will not find you a free slot. The address is how
  you build muscle memory, and a grid that assigns addresses is one you look
  up rather than know. `--count` is not an exception: it fills *from* an
  address you named, rightwards, and refuses rather than looking elsewhere.
- **No text into a running session.** There is no `--goal`, no broadcast
  and no send. tally is an observer and never holds the control lease
  ([01](01-scope.md)); a goal reaches an agent as `argv` or not at all.
- **No other subcommands** beyond the one below. `anchor ls` already lists,
  `$EDITOR` already edits. Each subcommand here exists because nothing else
  could do it.

## `tally attention on|off`

The second subcommand, and it arrived the same way the first one did: a gap
that was measured rather than guessed.

```text
tally attention on          set    tl.attention=1 on the calling session
tally attention off         remove tl.attention  from the calling session
                            …and either way, record tl.claude=<conversation>
```

It is written for exactly one caller — a Claude Code hook — because the
derived attention signal does not fire against that agent at all
([14](14-attention.md)). `docs/04` had the hook call `anchor label` directly
and it could not: **`anchor label` needs a session id, and nothing in a
hook's environment has one.** The `$ANCHOR_SESSION` the recipe used is set
by nothing.

So the job is the lookup, not the label. It walks its own process ancestry
until it finds a pid anchord recorded, which is the only key that works when
two agents are running in one repo — the normal case on this grid.

**Not under anchor is success.** The hook fires on every turn of every
Claude Code session on this machine, and almost none of them are on the
grid; an error there would put a line in the user's agent on every turn for
a surface that is not even watching. Nothing on stdout, ever — stdout
belongs to the agent's transcript.

| | |
|---|---|
| exit `0` | written, **or** not under anchor |
| exit `1` | anchord refused, or the socket is gone — one line on stderr |
| exit `2` | usage: anything that is not exactly `on` or `off` |

It takes no address. The session is wherever the hook is, which is the same
principle `spawn` is built on: **the shell already knows**.

### It writes a second label

`tl.claude`, the Claude Code session id — the argument to
`claude --resume`, read from `CLAUDE_CODE_SESSION_ID` in the hook's own
environment, and the id that is actually worth having when a pad dies
([26](26-the-dead-pane.md)).

It rides this subcommand rather than one of its own because the expensive
half here is the lookup — a `ps -A` walk and an anchord `list` — and a
second hook would pay it again on every turn for a value that changes
approximately never. The write is skipped entirely when the label already
says what the environment says, so the ordinary turn is still a read.

Nothing changes for the caller: the hook lines are the ones
[14](14-attention.md) already asked for, and a caller that is not Claude
Code simply has no variable to report.

## Errors

Usage problems exit `2`, everything else exits `1`, and anything that
reaches anchord reports what anchord said.

| | |
|---|---|
| `9:1`, `a:b`, `1:2:3:4` | refused, **not clamped** — same rule as a label, and for the same reason ([03](03-model.md#limits)) |
| a page name no `slots.toml` declares | refused, and lists the ones that exist |
| the pad is drawn | refused, naming the session holding it and its state |
| `--count` running past column 8 | refused, **not clamped** and not wrapped |
| `--count` with any target drawn | refused before anything is created, naming the one in the way |
| `fork` from a pad nothing has ever held | refused — there is no directory to inherit |
| anchord unreachable | refused — a surface that cannot see is not a surface that spawns |
| `--` with nothing after it | refused; an empty argv is a config error, not a spawn of the empty string |

## ~~Open question — respawn on a shell-spawned pad~~ — settled

Respawn is gone. It asked `slots.toml`, there is no `slots.toml`, and a
day of use recorded zero presses because the preview could never light it
([18](18-the-launch-column.md)).

The record anchor keeps is still the answer, and it went to `>`: the right
column spawns into a **row**, inheriting that row's directory and command.
What a shell-claimed pad loses is nothing it had.
