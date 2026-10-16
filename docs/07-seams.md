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
| `purge` | the purge chord |
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

Nothing holds a lock across the read and the write, so two clients can
interleave: A reads `{}`, B reads `{}`, A writes `{tl.slot}`, B writes
`{teleport.owner}`, and `tl.slot` is gone. The loser does not lose its own
key — it loses **the entire map**, which is why namespacing does not help.
The stored-session path (`server.rs:501`) holds one guard throughout and is
genuinely safe; it is the live path, the common one, that is not. Tracked
as anchor#12.

Where that touches tally: **the attention hook and tallyd are two writers
on one live session.** The hook sets `tl.attention`; tallyd removes it when
output arrives ([04](04-signals.md#4-a-hook--it-told-us)). The window is
microseconds and the damage is bounded — a pad stuck amber until the next
output, or one missed amber — so this is a thing to know, not to design
around.

Specifically: **do not build a retry or a read-back here.** Both would add
a second failure mode to cover a race whose real fix is one lock held
across an existing read and write, and that fix belongs in anchor. A client
papering over a daemon's lost update is how two codebases end up each
assuming the other is careful.

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
