# 07 — The CLI

```text
tally ls        [--json]
tally spawn     [--detach] [--count N] <addr> [-- command [args…]]
tally fork      [--detach] [--count N] <src> <dst> [-- command [args…]]
tally purge     <addr>
tally mute      <addr> [--off]
tally attention <on | off>
```

An `addr` is `row:col` or `page:row:col`, everywhere.

**Each subcommand exists because nothing else could do it.** The CLI was
deferred with *"`anchor ls` and `$EDITOR slots.toml` cover most of it; defer
until something is actually awkward"*, and every one of these arrived from a
measured gap rather than a guess.

**No socket to tallyd.** The CLI talks to **anchord**, writes a label, and
tallyd notices within one reconcile (≤ 2 s). The grid is downstream of
anchor's state, never of this process ([08](08-daemon.md#one-process-no-socket)).

## `tally spawn` — the folder is the problem

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
advance, which is a prediction about work you have not done. So the CLI is
not a convenience wrapper. It is **the other half of spawning**, for the
case where the folder comes from you rather than from a file.

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
| view | opened on tally's tmux server, unless `--detach` | a pane on *your* server can never be a view ([06](06-views.md#tally-has-its-own-tmux-server)) |

**It creates the session, opens the view over there, and returns to your
shell.** It used to hand your shell over by adopting the pane you ran it in,
which stopped being possible when views moved to tally's own server. The
folder still comes from where you are standing, which was always the half
worth having.

So **`--detach` survives with a narrower meaning**: not *do not adopt my
pane*, which is no longer a thing that happens, but **open no view at all**.
The session runs, the pad lights, and the first press is what opens it.

| | what |
|---|---|
| `tally spawn 1:1` | creates it, opens the view, prints, returns |
| `tally spawn --detach 1:1` | creates it, opens nothing, prints, returns |
| `--count N`, `N > 1` | creates N, opens nothing — one shell, N views, and picking one is a guess |

**Opening the view is not fatal.** The session exists by the time the view
is attempted, so a tmux that will not cooperate costs you a view the next
pad press makes anyway. It prints where the session went and exits `0`.

### A detached spawn can land on the agent's own first question

⚠️ **Measured, and not fixed**
([#13](https://github.com/alehatsman/tally/issues/13)). Claude Code asks
*"Is this a project you created or one you trust?"* the first time it runs
in a directory. A detached spawn into a **freshly cloned repo** meets that
prompt with nothing attached to answer it:

```sh
cd ~/projects/thing && tally spawn --detach 1:1:1 -- claude "<prompt>"
```

`tally spawn` returns `0`, the pad lights, anchor says `running` — and the
agent is sitting on a dialog, indefinitely. **From the grid it is
indistinguishable from an agent that is thinking**, which is the worst
shape a failure can have on a surface whose whole job is to tell you which
session wants you. The pad looks like work.

It is also exactly when you hit it: `--detach` is how you start several at
once (`--count N` forces it above 1), and a fresh clone is when you do
that.

The workaround, and it is a workaround:

```sh
printf '\r' | anchor attach <id>
```

`anchor attach` is documented as an interactive passthrough, not a write
channel, so this leans on behaviour anchor does not promise. It works.

**Why nothing here fixes it.** The honest fix is for the pad to *notice* —
a session that has produced output and consumed no input for N seconds is
arguably already what `tl.attention` means. That adds a **third origin** to
a bit that already carries two, *blocked* and *finished*, and what that bit
should mean is the open question in
[#23](https://github.com/alehatsman/tally/issues/23). Building on an answer
nobody has given is how the listing came to say *waiting for an answer*
about a finished turn ([04](04-signals.md)). The other half of the answer
is not in this repo at all: a real `anchor send`, or `attach` promising
what it already does.

**No `--cwd` is deliberate.** A flag would re-admit the thing we are
removing — a directory typed by hand — and the shell has a perfectly good
one already. Scripts that want a different folder can use a subshell, or
call `anchor create` directly, which stays supported.

**No `tl.spec`.** A spec label is a *name looked up in `slots.toml`*.
Writing one that no `[[spec]]` declares would be a label that lies.

### It is exactly a pad press, with a folder

`tally spawn` **refuses an address the grid is drawing and allows one it is
not**, which is the same predicate `model::resolve` applies, run with no
watchers. **One predicate, read by a finger and by a shell** — if they ever
disagree, the grid is lying about what a press does.

| pad | `tally spawn` |
|---|---|
| dark | spawns |
| green, amber | refuses — something is alive there |
| dark, with exited rows behind it | spawns, and the old rows stay for `anchor logs` |
| red, dark red | claimable where `frees_the_pad` says so ([04](04-signals.md#the-state-a-pad-renders)) — a failed pad and a daemon-restart `lost` can be reclaimed; a `kill_timeout` `lost` cannot, because the process may still be out there |

The third row is not a special case: a clean exit holds its `tl.slot` in
anchor but renders as nothing, so the pad is dark and pressing it spawns a
second session. The CLI does the same, by asking the same function.

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

### The occupancy check is advisory

It is a list, a decision, then a create, with nothing holding the address in
between, and anchor puts no uniqueness constraint on label values — so two
spawns racing for the same pad both succeed, and both sessions carry the
same `tl.slot`. The grid draws the newest and only the newest, which makes
the loser a running `claude` in a real repo that no pad shows, no chord
reaches and no amber can report: **you find out when the work never
arrives.**

That is not fixable from this side. It is a property of the protocol, and
the design already accepts *newest wins* everywhere else rather than pretend
a pad can be held. What the check buys is the case you actually hit — you
forgot that pad was taken — not the case where you and a script reach for
`1:1` in the same second. When a session goes missing, `anchor ls --label
tl.slot=1:1:1` is the only thing that shows it, and killing it by id is the
only way to end it.

## `tally fork <src> <dst>`

`spawn` answers *claim a pad for what I am doing here*. `fork` answers the
other half: **another one of those**, where *those* is a pad you can see and
a directory you are not standing in.

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
rows included.** Same rule `>` runs on, and it is what makes fork work on a
pad whose agent has already exited: anchord keeps `cwd` and `command` on
every row it has, so a pad that has *ever* held a session can be forked
from.

**The arguments are deliberately dropped**, and this is the rule that makes
fork safe rather than clever. A pad running `claude --resume <id>` describes
**one conversation**. Copying its argv would put a second agent on the same
conversation, which is the one thing a new agent must not be.

So fork is *same place, fresh agent*. It is not "fork the conversation", and
the surface has no way to be — the session id a resume would need is the
agent's to know, not ours.

**`src` and `dst` may be the same pad** only if that pad is not drawn —
which is the ordinary re-spawn-a-dead-pad case, allowed by exactly the
occupancy rule above and not by a carve-out.

## `--count N`

Both `spawn` and `fork` take `--count N`, `1..=8`. It claims **N consecutive
pads, rightwards, in the address's row.**

```sh
$ cd ~/projects/futurumlab/tally
$ tally spawn 2:1 --count 4 -- claude -p "port the parser to the new API"
spawned 01M5…A on 1:2:1
spawned 01M5…B on 1:2:2
spawned 01M5…C on 1:2:3
spawned 01M5…D on 1:2:4
```

That line is the whole of *"spawn four agents and give them a goal"*, and
the two halves are worth separating: **the four pads** are this flag, and
**the goal** is not a feature at all — it is `argv`, after `--`, exactly as
it has always been.

**There is no `--goal`.** A goal is text handed to a *new* process, which is
what a command line already is. There is deliberately no way to send text to
a session that is **already running**: tally attaches as an observer and
never holds the control lease, so it cannot type into a live PTY and must
not learn how. *Spawn four on a goal* is free; *redirect four onto a goal*
is out of scope by construction.

**It refuses before it spawns anything.** Every target address is checked —
in the row, inside column 8, and not drawn — and a single bad one fails the
whole command with nothing created. The alternative is a partial fleet you
then have to clean up by hand, which is worse than retyping.

**It never leaves the row.** `--count 4` from `1:7` is a usage error, not a
wrap onto row 2. Same rule as every address here: refused, not clamped.

**A failure part-way through stops and reports.** The check is advisory for
the same reason `spawn`'s is, so the honest thing on the third create
failing is to say three landed and where, not to unwind two working agents.

## `tally ls`

The grid as text, with the caller's own pad marked — the one subcommand that
writes nothing and takes no address. It exists because the agents this CLI
is for **cannot see the device**.

An agent running on a pad was asked to spawn two more onto adjacent pads.
The spawn was one command and worked first try. Finding out *where it was*
cost six tool calls: `anchor list` (wrong subcommand), `anchor ls`,
`anchor show` (wrong subcommand), `anchor --help`, a hand-walked process
tree, then two `anchor get`s parsed as JSON and matched against
`$CLAUDE_CODE_SESSION_ID`.

**What was missing was sight, not a primitive.** The first design was two
new verbs — `tally here` to print the caller's address, and `tally spawn
next` to claim the next free pad in its row. Both were dropped. An agent on
a pad is not short a verb; **it is blind**, and two narrower verbs answer
two phrasings of one question. `anchor ls` is not that listing either: it
lists *sessions* — ids, states, pids — and knows nothing about pages, about
`tl.slot`, or about which of them is the one asking.

`tally spawn next` would also have meant amending a decision already argued:
**no pad-picking.** `ls` does not touch it. It shows the grid; it never
assigns an address. The agent reads the picture and types `tally spawn
1:1:5` like anybody else. **Dissolving the conflict beat resolving it.**

```text
$ tally ls
       1  2  3  4  5  6  7  8
  1:1  #  #  #  @  .  .  .  .
  1:2  #  .  .  .  ?  .  .  .
  1:3  !  .  .  .  .  .  .  .

  @ you   # running   ? wants you   ! failed   x lost   . free

  1:1:1  01M57RWW07PGARAA4BD4QN9PPE  running                 /repo/dotfiles
  1:1:2  01M57RX1C8QGBSBB5CE5RO0QQF  running                 /repo/tally
  1:1:3  01M57SZ0MXAXN70VX1D0RJ486N  running                 /repo/tally-fix-move
  1:1:4  01M57T8BQK95WCTN0GC30RESJK  running                 /repo/tally-feat-ls  <- you
  1:2:5  01M57SM03QVQ7BVN99MSBYDJ45  wants you               /repo/provision
  1:3:1  01M52ZZ5R4VJ8T1H6PQW3NXK2D  failed, exit 1          /repo/anchor

  rows not shown hold nothing
```

**The session id is whole, not elided.** It is the argument `anchor kill`
and `anchor label` take. A listing a reader cannot act on costs another
command, which is the thing this subcommand is about.

### One glyph per colour

| device | `SlotState` | glyph |
|---|---|---|
| off | `Empty` | `.` |
| green, static | `Working`, `Quiet` | `#` |
| amber, pulsing | `NeedsAnswer` | `?` |
| red | `Failed { .. }` | `!` |
| dark red | `Lost { .. }` | `x` |
| — | the session asking | `@` |

**That mapping is the point, not a convenience.** The CLI already refuses to
form its own opinion about what a pad holds. A listing with its own
vocabulary would be a third opinion. One glyph per colour means the text and
the hardware cannot drift, because there is nowhere for them to drift to.

**Green is one colour, and that is load-bearing here.** The CLI cannot see
working-vs-quiet *at all*: it is derived from PTY bytes by watchers only
tallyd runs, so from this side a live session resolves to `Quiet` always and
`Working` never. So `ls` says **`running`** for both — not a simplification,
but the honest word for what a listing without watchers actually knows.

**The amber says `wants you`, and that is two states.** The listing shipped
saying `waiting for an answer`, which is false most of the times it printed:
`Stop` fires every turn and `Notification` rarely, so the common case behind
an amber is an agent sitting at a prompt with its work done. A supervisor
that read *waiting for an answer* and went to unblock it was acting on a
question nobody asked. `wants you` is true of both states, which is the most
a listing can honestly print from one bit. **It is vague on purpose and it
is not the fix** — [#23](https://github.com/alehatsman/tally/issues/23) is
whether the two should be distinguished at all. This is the cost of a
document acquiring a reader it was not written for
([04](04-signals.md#one-bit-two-states)).

**Which rows:** those holding at least one drawn session, plus the caller's,
in address order. Sixty-four rows of dots is wallpaper. The trailing line —
`rows not shown hold nothing` — makes the omission a complete statement
rather than a cap. Row labels are `page:row`, so a pad's address is the row
label and the column heading joined: no arithmetic, which was the other half
of what those six calls were doing.

**Finding the caller** uses the same walk and the same function
`tally attention` runs: the process tree upwards until a pid some **live**
session claims, nearest wins. There is no second process walk and there must
not be — that function already encodes the two rules a naive walk gets
wrong: agents nest, so the innermost owner is the answer; and a dead row
keeps its pid in anchor's history while the OS reuses it.

**Not being under anchor is not an error.** `tally ls` from an ordinary
shell prints the grid with no `@`. A caller that *is* under anchor but whose
session holds no `tl.slot` gets a line of its own rather than silence:

```text
  you are in 01M53KT… , which holds no pad
```

Naming the id is the point: it is the argument `anchor label <id>
tl.slot=1:2:3` needs. **This is the question that killed `tally here`**: a
query whose entire output is an address, whose answer is "nowhere", has to
choose between a silent exit 0 and a failure. A listing has no such problem.

### `--json`

```json
{
  "you": "1:1:4",
  "unplaced": null,
  "pads": [
    {"slot": "1:1:1", "session": "01M57RWW07PGARAA4BD4QN9PPE",
     "state": "running", "free": false,
     "cwd": "/repo/dotfiles", "claude": "c1567a67-…"},
    {"slot": "1:3:1", "session": "01M52ZZ5R4VJ8T1H6PQW3NXK2D",
     "state": "failed, exit 1", "free": true,
     "cwd": "/repo/anchor", "exit_code": 1}
  ]
}
```

Occupied pads only — a free pad is the absence of an entry, and sixty-four
objects to say "nothing" is not a format.

`you` is `null` when the caller is not on a pad. **`unplaced` is the other
half of that**: `you: null` with `unplaced: null` is *not under anchor*, and
`you: null` with an id is *under anchor, holding no pad*. Collapsing them
would make a session you can label back onto the grid indistinguishable from
a shell that was never on it.

`free` is `frees_the_pad`, **stated rather than inferred**: it is the one
thing a caller is deciding on, and making it re-derive the predicate from a
state string would be the second opinion this refuses.

### What this output promises

**Something parses this.** That is not hypothetical — it is the habit that
closed the squad spec: an agent reads the grid, finds a free pad, and calls
`tally spawn` itself. A listing with readers that are programs is an
interface, so it owes them a statement of what moves.

| | |
|---|---|
| **stable** — changing it is a break, and needs a reason in a document | `--json`'s keys and their meanings; the `you` / `unplaced` distinction; `free` as `frees_the_pad`; the exit codes; the glyph-per-colour mapping |
| **not stable** — read it, do not parse it | the text picture: column widths, spacing, the key line, row ordering, which rows are shown |
| **follows the device, not this document** | the glyph set. If the surface grows a colour, the text grows a glyph |

**The state strings are the awkward case.** They are one vocabulary shared
with `tally spawn`'s refusals on purpose, they appear in `--json` as
`state`, and they have already moved once. Treat `state` as **prose for a
human, carried in a machine format**: branch on `free`, which is a boolean
and is what a caller is actually deciding, and never on the sentence.

### What `ls` does not do

- **It does not pick pads.** The rule stands.
- **It does not show dead rows.** A dark pad can have a dozen exited rows
  behind it, and those are `tally purge`'s business.
- **It does not take an address.** `tally ls 1:1` would be a filter on eight
  lines of output.
- **It does not talk to tallyd.** `anchor ls` + `model::resolve`, same as
  every other subcommand. It renders what the device *would* show, not what
  the device is showing this millisecond.
- **It does not show working-vs-quiet or the queue.** No watchers, no
  derived signals.

## `tally purge` and `tally mute`

Both lost their buttons to the launch column and must not lose their
meaning. They went to the shell on the principle the usage data states
plainly: **the grid does the common things, the shell does the rare ones.**
341 jumps against 0 purges is not a close call.

**`purge` does not kill.** It forgets the *ended* rows behind a pad — every
session anchord reports as exited or lost that carries the pad's `tl.slot`,
rows and logs together — and it closes their views. A running session is
never in that set.

```sh
$ tally purge 1:3:1
purged 4 on 1:3:1
```

With nothing ended behind the pad it prints `nothing to purge on <addr>` and
exits `0`, so a script reading the exit code learns nothing about whether
anything is still running there. An agent that reads `tally purge <addr>` as
*kill whatever is on that pad* runs it against a live session, gets a
success exit, and the session keeps running (#11).

**Purge has to keep existing** because `anchor` has no `purge` subcommand
(anchor#14), so deleting tally's gesture without replacing it would leave
rows accumulating with no remedy anywhere — twenty-five rows after a day.

**`mute` suppresses attention and nothing else.** It sets `tl.mute` on the
session currently drawn on the pad, so the pad shows the state underneath
instead of amber, and stops counting toward the queue head and the tempo.
The session is not touched. The label is on the *session*, not the pad: a
fresh spawn there starts unmuted. A dark pad has no session to carry it, so
`mute` refuses with `nothing is on <addr>`.

```sh
$ tally mute 1:3:1
muted 1:3:1
$ tally mute 1:3:1 --off
unmuted 1:3:1
```

**It takes a flag rather than toggling**: a shell cannot see the pad, so it
says which way it meant.

**Mute has to keep existing for a reason that post-dates its button.** It is
the starvation hatch for the queue and the tempo: a prompt you are never
going to answer otherwise sits at the head of `►` forever and drives the
breathing up with it. Zero presses in a day is not an argument that it is
unneeded — it is an argument that the signal has not lied yet.

## `tally attention on|off`

```text
tally attention on          set    tl.attention=1 on the calling session
tally attention off         remove tl.attention  from the calling session
                            …and either way, record tl.claude=<conversation>
```

Written for exactly one caller — a Claude Code hook
([04](04-signals.md#4-a-hook--it-told-us)) — and it exists because
**`anchor label` needs a session id, and nothing in a hook's environment has
one.** The recipe this replaced was `anchor label $ANCHOR_SESSION
tl.attention=1`; that variable is set by nothing, appears nowhere in
anchor's source, and the line expanded to an empty argument.

So the job is the lookup, not the label.

```text
pid = getppid()
loop:
    if pid matches some live session's `pid` in `anchor list`  ->  that session
    if pid == 1 or pid == 0                                    ->  not under anchor
    pid = parent of pid
```

Verified: anchord records the agent's pid directly, and a hook is a
descendant of it. **No lookup by cwd** — two sessions in one repo is the
normal case on this grid, and cwd cannot tell them apart.

Rejected alternatives: **a lookup inside the hook** (ships today, but it is
several `anchor` calls on every turn and the only key a shell one-liner has
is cwd); **asking anchor to inject the session id** (cleanest, and the
original recipe would then be literally correct — not refused, **deferred**,
and if anchor ever grows it this subcommand becomes a one-line wrapper
rather than wasted work).

| | |
|---|---|
| exit `0` | written, **or** not under anchor |
| exit `1` | anchord refused, or the socket is gone — one line on stderr |
| exit `2` | usage: anything that is not exactly `on` or `off` |
| stdout | nothing, ever |

**Not being under anchor is success, and that is the load-bearing cell.**
The hook fires on every turn of every Claude Code session on this machine,
and almost none are on the grid; a non-zero exit there puts an error in the
user's agent on every single turn, for a surface that is not even watching.
The same argument covers anchord being down: reported, because it is a real
failure, but once, in one line, and never to stdout — **stdout belongs to
the agent's transcript.**

It takes no address. The session is wherever the hook is, which is the same
principle `spawn` is built on: **the shell already knows.**

### It writes a second label

`tl.claude`, the Claude Code session id — the argument to `claude --resume`,
read from `CLAUDE_CODE_SESSION_ID`, and the id that is actually worth having
when a pad dies ([06](06-views.md#the-dead-pane-names-its-conversation)).

It rides this subcommand rather than one of its own because the expensive
half here is the lookup, and a second hook would pay it again on every turn
for a value that changes approximately never. The write is skipped entirely
when the label already says what the environment says, so the ordinary turn
is still a read.

**It is a second write in a function named for the first one**, and that is
the one judgment call here. It is accepted because the hook's real job is
*tell tally what this agent is*, and attention is one fact of two.

## Errors

Usage problems exit `2`, everything else exits `1`, and anything that
reaches anchord reports what anchord said.

| | |
|---|---|
| `9:1`, `a:b`, `1:2:3:4` | refused, **not clamped** — same rule as a label |
| a page name no `slots.toml` declares | refused, and lists the ones that exist |
| the pad is drawn | refused, naming the session holding it and its state |
| `--count` running past column 8 | refused, **not clamped** and not wrapped |
| `--count` with any target drawn | refused before anything is created, naming the one in the way |
| `fork` from a pad nothing has ever held | refused — there is no directory to inherit |
| anchord unreachable | refused — a surface that cannot see is not a surface that spawns |
| `--` with nothing after it | refused; an empty argv is a config error, not a spawn of the empty string |
