# 01 — Scope

tally is **a control surface for anchor sessions**. The closed list:

1. **slot** — a durable binding from one pad to one thing you run
2. **spawn** — from a shell that knows the folder, or from the row that
   already does: claim an empty slot, get a session
3. **jump** — press a bound slot, end up looking at its terminal
4. **state** — the pad's colour says what the session is doing
5. **attention** — a session that needs an answer says so, loudly
6. **a few destructive gestures** — kill, forget, move — behind a deliberate chord

Six bullets. Everything not on that list belongs somewhere else.

**Bullet 2 used to read *press an unbound slot, get a session*,** and it
was reworded rather than kept, because the gesture it described was cut
(#40). The two spawn paths that exist both take their directory from
something that already knows it — a shell (`tally spawn`), or the row's
newest session (the `>` column). A config file could only predict it. The
word the bullet lost is the one the measurement took: **a bare tap on a
dark pad now does nothing**
([02](02-model.md#there-used-to-be-three-tables)).

## What it is

A client of [anchor](https://github.com/alehatsman/anchor), and nothing
else. anchor owns the PTYs; tally owns a grid of LEDs and an opinion about
what they mean.

- **slots** — 64 pads, each a durable binding to *a thing you run*, not to
  a session id ([02](02-model.md))
- **signals** — working / idle / needs-you / exited. Liveness comes from
  anchor; *needs-you* comes from hooks the agent calls, because it never
  rings the bell ([04](04-signals.md))
- **gestures** — jump, launch into a row, kill, forget, move a pad
  somewhere else, and one held button that turns the grid into a map of
  itself ([05](05-interaction.md))
- **jump** — the press that puts the real terminal in front of you
  ([06](06-views.md))

## What it is not

**Not a terminal multiplexer.** tmux does that, you already use it, and the
jump target is a tmux pane. tally does make panes — one per live slot, on
[its own tmux server](06-views.md), where a row is a session and a column is
a window — but it makes exactly the ones the grid implies and gives you no
way to arrange *them*. The layout is not a feature; it is the grid,
reflected. **Your own tmux is never touched**, and that is a property of the
socket rather than of good behaviour: a `switch-client` cannot reach a
client it cannot list.

**Not a session daemon.** It never owns a PTY, never holds a control lease,
and goes away without anything noticing. anchor exists precisely so this
layer can be disposable.

**Not a MIDI library.** It drives one device, in one mode, in one way.

**Not a Stream Deck.** It does not run arbitrary macros. Sixty-four buttons
that each do a different thing is a configuration problem nobody solves
twice; sixty-four buttons that each do *the same thing to a different agent*
is a product.

## How to reject a feature

Ask which of the six it is. If the answer needs a conjunction, it is not one
of them.

**"Let me bind arbitrary shell commands to pads."** No. That is a Stream
Deck, and the configuration surface it needs (per-pad command, args, icon,
confirmation, output handling) is larger than the rest of this program. A
slot holds a session, and what to run is whatever `tally spawn` was handed
or the row's newest session already ran.

**"Show the agent's output on the grid."** No. Eight by eight
monochrome-ish pads is not a display. The grid says *which* session wants
you; your terminal says what it wants. This was tried — see
[D3](10-decisions.md#d3--the-title-scroll).

**"Support the Launchpad X / Pro / APC / any other grid."** Not until there
is a second device in the room. The surface layer is already an internal
boundary ([03](03-surface.md)); generalising it before a second
implementation exists is guessing at the shape of the abstraction.

**"Make it work without anchor."** No. The session model *is* anchor's
model. A tally that spawned its own processes would need a PTY layer, a log,
a lease and a restart story — all of which already exist, one directory
over, with a test suite.

**"Remote control over the network."** No. The device is on the desk, by
USB. If you are not at the desk, you are on your phone, and that is
teleport. (The one exception is [the bridge](13-bridge.md), built: it
carries the same device bytes across the WSL line to a `tallybridge` that
listens on loopback only, and `tallyd` finds it by `TALLY_BRIDGE` — unset
is the local port. It is not remote control.)

**"Let it hold the control lease so I can type from the pads."** No. One
writer per PTY is anchor's invariant, and the writer is the terminal you
jumped to. tallyd attaches as an **observer**, always.

**"Answer the agent from the grid."** Mechanically possible — `tmux
send-keys` reaches the pane, which is the path a keyboard takes, so tallyd
would take no lease and anchor would need no new call. Refused anyway, and
not on the mechanism: **it is a button that answers an amber light rather
than a command.** You cannot read what you are agreeing to from a pad, and
narrowing it to "the one answer safe to give without reading" is the same
button with a smaller blast radius.

## The boundary, stated plainly

**If `tallyd` dies, nothing happens.** No session is affected; the lights go
stale and then, when the next process takes the port, correct. That is the
property worth protecting: this is the most disposable component in the
stack, and every design decision that trades disposability for cleverness is
the wrong one.

**If the Launchpad is unplugged, nothing happens.** tallyd keeps reconciling
and repaints on reconnect.

**If `anchord` is down, the grid says so rather than lying.** An empty
session list and an unreachable daemon are different facts: the logo goes
dark red and the grid holds its last truth. Sixty-four dark pads mean
"nothing is running". They must never mean "I cannot tell".

## What tally asks of anchor, and what it must not

Six of anchor's nine ops:

| op | for |
|---|---|
| `list`, unfiltered | reconstruct the grid, every 2 s |
| `create` with `tl.*` labels | spawn a slot |
| `attach` with `control:false` | derive signals ([04](04-signals.md)) |
| `terminate` | the Stop chord |
| `purge` | `tally purge`, and the Stop chord on an ended pad |
| `label` | rebind a slot, mute, clear declared attention |

Never `resize` (the view window owns the size), never `get` (the `list`
already has it), never `log` (the watcher streams).

anchor's scope doc is a closed list and this client does not get to grow it:

- **No "notify me when a session changes" op.** Poll `list`; attach for
  bytes. If a watch op is ever worth adding it is because *two* clients need
  it.
- **No agent-state field.** "This agent is waiting for you" is a policy
  about what a UI shows, and those stay in clients ([04](04-signals.md)).
- **No label-presence filter.** anchor's filter is exact-match-on-value, AND
  across keys, and `tl.slot`'s value differs per slot — so the reconcile
  lists everything and keeps the rows carrying a `tl.slot` key. A
  client-side predicate over a few dozen rows costs nothing. Inventing a
  constant `tl.owner=tally` label purely to make a query work would make
  that label mandatory, and `anchor create --label tl.slot=work:3:5 --
  claude` from a shell script would silently not light a pad
  ([02](02-model.md#where-the-binding-lives)).
- **No slot concept.** `tl.slot` is a label. anchor never interprets a
  label, and the `tl.` prefix says whose opinion it is.

### teleport is a sibling, not a dependency

teleport is **not** in the data path. It is another anchor client, and tally
talks to anchord directly. What it is instead: the reference implementation
(`anchor_watch.rs`, `anchor_client.rs`, `session/osc.rs` are the three files
this project ported), and the remote half — the desk has a grid, the phone
has teleport, same sessions, same anchord, no integration required.

**Prefix discipline is necessary and not sufficient.** teleport writes
`teleport.*`; tally writes `tl.*`. Different namespaces, no conflict — as
long as neither ever writes the other's prefix. But anchor's `label` op once
read, edited and wrote the whole map without holding a lock across the three,
so two clients could interleave and the loser lost **the entire map**, not
just its own key. Fixed in anchor `e1503c9` (#12): one guard across read,
edit, write *and persist*.

**The version that matters is the one serving the socket.** That guarantee
is a property of the running `anchord` binary, not of a pin in these docs. A
daemon installed from an older commit keeps the old behaviour until someone
rebuilds and restarts it. Check `~/.local/bin/.anchord.commit`, not the
checkout. Same distinction as everywhere else here: a fixed bug and a
deployed fix are different facts.

## provision owns the machine state

`tallyd` is a long-running process on a machine, and this machine's state is
declared in `~/dotfiles` + provision. The launchd user agent that starts
`tallyd` is a provision component, not a thing a `tally service install`
command does on its own. Any brew-installed dependency — and there should be
none — goes the same way. anchor ships `anchor service install` because it
is distributed to people who do not have provision. This is not distributed
to anybody.
