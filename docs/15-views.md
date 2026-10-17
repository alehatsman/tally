# 15 — Where a view lives

A view is the thing a pad press puts in front of you: a tmux pane running
`anchor attach`. This document is about the unit it is addressed by, why it
outlives the agent in it, and how you get back from a jump.

**A view is addressed as a pane.** The identity is a *pane* option — never
a window or session one — and everything else here follows from that.

The container around it is [24](24-the-tally-server.md)'s: tally's own tmux
server, a row is a session, a column is a window. A view is created with
`new-window` and is the one pane in it. None of that changes the unit it is
addressed by, which is why the two documents compose rather than compete.

**Why a pane and not a window.** A window option is inherited by every pane
in the window, so a window-scoped identity is a claim about panes tally may
not have created — and the moment anything splits a view, two panes claim
one session and a jump lands on either. A pane is also the unit that can
outlive its command and the unit a return gesture can name.

## What was measured

Everything below was run against a live tmux **3.6b** server on this
machine. Where something is documented but not yet observed, it says so.

✅ **A window option is inherited by every pane in the window.** Under
`set-option -w`, splitting a view gives two panes that both claim the same
anchor session, and a jump lands on either:

```text
%99  [01M5311CGQ2DED0GKQAG23RTKN]    <- the view
%102 [01M5311CGQ2DED0GKQAG23RTKN]    <- a split of it, claiming the same session
```

Same class of bug as the missing `-w` in
[06-jump.md](06-jump.md#recovering-a-view), one level down. **This is the
measurement the pane scope exists for.**

✅ **A pane option is not inherited by siblings.** `set-option -p` on one
pane leaves the other blank — `%103 [SESS_A]`, `%104 []`. Pane scope is
the fix, not an addition beside the window scope.

✅ **`remain-on-exit` is pane-scoped in 3.6.** `set-option -p -t <pane>
remain-on-exit on` is accepted and applies to that pane alone — the sibling
reports `roe=off`. Scope matters for the same reason as the identity: a
window-scoped option is a claim about panes tally did not create.

✅ **A session whose only pane's command exits is destroyed** — which is
the detach, reproduced:

```text
command exits, no remain-on-exit   -> session DESTROYED   <-- the detach
command exits, remain-on-exit on   -> session SURVIVES, pane dead=1
```

✅ **A dead pane keeps its option and its content.** `%106 dead=1
[SESS_C]`, and `capture-pane` still returns the lines the command printed.
The corpse is readable, which is the whole point of keeping it.

✅ **`respawn-pane` reuses the pane and its id.** `%109` before, `%109`
after, `dead=1` → `dead=0`. So a view can be brought back **in place** —
in your layout, in the pane you put it in — rather than abandoned for a new
window somewhere else.

✅ **`$TMUX_PANE` is the truth inside a pane.** tmux reported `%112` for a
pane whose process read `$TMUX_PANE` and got `%112`. `tally spawn` does not
have to ask tmux which pane it is in; it is told.

⚠️ **The obvious way to find a client's pane is wrong.**
`display-message -p -c <client> '#{pane_id}'` returned a pane in an
unrelated session — `-c` chooses where a message would be *displayed*, not
what the target resolves against. The one-call list form is correct:

```text
tmux list-clients -F '#{client_name} #{pane_id} #{session_name}'
/dev/ttys000 %99 tally-dev        <- right client, right pane
```

Recorded because the wrong one looks more correct than the right one.

📖 **`switch-client -t` accepts a pane.** *"As a special case, -t may refer
to a pane (a target that contains ':', '.' or '%'), to change session,
window and pane."* — tmux(1), 3.6b. Documented, not yet observed on an
attached client; it is on the hardware checklist, because a jump that
switches the session and not the pane is a jump that lands you next to the
thing you asked for.

## Identity moves to the pane

`@tally_session` is the identity, and it lives at pane scope.

| | |
|---|---|
| set | `set-option -p -t <pane> @tally_session <id>` |
| read | `list-panes -a -F '…'`, parsed by position ([27](27-the-slot-collision.md)) |
| focus | `switch-client -c <client> -t <pane_id>` |

`switch_args` takes a target string and does not care what kind it is, so
focusing a pane costs the same call as focusing a window.

The focus line is unchanged in shape: `switch_args` (`src/jump.rs`)
takes a target string and does not care what kind it is. That is the whole
diff for the third row.

**Migration is self-healing and must be made so deliberately.** A view
window tagged the old way still matches, because the pane inherits the
window option — a one-pane view window resolves to its one pane, correctly.
So nothing breaks on the first run after the change. But the inherited tag
is exactly the ambiguity this section removes, so when a lookup resolves a
session to a pane, tally **re-tags at pane scope and clears the window
option** (`set-option -uw -t <window> @tally_session`). One extra command on the first jump per old view, and the ambiguity is gone
for good rather than lying dormant until someone splits a window.

## A view pane outlives its command

Every pane tally owns gets `remain-on-exit on`, set at pane scope at the
moment it is tagged.

The consequences, in the order you meet them:

- **Kill a session you are watching: you stay attached, looking at the
  corpse.** The pane survives `anchor attach` exiting; the window survives
  the pane; the `tally` session survives the window; your client survives
  all of it. The chain is broken at its first link, which is the only link
  tally owns.
- **An agent that exits on its own does the same thing.** This is not
  kill-specific; kill is just the reliable trigger.
- **Dark is not forgotten, literally.**
  [05-interaction.md](05-interaction.md#kill-is-the-one-that-needs-the-care)
  promises that the row and its scrollback stay. Without this the
  scrollback stayed in *anchor* and
  the pane you were looking at vanished. Now both stay.

**A dead view pane does not mean a dead agent**, and this is the trap worth
naming: detaching from `anchor attach` also ends the attach. So `pane_dead=1`
says *nothing is attached here*, which can mean the agent is gone, or that
you pressed the detach key. The pad already answers which — it is green or
it is not — and the view layer must not form a second opinion.

## Recovery: respawn in place

[06-jump.md](06-jump.md#recovering-a-view) says a pad press recreates a lost
view. With panes, the recreation has three cases instead of one:

```text
find the pane whose @tally_session is this session
├─ found, alive  -> switch-client -t <pane>
├─ found, dead   -> respawn-pane -t <pane> 'anchor attach <id>'  then switch
└─ not found     -> create a view under `tally`, tag it, switch
```

**The middle case is what keeps a pad's promise.** Without it, an agent
dying leaves a corpse and the next press opens the view somewhere else, so
the pad quietly stops meaning the place you put it. With it, press the pad
and the thing comes back **where you left it** — `respawn-pane` keeps the
pane id and therefore the tag, the column and the window.

A pane you actually *closed* has nothing to respawn into, and falls to the
third case. That is a behaviour difference rather than an accident: a dead
pane is a thing you are meant to read, and a closed one you chose to be rid
of.

The address is a lookup too, and the second one: a corpse still occupying a
column's window index will refuse a new view with `index in use`, so a dead
pane at the slot is reclaimed by `@tally_slot` before anything is created
([27](27-the-slot-collision.md)).

## The return trip

The gesture is `◄` on the top row ([16](16-the-queue.md)): its own button,
with its own affordance, so no tap on a pad carries a second meaning.

What it needs is the thing a bare `switch-client` throws away — where the
client was standing before it moved.

**Record it, one deep, per client.**

```text
before switching:  tmux list-clients -F '#{client_name} #{pane_id}'
                   for each client, if its current pane is NOT a view pane,
                   remember it as that client's origin
```

Three rules, and each of them is load-bearing:

1. **Per client.** A jump switches every attached client on purpose
   (`Jump::jump`), so "where I was" is per client and never global.
   Two terminals jump to the same place from two different origins and both
   go home correctly.
2. **One deep, and the first jump of a trip wins.** Jump to pad A, then to
   pad B, then back — you land where you started, not on A. A chained jump
   never overwrites the origin because one is already recorded. A stack
   would let you walk back through five agents, which is not a thing
   anybody has wanted while holding a grid.

   **A view is a valid origin**, and that is the half that is easy to get
   wrong. Excluding them assumes you jump *from* nvim; every pane this grid
   is used from is an anchor session, so excluding them records nothing and
   `◄` is dark forever. What the exclusion was protecting — walking back
   through agents one at a time — is held by *first jump wins* instead,
   which does not care what kind of pane you started in
   ([16](16-the-queue.md#back-and-where-you-were)).
3. **Staleness degrades to nothing.** If the origin pane is gone, the press
   does nothing and says so in the log. It must not pick a nearby pane, the
   window the pane was in, or the last session. Landing somewhere arbitrary
   is worse than not moving, because not moving is something you can
   understand from across the room.

**`switch-client -l` does not do this job.** It returns to the last
*session* — not the window, not the pane, and not the case where you were
already inside tally's server, which is every case on this grid.

**There is no focused colour**, and the return gesture needs none: you are
looking at the thing, so the pad you are on is the pad whose view fills
your terminal. The tmux lookup happens **on the press**, never on the tick,
which is what keeps [05](05-interaction.md#the-colour-language)'s objection
answered — tallyd polling tmux every tick to know where you are is the cost
that rejected a focused colour in the first place.

### Where the origin is kept

In tallyd's memory, keyed by client name. A tallyd restart loses it, and
the degradation is rule 3: the first press after a restart jumps instead of
returning.

The alternative, named so it is not reinvented later: a global user option
per client (`@tally_origin_<client>`), which survives a tallyd restart and
is inspectable with `show-options -g`, in the same spirit as `tl.*` labels
being the integration channel. It is rejected **for now** on cost — client
names are paths (`/dev/ttys000`), so they need sanitising into option names,
and the state leaks past the process that owns it. Revisit if restarts turn
out to eat returns often enough to notice.

## Who reaps the corpses

`remain-on-exit` fixes the detach by making panes accumulate. Something has
to clear them, and it should be a gesture that already exists.

**Purge closes the view pane.** Purge already means "make anchord forget
it" and already acts on every purgeable row behind a pad
(`Act::chord`), including the dark-because-killed ones
(`model::purgeable`). Extending it to close those sessions' view panes keeps
the mental model in one sentence: **purge clears the pad, so it clears the
view.**

Nothing else reaps. In particular a jump does **not** tidy up panes it
passes, because a dead pane is a thing you are meant to be able to read, and
a surface that cleans up while you are walking towards it is a surface that
loses your evidence.

The cost, stated: a `tally` session accumulates dead panes until you purge.
That is visible, bounded by how many agents you have killed, and reversible
with a gesture. The alternative — tally closing panes on a timer — trades a
visible mess for an invisible one.

## Interfaces

### tmux, complete

| what | command |
|---|---|
| find a view | `list-panes -a -F '#{pane_id} #{@tally_session} #{pane_dead}'` |
| tag a pane | `set-option -p -t <pane> @tally_session <id>` |
| hold the corpse | `set-option -p -t <pane> remain-on-exit on` |
| label the corpse | `set-option -p -t <pane> remain-on-exit-format '… #{@tally_session} …'` ([19](19-the-dead-pane.md)) |
| clear a legacy tag | `set-option -uw -t <window> @tally_session` |
| revive in place | `respawn-pane -t <pane> '<anchor> attach <id>'` |
| create a view | `new-window -d -P -F '#{pane_id}' -t tally: -n <name> '<anchor> attach <id>'` |
| create the first view | `new-session -d -s tally -P -F '#{pane_id}' -n <name> '<anchor> attach <id>'` |
| focus | `switch-client -c <client> -t <pane>` |
| read clients and their panes | `list-clients -F '#{client_name} #{pane_id}'` |
| reap | `kill-pane -t <pane>` |

The two create commands change only their `-F` format: `#{window_id}` →
`#{pane_id}`. A freshly created window has exactly one pane, so the pane
they print is the view.

### The Rust seam

[`Jump`](../src/jump.rs) is one function behind one trait and that is still
the right shape. It grows two, and the trait is what keeps tmux out of the
state layer:

```rust
pub trait Jump {
    /// Focus the view for `session`, creating or reviving it if it is gone.
    fn jump(&self, session: &str, at: &str) -> Result<(), Error>;

    /// Put each client back where it was before it jumped to `session`.
    /// `Ok(false)` when there was nowhere to go — not an error.
    fn back(&self, session: &str) -> Result<bool, Error>;

    /// Close the view pane for `session`, if there is one.
    fn close(&self, session: &str) -> Result<(), Error>;
}
```

`back` is in the trait rather than beside it because the origin table is
per-client tmux state, and the trait is the seam that owns tmux state. The
return decision — *is this client on that view right now* — is therefore
answered in one place, by the thing that already has the listing open.

`tally::act` still never opens a terminal and `tally::jump` still never
opens a socket (`src/bin/tallyd.rs`). `close` is called from `perform`
after a purge reports what it cleared, not from inside `act`.

### CLI

`tally spawn` places the view on tally's server and returns to your shell.
`--detach` declines **the view**: the session runs, the pad lights, and the
first press is what opens it ([11](11-cli.md#tally-spawn)).

## Edge cases

These are about a pane outliving its command.
[24](24-the-tally-server.md#edge-cases) carries the ones about the server
it lives on.

| situation | behaviour |
|---|---|
| view's agent killed | pane stays, dead, holding the scrollback; pad goes dark |
| pad pressed again after a kill | `respawn-pane` revives the view in place — but the session is gone, so `anchor attach` fails there and the pane shows why |
| view's window closed by you | next press creates one again; the pad still works |
| you detach from a view | pane dead, agent alive, pad still green; next press revives it in place |
| a view's window split by you | only the tagged pane matches; the split is yours |
| a corpse holding the column's index | reclaimed by address, in place ([27](27-the-slot-collision.md)) |
| `tally spawn --detach` | no view; the first press opens one |
| spawn refused (held pad, bad address, anchord down) | nothing tagged, nothing created, shell alive |
| two clients, one on the view, one not | the on-view client returns, the other jumps; both in one press |
| origin pane closed before you return | nothing moves; a log line says why |
| origin recorded, tallyd restarted | no origin; the press jumps, which is today's behaviour |
| jump A → jump B → press B | returns to the non-view origin, not to A |
| no tmux server at all | unchanged: the grid is truthful, the jump starts a server ([06](06-jump.md)) |
| purge a pad with live dead panes behind it | every purged session's view pane is closed |

## What does not change

- **`VIEW_SESSION` stays `tally`**, and is still created *around* its first
  view so no stray shell window appears (`src/jump.rs`). #2's acceptance
  criterion is explicit about this and it survives intact.
- **`@tally_session` keeps its name**, so anything of yours reading it keeps
  working — it is read from panes either way.
- **The lease lives in the pane running `anchor attach`**
  ([06](06-jump.md#who-holds-the-lease)). There is one writer per PTY and
  this layer never becomes it.
- **tallyd still has no socket** and the grid is still downstream of
  anchor's state ([08](08-lifecycle.md#one-process-no-socket)).
- **The colour language is untouched.** No new state, no new colour, no
  focused pad. The one gesture this adds reuses a press that exists.

## Validation

**Unit-testable, and therefore tested** — everything here is argument
construction and listing parsing, which is how `src/jump.rs` is already
built:

- `list-panes` output parses, and an untagged pane never matches an empty
  session id (today's `a_window_with_no_option_is_not_a_candidate`, moved
  down a level)
- a dead pane is distinguished from a live one in the same listing
- `set-option` carries `-p`, not `-w` — the mirror of the existing
  `the_tag_is_a_window_option_not_a_session_one` test, which stays as the
  record of why scope matters
- `respawn-pane` args name their pane and run the right attach
- origin is recorded only for non-view panes; a second jump does not
  overwrite it
- origin is per client, and two clients do not share one
- a missing origin returns `Ok(false)`, not an error
**Only the hardware says:**

- [ ] `switch-client -t <pane>` moves an attached client's pane, not just
      its session (the one 📖 above)
- [ ] kill a session while focused on its view: still attached, corpse
      readable, layout intact
- [ ] kill the only live session: the row session survives
- [ ] `◄` lands you where you were standing, cursor intact
- [ ] jump from two terminals at once; return on both, to different places
- [ ] a view's window split by hand: only the tagged pane matches
