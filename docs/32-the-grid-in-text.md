# 32 — The grid in text

The surface has rendered sixty-four pads continuously since M0. It has
never rendered one as **text**, and the agents on it cannot see colour.

## The measurement

An agent running on a pad was asked to spawn two more agents onto
adjacent pads. The spawn was one command and worked first try. Finding
out *where it was* cost seven tool calls:

```text
tally --help          ok
anchor list           FAILED — the subcommand is `ls`
anchor ls             ok
anchor show <id>      FAILED — the subcommand is `get`
anchor --help         ok
ps -o ppid= …         walked its own process tree by hand
anchor get <id> ×2    parsed JSON, matched tl.claude
                      against $CLAUDE_CODE_SESSION_ID
                      -> "I am at 1:1:1"
tally spawn --detach 1:2 -- claude "…"      worked first try
```

Six calls to locate itself, one to act. **The spawn primitive is fine.**

## What was missing was sight, not a primitive

The first design for this was two new primitives — `tally here` to print
the caller's address, and `tally spawn next` to claim the next free pad
in its row. Both were dropped, and the reason is the whole argument of
this document.

An agent on a pad has no address, no map, and no way to ask for either.
It is not short a verb. **It is blind**, and the six calls above are what
blindness costs. A listing answers the question the agent actually had,
and two narrower verbs answer two phrasings of it.

**`anchor ls` is not that listing.** It lists *sessions*: ids, states,
pids. It knows nothing about pages, about `tl.slot`, or about which of
those sessions is the one asking — which is precisely the gap those six
calls fell into. [11](11-cli.md) sets the bar for a new subcommand —
*"each subcommand here exists because nothing else could do it"* — and
nothing else can do this.

### The rule that did not need amending

[11](11-cli.md#what-it-does-not-do) argues, and keeps arguing:

> **No pad-picking.** It will not find you a free slot. The address is
> how you build muscle memory, and a grid that assigns addresses is one
> you look up rather than know.

`tally spawn next` is that rule's exact prohibition, and shipping it
would have meant amending a decision that was argued once already. **`ls`
does not touch it.** It shows the grid; it never assigns an address. The
agent reads the picture and types `tally spawn 1:1:5` like anybody else,
so tally still picks no pads for anyone and the muscle-memory argument is
untouched for the human it was written about.

Dissolving the conflict beat resolving it. If *find a free pad and claim
it in one step* turns out to matter once `ls` is in use, it gets argued
then, on evidence — which is the bar this document had to clear itself.

## `tally ls`

```text
tally ls [--json]
```

No address. The grid is the whole of it, and the caller is wherever it
is.

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

**The session id is whole, not elided.** It is the argument
`anchor kill` and `anchor label` take, and the refusal `tally spawn`
prints already gives it in full. A listing a reader cannot act on costs
another command, which is the thing this document is about.

### One glyph per colour

The picture has exactly as many glyphs as the device has colours, and
they mean the same things:

| device ([`render::slot`](../src/render.rs)) | [`SlotState`](../src/signal.rs) | glyph |
|---|---|---|
| off | `Empty` | `.` |
| green, static | `Working`, `Quiet` | `#` |
| amber, pulsing | `NeedsAnswer` | `?` |
| red | `Failed { .. }` | `!` |
| dark red | `Lost { .. }` | `x` |
| — | the session asking | `@` |

**That mapping is the point, not a convenience.** The CLI already refuses
to form its own opinion about what a pad holds — `tally spawn` asks
`model::resolve`, the same function the frame is drawn from, *"so if they
ever disagree the grid is lying about what pressing it does"*
([11](11-cli.md#it-is-exactly-a-pad-press-with-a-folder)). A listing with
its own vocabulary would be a third opinion. One glyph per colour means
the text and the hardware cannot drift, because there is nowhere for them
to drift to.

`@` replaces a `#`. The session running `tally ls` is alive by
construction, so the marker hides only "green" — and the table below
carries that row's state in words regardless, including the one case
where the caller has its own attention raised.

### Green is one colour, and that is load-bearing here

`Working` and `Quiet` render identically on the device, deliberately: the
distinction *"stopped being shown, because showing it cost the only thing
motion had left to say"* (`src/render.rs`). The text inherits that, and
it has a second, harder reason to.

**The CLI cannot see the distinction at all.** Working/quiet is derived
from PTY bytes by watchers only tallyd runs; `src/bin/tally.rs` calls
`model::resolve` with an empty `derived` map, so from this side a live
session resolves to `Quiet` *always* and `Working` *never*
(`signal::slot_state`). The existing `describe` helper renders `Quiet` as
`"running, idle"` — a claim about idleness this process has no way to
make.

So `ls` says **`running`** for both. Not a simplification: the honest
word for what a listing without watchers actually knows, which is that
anchord says the session is alive.

### The amber says *wants you*, and that is two states

⚠️ This section was added after the fact. The listing shipped saying
**`waiting for an answer`**, which is false most of the times it printed.

[14](14-attention.md) raises `tl.attention` from two hooks — `Notification`
when the agent is **blocked** on a dialog, `Stop` when its **turn is
finished** — and `tally attention on` takes no argument, so *which hook
fired is lost at the write*. One bit, two states, by design: for a human
glancing at the grid both mean *go look*, which is the whole job of the
amber and does not change here.

For a reader that is a **program** it is not the same claim. `Stop` fires
every turn and `Notification` rarely, so the common case behind an amber is
an agent sitting at a prompt with its work done — and a supervisor that
read `waiting for an answer` and went to unblock it was acting on a
question nobody asked.

So the words say exactly what the label says and no more. `wants you` is
true of both states, which is the most a listing can honestly print from
one bit. **It is vague on purpose and it is not the fix** — whether the two
should be distinguished at all, and which layer would own it, is
[#23](https://github.com/alehatsman/tally/issues/23), and it stays open.
The glyph is unchanged, the colour is unchanged, and the pad still pulses
amber for both.

This is the cost of a document acquiring a reader it was not written for.
`14` was measured against a human walking past a desk and was right about
that reader; `32` added a second one and did not go back and re-read it.

### Which rows

Rows holding at least one drawn session, plus the caller's row, in
address order. Sixty-four rows of dots is wallpaper, not a listing.

The trailing line — `rows not shown hold nothing` — makes the omission a
complete statement rather than a cap. A reader that wants an empty row
can address one; every row absent from the picture is empty, and that is
said rather than implied.

Row labels are `page:row`, so a pad's address is the row label and the
column heading joined: row `1:2`, column `5`, is `1:2:5`. With named
pages it reads `work:2` and `work:2:5`. **The address you type is on the
screen** — no arithmetic, which was the other half of what the six calls
were doing.

### What "free" means

`.` is a pad nothing is drawn on, and a spawn will take it.

`!` and `x` are **also claimable** in some cases — `SlotState::frees_the_pad`
is `Empty | Failed { .. } | Lost { reclaimable: true }`, and it is the one
predicate the grid and the shell share by construction (`src/signal.rs`).
The picture does not try to encode that, because the device does not
either: reclaimable and unreclaimable `lost` are the same dark red, and
*"what changes is whether the pad can be claimed, which is a question for
the press, not for the eye"* (`src/render.rs`).

The table answers it in words instead, reusing the sentences
`tally spawn` already refuses with — `lost to an anchord restart` can be
claimed, `lost, and may still be running` cannot. One vocabulary for
occupancy, in the listing and in the refusal.

> ⚠️ [11](11-cli.md#it-is-exactly-a-pad-press-with-a-folder) still has a
> table saying red and dark red *refuse* a spawn. That is stale — a
> failed pad has been claimable since [#9](https://github.com/alehatsman/tally/issues/9),
> and [#14](https://github.com/alehatsman/tally/issues/14) tracks the
> correction. **This document takes its predicate from
> `frees_the_pad`, not from that table**, and does not fix 11 in passing:
> #14 asks for a pass over the surrounding text, because if that claim
> drifted its neighbours may have too.

### Finding the caller

The same walk `tally attention` runs, and the same function:
`attention::owning_session(&sessions, me, &ancestry)` — the process tree
upwards until a pid some **live** session claims, nearest wins, bounded
by `MAX_DEPTH` (`src/attention.rs`).

**There is no second process walk, and there must not be.** That function
already encodes the two rules a naive walk gets wrong: agents nest, so
the innermost owner is the answer; and a dead row keeps its pid in
anchor's history while the OS reuses it, so only `running` or `closing`
can match.

`attention` has had the answer since [14](14-attention.md) and has never
been able to say it — *"nothing is ever written to stdout; stdout belongs
to the agent's transcript, and this is not something the agent said."*
That rule is right and stays. It just meant the lookup had no reader.
This is the reader.

### Not being under anchor is not an error

`tally ls` run from an ordinary shell prints the grid with no `@` on it.
Nothing failed: the grid is a thing you can look at from outside, and
most shells on the machine are outside.

A caller that *is* under anchor but whose session holds no `tl.slot` —
released with `anchor label <id> tl.slot-`, or never claimed — is a real
state and gets a line of its own rather than silence:

```text
  you are in 01M53KT… , which holds no pad
```

Naming the id is the point: it is the argument `anchor label <id>
tl.slot=1:2:3` needs to put it on the grid.

This is the question that killed `tally here`. A query whose answer is
"nowhere" has to say so out loud or it reads as *"I am at the empty
address"* — which, for a verb whose entire output is an address, meant
choosing between a silent exit 0 and a failure. A listing has no such
problem. The grid prints either way, and the caller's absence from it is
one line of ordinary output.

### `--json`

For a caller that would otherwise parse a fixed-width picture.

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

`you` is `null` when the caller is not on a pad. **`unplaced` is the
other half of that**, and the two are not interchangeable: `you: null`
with `unplaced: null` is *not under anchor*, and `you: null` with
`unplaced: "01M53KT…"` is *under anchor, holding no pad* — the same two
answers the text gives with a missing `@` and the no-pad line. Collapsing
them into one null would make a session you can label back onto the grid
indistinguishable from a shell that was never on it.

`free` is `frees_the_pad`, stated rather than inferred: it is the one
thing a caller is deciding on, and making it re-derive the predicate from
a state string would be the second opinion this document keeps refusing.

## What this output promises

**Something parses this.** That is not a hypothetical — it is the habit
that closed [25](25-the-squad.md): an agent reads the grid, finds a free
pad, and calls `tally spawn` itself. A listing with readers that are
programs is an interface, so it owes them a statement of what moves.

| | |
|---|---|
| **stable** — changing it is a break, and needs a reason in a document | `--json`'s keys and their meanings; the `you` / `unplaced` distinction; `free` as `frees_the_pad`; the exit codes; the glyph-per-colour mapping |
| **not stable** — read it, do not parse it | the text picture: column widths, `STATE_WIDTH`, spacing, the key line, row ordering, which rows are shown |
| **follows the device, not this document** | the glyph set. If the surface grows a colour, the text grows a glyph, and that is the rule [above](#one-glyph-per-colour) rather than a promise made here |

**The state strings are the awkward case**, and this section exists partly
to say so. They are one vocabulary shared with `tally spawn`'s refusals on
purpose, they appear in `--json` as `state`, and they have already moved
once — `waiting for an answer` became `wants you` in exactly the kind of
silent break this table is meant to prevent. Treat `state` as **prose for a
human, carried in a machine format**: branch on `free`, which is a boolean
and is what a caller is actually deciding, and never on the sentence.

A caller that needs a state it can switch on is asking for a field this
document does not have, and should say which decision it is making — which
is also how [#23](https://github.com/alehatsman/tally/issues/23) gets
answered with evidence instead of guessing.

## What it does not do

- **It does not pick pads.** See above. The rule stands.
- **It does not show dead rows.** A dark pad can have a dozen exited rows
  behind it ([03](03-model.md#exited-rows-accumulate-and-purge-is-the-answer)),
  and those are `tally purge`'s business and already have a verb. A
  listing that showed them would be reporting history, not the grid.
- **It does not take an address.** `tally ls 1:1` would be a filter on
  eight lines of output.
- **It does not talk to tallyd.** `anchor ls` + `model::resolve`, same as
  every other subcommand ([08](08-lifecycle.md#one-process-no-socket)).
  It renders what the device *would* show, from the same state the device
  renders from — not what the device is showing this millisecond, which
  would need a socket that deliberately does not exist.
- **It does not show working-vs-quiet or the queue.** No watchers, no
  derived signals. See above.

## Errors

| | |
|---|---|
| exit `0` | the grid printed — **including** when the caller is not under anchor, or holds no pad |
| exit `1` | anchord unreachable or refusing — one line on stderr, nothing on stdout |
| exit `2` | usage: an argument, or a flag that is not `--json` |

anchord being down is the one real failure, and it is the same answer
`attention` gives for the same reason: with anchord down we cannot tell
*"not under anchor"* from *"could not look"*, so we report rather than
guess (`src/bin/tally.rs`).
