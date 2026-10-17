# 15 — Where a view lives

Three open issues ask three questions — [#1](https://github.com/alehatsman/tally/issues/1)
where a spawned view lands, [#2](https://github.com/alehatsman/tally/issues/2)
what happens when a view's agent dies under you, and
[#3](https://github.com/alehatsman/tally/issues/3) how you get back from a
jump. Each of them names the other two, and each of them is blocked on the
same sentence being wrong.

This document fixes the sentence.

> ⚠️ **Amended twice by [24](24-the-tally-server.md).** *Adoption is
> gone* — views live on tally's own tmux server now, and a pane on your
> personal server cannot be one. And *the container changed*: `24` was
> itself amended from pane-per-column to **window-per-column**, so a view
> is created with `new-window`, not `split-window`. What this document
> settled survives that and is the reason it works — the **identity** is
> a pane option (`-p`), never a window or session one, so the unit a view
> is addressed by is still a pane. `remain-on-exit`, respawn in place and
> origin-as-a-pane-id are unchanged.
>
> Read the headline below as *a view is addressed as a pane*, which is
> what the rest of the document argues and what shipped.

## The claim

**A view is a pane, not a window.**

[06-jump.md](06-jump.md) says a window, and said it for a good reason: when
the only thing that creates views is `tallyd`, under launchd, with no client
attached, a window in a dedicated session is the only place that is safe to
put one. That is still true of the views `tallyd` creates. It is not true of
views in general, and all three issues are consequences of treating the
special case as the definition.

Change the unit from window to pane and the three questions collapse into
one shape:

| issue | as a window question | as a pane question |
|---|---|---|
| #1 adopt the pane I ran from | a window cannot be a pane you already have | the pane you ran from **is** the unit |
| #2 the kill detaches me | the window closes, the session empties, the client falls out | panes have `remain-on-exit`; nothing above them closes |
| #3 no way back | "back" is a window, except when it is not | origin is a pane id, which is the whole address |

None of these is a patch beside the others. They are one change, read three
ways, which is why this is one document rather than three.

## What was measured

Everything below was run against a live tmux **3.6b** server on this
machine. Where something is documented but not yet observed, it says so.

✅ **A window option is inherited by every pane in the window.** With
today's code — `set-option -w` (`src/jump.rs`) — a view window's option
is visible from each of its panes, and splitting a view window gives two
panes that both claim the same anchor session:

```text
%99  [01M5311CGQ2DED0GKQAG23RTKN]    <- the view
%102 [01M5311CGQ2DED0GKQAG23RTKN]    <- a split of it, claiming the same session
```

This is the same class of bug as the missing `-w` that
[06-jump.md](06-jump.md#recovering-a-view) records, one level down. It
does not bite today only because nothing splits a view window. #1 makes
splitting normal, so it would.

✅ **A pane option is not inherited by siblings.** `set-option -p` on one
pane leaves the other blank — `%103 [SESS_A]`, `%104 []`. Pane scope is
the fix, not an addition beside the window scope.

✅ **`remain-on-exit` is pane-scoped in 3.6.** `set-option -p -t <pane>
remain-on-exit on` is accepted and applies to that pane alone — the sibling
reports `roe=off`. This matters more than it looks: under #1 the view is a
pane in **your** window, and a window-scoped option would change how your
other panes behave.

✅ **The #2 chain, reproduced and broken.** A session whose only pane's
command exits is destroyed:

```text
command exits, no remain-on-exit   -> session DESTROYED   <-- the detach
command exits, remain-on-exit on   -> session SURVIVES, pane dead=1
```

✅ **A dead pane keeps its option and its content.** `%106 dead=1
[SESS_C]`, and `capture-pane` still returns the lines the command printed.
The corpse is readable, which is what #2 asks for.

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

`@tally_session` keeps its name and its job. It changes scope.

| | today | after |
|---|---|---|
| set | `set-option -w -t <window>` | `set-option -p -t <pane>` |
| read | `list-windows -a -F '#{window_id} #{@tally_session}'` | `list-panes -a -F '#{pane_id} #{@tally_session}'` |
| focus | `switch-client -c <client> -t <window_id>` | `switch-client -c <client> -t <pane_id>` |

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
moment it is tagged. (It said *created or adopted* until
[24](24-the-tally-server.md) removed adoption; the rule is unchanged, the
second case simply no longer exists.)

The consequences, in the order you meet them:

- **Kill a session you are watching: you stay attached, looking at the
  corpse.** The pane survives `anchor attach` exiting; the window survives
  the pane; the `tally` session survives the window; your client survives
  all of it. The chain in #2 is broken at its first link, which is the only
  link tally owns.
- **An agent that exits on its own does the same thing.** #2 is right that
  this was never kill-specific; kill was just the reliable trigger.
- **Dark is not forgotten, now literally.**
  [05-interaction.md:295](05-interaction.md) already promised that the row
  and its scrollback stay. Until now the scrollback stayed in *anchor* and
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

The middle case is new and it is the one that makes adoption survivable.
Without it, killing an agent in an adopted pane leaves a corpse in *your*
window, and the next press opens the view somewhere else — so the pad
quietly stops meaning the pane you put it in. With it, the pad keeps its
promise: press it and the thing comes back **where you left it**, because
`respawn-pane` keeps the pane id and therefore the tag.

#1 worried that a closed adopted pane has no pane to recreate in, and that
the fallback to a `tally` window is a behaviour change rather than an
accident. That is still true and still the answer — but it now applies only
to a pane you actually *closed*, not to one whose agent merely died.

## Adoption: `tally spawn` from a pane

> ⛔ **Superseded by [24](24-the-tally-server.md). This section is the
> record of a decision that was reversed, not current behaviour.**
> Everything below was true and shipped; it stopped being true when
> views moved to tally's own tmux server, because a pane on your
> personal server cannot be a view on tally's. `tally spawn` now places
> the view in the row session the pad belongs to and leaves your shell
> where it is. The half this section was really defending — *the folder
> comes from where you are standing* — survives untouched.
>
> ✅ **Cleared by [16](16-the-queue.md).** Addressing stays
> `page:row:col`, declared, so there is no one-way migration to build
> this twice against.

Spawning from a shell means standing somewhere on purpose. `tally spawn`
run **inside tmux** adopts the pane it was invoked in.

```text
tally spawn 1:1
  resolve the address, read $PWD                   ) every refusal path
  list sessions, refuse an occupied pad            ) exits here, with the
  anchor create ...                                ) shell still alive
  --- the claim is accepted; only now does anything become irreversible ---
  set-option -p -t $TMUX_PANE @tally_session <id>
  set-option -p -t $TMUX_PANE remain-on-exit on
  exec anchor attach <id>
```

**Nothing execs before the claim is accepted.** That ordering is the whole
safety argument, and it is why the error paths in
[11-cli.md](11-cli.md#errors) need no changes: they all happen above the
line. A bad address, a held pad, an unreachable anchord — the shell prints
and returns, exactly as today.

| where it runs | what it does | why |
|---|---|---|
| inside tmux | adopts `$TMUX_PANE`, execs | you navigated here on purpose |
| inside tmux, `--detach` | creates a view under `tally`, prints, returns | scripts, and the old behaviour when you want it |
| outside tmux | creates a view under `tally`, prints, returns | nothing to adopt that tally can later find |
| `tallyd`, from a pad | creates a view under `tally` | no `$TMUX`, possibly no client at all ([06](06-jump.md)) |

The last two rows are today's behaviour, unchanged. Adoption is an addition
to the lookup's *inputs*, not a second lookup.

**Outside tmux falls back rather than adopting the terminal.** A bare
terminal running `anchor attach` is a perfectly good view and a completely
unfindable one: there is no pane to tag, so a pad press could never focus
it and would open a second view onto the same session. The smaller change
is also the honest one.

**An adopted pane is not renamed.** `window_name` (`src/jump.rs`) keeps
doing its job for views tally creates. The window an adopted pane lives in
is yours, it probably holds other things, and naming it `1:1:1-5ZWT` would
be tally redecorating a room it was invited into.

**No `tl.spec` still applies**, and so does its consequence — the respawn
chord does nothing on a pad claimed this way ([11-cli.md](11-cli.md#what-it-does-not-do)).
Adoption does not change what a shell-spawned session knows about itself.

## The return trip

> ✅ **Cleared by [16](16-the-queue.md), with rule 3 replaced.** The
> return gesture is `◄` on the top row, not a second press on a pad — it
> gets its own button, with an affordance, and no tap is overloaded. The
> other three rules below are unchanged and are what `◄` is built on.

A jump is one-directional today because `jump` (`src/jump.rs`) switches
the client without first reading where it was. The information exists for
one instant and is dropped.

**Record it, one deep, per client.**

```text
before switching:  tmux list-clients -F '#{client_name} #{pane_id}'
                   for each client, if its current pane is NOT a view pane,
                   remember it as that client's origin
```

Four rules, each answering one of #3's open questions:

1. **Per client.** A jump switches every attached client on purpose
   (`Jump::jump`), so "where I was" is per client and never global.
   Two terminals jump to the same place from two different origins and both
   go home correctly.
2. **One deep, and the first jump of a trip wins.** Jump to pad A, then to
   pad B, then back — you land where you started, not on A. A chained jump
   never overwrites the origin because one is already recorded. A stack
   would let you walk back through five agents, which is not a thing
   anybody has wanted while holding a grid.

   ⚠️ This rule first read *"only non-views count"*, which assumed you
   jump from nvim. Every pane this grid is used from is an anchor session,
   so nothing was ever recorded and `◄` was dark forever — caught by the
   journal an hour after shipping. See
   [16](16-the-queue.md#back-and-where-you-were).
3. **The gesture is a second press on the pad you are already on.** No new
   button, no new colour, and the device already has the concept: the top
   row's rule is *one bright thing, and it is where you are*. Per client
   falls out of it for free — a client sitting on that view returns, a
   client that is not jumps, and both are correct at the same time.
4. **Staleness degrades to nothing.** If the origin pane is gone, the press
   does nothing and says so in the log. It must not pick a nearby pane, the
   window the pane was in, or the last session. Landing somewhere arbitrary
   is worse than not moving, because not moving is something you can
   understand from across the room.

**`switch-client -l` is what this replaces.** It returns to the last
*session*, which covers exactly the case where you jumped from a different
session and nothing else: not the window, not the pane, and not the case
where you were already inside `tally`. Under #1 views stop living in a
separate session at all, at which point `-l` covers nothing.

**There is still no focused colour.**
[05-interaction.md](05-interaction.md) rejects one on two grounds — tallyd
would have to poll tmux every tick, and white would overwrite *working* on
the one pad whose working state you most want to see. Both still hold. The
return gesture needs no indicator because you are looking at the thing: the
pad you are on is the pad whose view fills your terminal. The tmux lookup
happens **on the press**, not on the tick, which is what keeps the first
objection answered.

### Where the origin is kept

In tallyd's memory, keyed by client name. A tallyd restart loses it, and
the degradation is rule 4 — the first press after a restart jumps instead
of returning, which is what it does today.

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

> ⛔ **Superseded by [24](24-the-tally-server.md).** `tally spawn` always
> returns now, and `--detach` kept its name while losing this meaning: it
> declines **the view**, not the hand-over, because there is no hand-over
> left to decline.

`tally spawn` gains `--detach` and, inside tmux, stops returning.
[11-cli.md](11-cli.md#tally-spawn) needs both, and the table there gains a
row: **pane — adopted unless `--detach`**.

## Edge cases

> ⚠️ **The rows naming an *adopted* pane are superseded by
> [24](24-the-tally-server.md)** — there are no adopted panes. Every
> other row still holds, and holds for the same reason: they are about a
> pane outliving its command, which is unchanged. [24](24-the-tally-server.md#edge-cases)
> carries the table for views on tally's own server.

| situation | behaviour |
|---|---|
| adopted pane, agent killed | pane stays, dead, holding the scrollback; pad goes dark |
| adopted pane, agent killed, pad pressed again | `respawn-pane` revives the view in place — but the session is gone, so `anchor attach` fails there and the pane shows why |
| adopted pane closed by you | next press creates a view under `tally`; the pad still works, in a new place |
| you detach from an adopted view | pane dead, agent alive, pad still green; next press revives it in place |
| view window split by you | only the tagged pane matches; the split is yours |
| legacy window-tagged view | matches on first lookup, then re-tagged to the pane and the window tag cleared |
| `tally spawn` outside tmux | view under `tally`, prints and returns, as today |
| `tally spawn --detach` inside tmux | view under `tally`, prints and returns |
| spawn refused (held pad, bad address, anchord down) | nothing tagged, nothing exec'd, shell alive |
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
- **The lease still lives in the pane running `anchor attach`**
  ([06](06-jump.md#who-holds-the-lease)). Adoption moves where the single
  writer sits; it does not add one.
- **tallyd still has no socket** and the grid is still downstream of
  anchor's state ([08](08-lifecycle.md#one-process-no-socket)).
- **The colour language is untouched.** No new state, no new colour, no
  focused pad. The one gesture this adds reuses a press that exists.

## Amendments this forces

| file | change |
|---|---|
| [06-jump.md](06-jump.md) | the unit is a pane; three-case recovery; identity at pane scope; jump becomes a round trip |
| [05-interaction.md](05-interaction.md) | what a second press on the pad you are on means; purge also closes the view |
| [11-cli.md](11-cli.md) | ⛔ reversed by [24](24-the-tally-server.md): `spawn` places a view and returns. The doc was amended to match |

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
- `tally spawn` builds its adopt sequence in the order above, with the exec
  last

**Only the hardware says** — this is the fourth time the project has needed
this list, after the three greens, the hard amber, and the title scroll:

- [ ] `switch-client -t <pane>` actually moves an attached client's pane,
      not just its session (the one 📖 above)
- [ ] kill a session while focused on its adopted pane: still attached,
      corpse readable, layout intact
- [ ] kill the only live session: the `tally` session survives
- [ ] press the pad you are on: back in nvim, cursor where you left it
- [ ] jump from two terminals at once; return on both, to different places
- [ ] spawn into a split, split again, press the pad: lands on the right
      pane of the two
- [ ] a week of this without `--detach` existing in muscle memory

The last one is the real test and it is the same shape as M6's: the
criterion that cannot be hurried.

## What is cleared to build

The pane claim is right and the three steps below are the right
decomposition. **Two of the three are not cleared**, and the reason is not
in this document — it is in
[`docs/12`](12-layout-proposal.md) and [`docs/13`](13-layout-review.md),
tracked as [#8](https://github.com/alehatsman/tally/issues/8).

| step | closes | cleared | why |
|---|---|---|---|
| 1. identity to the pane | **#2** | ✅ | nothing in any layout changes whether a pane closes when its command exits |
| 2. adoption | #1 | ✅ | [16](16-the-queue.md) keeps addressing declared and unchanged |
| 3. the return trip | #3 | ✅ | [16](16-the-queue.md) gives it `◄`, its own button |

> **All three are cleared as of [16](16-the-queue.md).** What follows is
> the record of why two of them were held, kept because the reasoning is
> the useful part — a spec validated against the code and not against the
> other specs will do this again.

**The #3 conflict is the one worth stating plainly.** The return gesture
was proposed here as *no new button, no new colour* — and that was true
and not sufficient, because the press was already spoken for. A tap on a
bound slot jumps (`05`); a tap on an amber slot enters answer mode and a
second tap jumps (`13` §5); this document adds *and a press while you are
already looking at it returns you*. Three meanings on one press, separated
by state that is not on the surface. That is the shape of the three greens,
the two bright page colours and the title scroll — each of which was
legible and unmemorable, and each of which took hardware to disprove.

It does not mean the return trip is wrong. It means the gesture has to be
chosen **after** #8 decides what the top row and the arrows are for, not
before — because #8 is where the rest of the gesture budget is being spent.

**Step 1 does not wait.** The detach is a daily bug, it is plumbing beneath
every layout in both documents, and fixing it changes nothing a user sees
except that it stops happening.

## Sequencing

Three steps, in dependency order, each shippable alone:

1. **Identity to the pane**, plus `remain-on-exit`, plus revive-in-place.
   Closes **#2** on its own, because the detach is fixed the moment the
   pane stops closing. Nothing user-visible changes except that the bug
   stops.
2. **Adoption.** Closes **#1**. Depends on step 1 for the pane to be a
   findable unit, and on revive-in-place for an adopted pane to be
   survivable.
3. **The return trip.** Closes **#3**. Depends on step 1 for the origin to
   be a pane-level coordinate at all.

#2 asked whether it should be fixed first or together with #1. First — and
not as a compromise: it is the step the other two are built on.

Steps 2 and 3 were held behind
[#8](https://github.com/alehatsman/tally/issues/8) and are released by
[16](16-the-queue.md). Step 3's gesture is `◄`, not the second pad press
proposed above.
