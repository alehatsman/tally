# 07 — Seams

Where this program meets the two it is built on.

## anchor

**Everything about sessions comes from anchor, through its socket.** The
full surface we use:

| op | for |
|---|---|
| `list`, unfiltered | reconstruct the grid, every 2s |
| `create` with `tl.slot` / `tl.spec` labels | spawn a slot |
| `attach` with `control:false` | derive signals ([04](04-signals.md)) |
| `terminate` | the kill chord |
| `purge` | `tally purge` ([11](11-cli.md)) |
| `label` | rebind a slot, mute, clear declared attention |

That is six of nine ops. We never `resize` (the view window owns the size),
never `get` (the `list` already has it), never `log` (the watcher streams).

**The `list` is unfiltered, and that is forced.** anchor's filter is exact
match on a value, AND across keys (`anchor/src/store.rs:96-105`); there is
no "has this key" form, and `tl.slot`'s value differs per slot. teleport can
filter because `teleport.owner` has one constant value
(`anchor_watch.rs:155`); tally cannot without inventing a second label whose
only job is to make a query work — and that label would become mandatory,
breaking `anchor create --label tl.slot=… -- claude` from a shell script,
which is a property [03-model.md](03-model.md#where-the-binding-lives) is
explicitly buying. So: list everything, keep the rows with a `tl.slot` key.
The filtering is one predicate over a few dozen rows.

**Port teleport's client, do not write a second one.**
`teleport/daemon/src/anchor_client.rs` already speaks both framings —
newline-JSON control and the 13-byte binary stream header — and has been
through the error cases. A second dialect of the same protocol is a second
place for the offset arithmetic to be subtly wrong.

### What we must not ask anchor for

anchor's scope doc is a closed list and this client does not get to grow it.
Specifically:

- **No "notify me when a session changes" op.** It does not exist, and it
  is not ours to add. Poll `list`; attach for bytes. If a watch op is ever
  worth adding it is because *two* clients need it, and teleport getting
  there first is the evidence.
- **No agent-state field.** [04-signals.md](04-signals.md) explains why it
  stays here.
- **No label-presence filter.** The exact-match filter is what anchor has,
  and wanting a second query form is not a reason to grow its protocol. A
  client-side predicate costs nothing at this scale; if it ever does cost
  something, the evidence will be a measurement, not an inconvenience.
- **No slot concept.** `tl.slot` is a label. anchor never interprets a
  label, and the `tl.` prefix says whose opinion it is — the same discipline
  as `teleport.owner` / `teleport.kind`
  (`teleport/daemon/src/anchor_bridge.rs:25-29`) and
  `teleport.claude_resume_id` (`anchor_watch.rs:51`).

### The one thing to watch

anchor is at **M5 — flipping teleport's default onto it**
(anchor `ROADMAP.md`). The protocol is implemented and tested, but it is
being actively shaped by its first real consumer. Building a second client
*now* is useful — a second consumer is how a boundary gets proven — and it
means pinning a commit rather than a release, and reading anchor's diffs.

**Pinned at `e1503c9`** — the commit carrying the label-write fix above.
Check `~/.local/bin/.anchord.commit` to see what is actually *running*,
which is a different question and the one that decides behaviour. Being the second consumer has
already paid for itself twice in one day: tally found the filter semantics
that shaped [03](03-model.md#where-the-binding-lives), and tally's use of
labels is what made #12 worth fixing rather than noting.

## teleport

teleport is **not** in the data path. It is a sibling client of anchor, and
tally talks to anchord directly.

What it is instead:

- **The reference implementation.** `anchor_watch.rs`, `anchor_client.rs`,
  `session/osc.rs` are the three files this project ports from. They are
  not dependencies; they are prior art with a test suite behind them.
- **The remote half.** The desk has a grid; the phone has teleport. Same
  sessions, same anchord, two surfaces. A session spawned from a pad shows
  up in teleport's list because they are both just anchor clients — no
  integration required, which is the dividend of putting the binding in a
  label.
- **A possible jump target, later.** `tally jump --web <slot>` printing or
  opening a teleport URL is a five-line feature once the session id is in
  hand. It is not the default ([06-jump.md](06-jump.md)).

### Where they could collide

Both will want to own a session's metadata. teleport writes
`teleport.claude_resume_id`; tally writes `tl.slot`, `tl.spec`, `tl.mute`
and `tl.attention`. Different namespaces, no conflict — **as long as neither
ever writes the other's prefix**, including "helpfully" copying one across.
Write that down in the code as a constant with the prefix in it, not as a
string literal at each call site.

**Prefix discipline is necessary and not sufficient.** An earlier draft of
this section claimed concurrent writes were safe because `label` takes
`set` and `remove` per key rather than replacing the map. That is true of
the *interface* and false of the implementation on a live session
(`anchor/src/session/manager.rs:308`):

```rust
let mut labels = session.labels.lock().clone();   // lock released here
labels.extend(set);                               // ... edit ...
session.set_labels(labels.clone());               // lock retaken, whole map overwritten
```

Nothing held a lock across the read and the write, so two clients could
interleave: A reads `{}`, B reads `{}`, A writes `{tl.slot}`, B writes
`{teleport.owner}`, and `tl.slot` is gone. The loser did not lose its own
key — it lost **the entire map**, which is why namespacing could not help.

**Fixed in anchor `e1503c9` (#12)**: one guard across read, edit, write
*and persist*. The persist being inside the guard is the part worth
knowing, because the obvious half-fix looks right and is not — serialising
the writers in memory while letting them reach the store in either order
leaves the persisted map one key short, and a restart resurrects the loss.

So the guarantee this section originally claimed is real **from
`e1503c9` onwards**: prefix-disjoint writes to the same live session keep
every key, in memory and on disk. No retry, no read-back, nothing to design
around.

### The version that matters is the one serving the socket

An earlier revision of this paragraph said "the guarantee is real" on the
strength of having re-pinned tally to `e1503c9`. That is a statement about
what tally *builds against*, and it is the wrong thing to check.

**This guarantee is a property of the running `anchord` binary, not of the
pin in these docs.** A daemon installed from an older commit keeps the old
behaviour until someone rebuilds and restarts it — and at the time of
writing, the `anchord` on this machine was built from `49e2f9d`
(`~/.local/bin/.anchord.commit`), which predates the fix. The pin moved;
the binary did not.

So: before relying on it, check the stamp, not the checkout. That is a
general rule for everything in this file — tally is a client of a daemon
with its own release cycle, and *"the source has it"* and *"the thing
answering my socket has it"* are different facts. Which is the same
distinction [01-scope.md](01-scope.md#the-boundary-stated-plainly) insists
on everywhere else: an empty list and an unreachable daemon are not the
same, and neither are a fixed bug and a deployed fix.

Where it would have touched tally: **the attention hook and tallyd are two
writers on one live session.** The hook sets `tl.attention`; tallyd removes
it when output arrives ([04](04-signals.md#4-a-hook--it-told-us)). That
being a real pair rather than a cross-client hypothetical is what moved the
fix up anchor's queue — worth remembering that reporting a seam problem
precisely is cheaper than working around it.

The real overlap is the **watcher**: if teleport and tallyd are both running,
every session has two observer attaches scanning the same bytes for the same
OSC sequences. That is wasteful but correct — anchor's fan-out is built for
many bounded subscribers and an observer never blocks the PTY reader. It is
not worth solving until it is measured, and the thing that would solve it
(one of them deriving for the other) is a coupling neither wants.

## provision

`tallyd` is a long-running process on a machine, and this machine's state is
declared in `~/dotfiles` + provision. So:

- the launchd user agent that starts `tallyd` is a provision component, not a
  thing a `tally service install` command does on its own
- any brew-installed dependency — and there should be none — goes the same
  way

anchor ships `anchor service install` because it is distributed to people
who do not have provision. This is not distributed to anybody.
See [08-lifecycle.md](08-lifecycle.md).
