# 06 — Views and the jump

Press a live pad; end up looking at that agent. This is the gesture the
whole device exists for, and it is the one piece that is not about MIDI at
all.

## The problem, and the unit

An anchor session is a PTY in a daemon. It is not a window. Something has to
be attached to it for there to be anywhere to jump *to*, and that something
has to be findable and focusable.

**A view is a tmux pane running `anchor attach`.** Spawning a slot creates
two things — an anchor session, and a view for it.

**The unit is a pane, not a window**, and everything else here follows from
that. A window option is inherited by every pane in the window, so a
window-scoped identity is a claim about panes tally may not have created,
and the moment anything splits a view, two panes claim one session and a
jump lands on either. Measured on tmux 3.6b:

```text
%99  [01M5311CGQ2DED0GKQAG23RTKN]    <- the view
%102 [01M5311CGQ2DED0GKQAG23RTKN]    <- a split of it, claiming the same session
```

A pane option is **not** inherited by siblings — `%103 [SESS_A]`,
`%104 []`. A pane is also the unit that can outlive its command, and the
unit a return gesture can name.

The view is a *view*, and that is the whole reason this composes: closing it
does not touch the agent, because anchor owns the PTY. Kill the pane, reopen
it from the same pad, and the scrollback replays from offset 0. That is
anchor's entire pitch, used as intended.

### Who holds the lease

The pane running `anchor attach`. One writer per PTY is anchor's invariant;
the writer is where you type. tallyd attaches to every session too, but
always as an **observer** — so the grid watching an agent never competes
with the window driving it. If a view pane dies while attached, the lease is
released by the connection ending, and the next `anchor attach` takes it.
Nothing to reconcile.

## tally has its own tmux server

Every view tally made used to live on **your** tmux server, in a session
called `tally`. That was cheap and it was wrong, and the cost was one line:

```rust
for (client, _) in &clients {
    Tmux::run("switch-client", &switch_args(client, &pane))?;
}
```

`clients` is *every client on the server*. Press a pad while you are reading
a diff in `main` and your personal client is yanked into a view. A jump is
supposed to put an agent in front of you; it is not supposed to take your
terminal away from you to do it.

**The fix is the socket, not a session name.** A session called `tally`
looked like it bounded what a jump could touch. It bounded nothing:
`list-clients` is server-scoped and was always telling the truth.

Measured, tmux 3.6b: a named socket is a separate server, invisible to the
default one (`has-session` on the default socket says `can't find session`);
and **clients are per server** — `list-clients` on the new socket returned
nothing while two clients were attached to the default one. That is the
whole fix: **`switch-client` cannot reach a client it cannot list.**

### A row is a session, a column is a window

```text
tmux -L tally
├── tally                      the home session, one shell, where the window lands
├── r07                        row 7 of the grid
│   ├── window 1  %4  @tally_slot=work:7:1  @tally_session=01K…A3
│   └── window 4  %9  @tally_slot=work:7:4  @tally_session=01K…Z8
└── r12
    └── window 2  %11 @tally_slot=ops:4:2   @tally_session=01K…Q1
```

**A column is a window and not a split**, and the reason is about reading
rather than addressing. A row of eight agents split across one terminal is
eight 23-column slivers reflowing megabytes of scrollback. A split shows you
all eight at once; the thing you do with a row is read **one** of them
([D6](10-decisions.md#d6--eight-panes-in-a-row)).

**The window index *is* the column.** `new-window -t r07:3` lands at 3
whatever else the row holds, so there is no insertion order to maintain and
no placement rule to get wrong. Windows are created lazily and kept
afterwards; `remain-on-exit` holds one when its attach ends.

**A dragged tab still jumps correctly.** The index is how a human reads the
row; the address is `@tally_slot` and that is what a lookup matches. Position
is never read back — which is also why nothing resizes anything.

### Naming

Everything on tally's server is named by **address**, never by content. A
name that describes what is running in a pane goes stale the moment the
agent moves on; an address is true for as long as the pad is.

| thing | name | by what rule |
|---|---|---|
| socket | `tally` | `tmux -L tally`, overridable with `TALLY_TMUX_SOCKET` |
| home session | `tally` | the one session that is not a row |
| row session | `r01` … `r64` | the **absolute** row, zero-padded so `list-sessions` sorts |
| column window | `c3 · work:7:3 · Z8YN` | one window per column, `automatic-rename off`; named by `tag` with the same string as the pane title (a window is born with its row's name, `r07`, until it is tagged) |
| pane identity | `@tally_session` = anchor id | pane-scoped |
| pane address | `@tally_slot` = `page:row:col` | what the border draws and what a human reads |
| pane title | `c3 · work:7:3 · Z8YN` | `select-pane -T`: the column, the address, a tail of the session id |
| terminal window | title `tally` | one Alacritty window, attached to this socket |

**Zero-padded two digits, always.** `r07` not `r7`, because the only time
you read these is in a `list-sessions` dump and an unsorted one is a list
you have to search instead of scan.

**The row session name is the absolute row, not the page name.** Page names
are config and config is editable; renaming a page must not orphan
sixty-four panes. **Match on the identity that survives the boundary, never
on a derived one** — which is the same correction the socket made, in a
second place.

**Window-scoped options are aimed at a pane.** `-t r07:` means *the current
window of r07*, which was harmless when a row was one window and is a bug
now.

### The home session and the terminal window

The Alacritty window attaches to `tally`, not to a row: a row session might
not exist yet, and `attach` to an empty server fails (`no sessions`, exit 1).
The home session is one window running your login shell, created only if
nothing else is there.

It is a stray shell, and the rule against those holds **for views**. This one
is asked for: it is what the terminal window shows when no agent is in front
of you. `◄` does not go there — with no origin it is dark and does nothing
([05](05-interaction.md#back-and-where-you-were)).

```sh
open -na Alacritty --args --title tally -e tmux -L tally attach -t tally
```

**One window, and tally opens it when a jump has nowhere to land.** `jump()`
lists clients on its own socket; zero clients means the view was created
correctly and nobody can see it. Rather than succeed invisibly, it launches
the window, and the next jump finds a client. Overridable as one string,
`TALLY_TERMINAL_CMD` — the one genuinely machine-specific line in the file.
Alacritty's own config is a managed file: anything this needs from it is
declared in `~/dotfiles` and applied with provision, never hand-edited.

**Raising the window is the other half, and it has its own override.**
After the switch, `raise()` runs `open -a $TALLY_TERMINAL` so the terminal
comes out from behind a browser; `TALLY_RAISE_CMD` replaces that command
whole, and the test harness sets it to `true`. On a Windows desk both
overrides are set, to `wt.exe` and `bridge/raise.ps1`
([12](12-install.md#6-start-both-halves-at-logon)). It has to be a separate
switch: **unsetting `TALLY_TERMINAL` does not turn the raise off**, because
`terminal_app()` falls back to `"Alacritty"` when it is unset. The harness
assumed otherwise for as long as that fallback existed, and every jump in
the integration suite raised a real window on the desk running it —
diagnosed by catching `open -a Alacritty` parented to the test binary while
its owner was in a browser.

### tally pins `renumber-windows off`

The address of a view is a row **session** plus a window **index**, and tmux
has an option that rewrites indexes when a window closes. **tmux reads
`~/.tmux.conf` for every server it starts**, so a named socket is a separate
*server*, not a separate *config* — tally inherits whatever the user set.

This was written up once as a measured ✅ — *"indexes do not renumber when a
sibling is killed"* — and was false in general. It held on the machine it
was measured on because that tmux had the option at its default, `off`. The
owner sets it `on`. ([D10](10-decisions.md#d10--a-measurement-that-was-a-config))

| | `renumber-windows on` | `off` |
|---|---|---|
| close window 2 of `1 2 3 4` | `1 2 3` | `1 3 4` |
| `move-window` within a row | not renumbered | not renumbered |
| `move-window` out of a row | source not renumbered | — |
| `set-option -g … off` on a running server | applies to **existing** sessions | — |

So it is a **close** that renumbers, never a move. Everything tally does
with `move-window` was already safe; the closes are the user's hand, `tally
purge`, and tally's own placeholder `kill-window`.

**The rule: the server tally talks to holds `renumber-windows off`, set by
tally.** `set-option -g renumber-windows off`, best-effort and idempotent,
at every point where the server could be wrong and nothing is yet at risk:
after each `new-session` tally issues (that call may have *started* the
server, which has just read the user's config); at the top of `view_pane`
(covers a server restarted or made by someone else); before `Tmux::close`
(the close is the act that renumbers, and `purge` runs from the CLI with no
daemon press before it); and at `tallyd` start when a server is already up.

Doing it **once at daemon start** fails twice: the server is routinely not
up at login, and a server that dies with its last session comes back reading
the user's config again. Doing it **per row session** covers only sessions
tally made, only after they exist.

`base-index` is **not** pinned. `new-session` puts a row's first window at
the user's `base-index` and `make_view` already follows it with a
`move-window` to the column, which is the whole answer — measured at
`base-index` 0 and 5. Pinning it would change where a *user's* own sessions
on this socket start, for no property tally needs.

`detach-on-destroy` is **out of scope** at server level. It is set off *per
session*, on the row about to lose its last window to a move (below). A
server-level `off` would change what happens when the user closes the last
tab of the row they are standing in, which is a behaviour change for the
user rather than an address invariant.

## The jump

```sh
# find the pane whose @tally_session option equals the session id
tmux list-panes -a -F '#{pane_id}|#{window_id}|#{pane_dead}|#{@tally_session}|#{@tally_slot}|…'
tmux switch-client -c "<client>" -t "<pane_id>"
```

`switch-client -t` takes a pane directly — tmux(1): *"As a special case, -t
may refer to a pane … to change session, window and pane."* One command, not
three.

...plus raising the terminal emulator itself, because tmux switching a
client that is behind a browser window changes nothing you can see. On macOS
with Alacritty that is `open -a Alacritty`, which focuses the existing app
rather than starting a second one. A failed raise is never an error — tmux
has already switched the pane, so the move has happened and only the window
did not come forward.

**Both tmux commands need an explicit target, and it is not obvious.**
`tallyd` runs under launchd. It has no controlling terminal, no `$TMUX`, and
possibly **no attached client at all** — so every tmux default that resolves
to "the current one" resolves to something arbitrary, or fails.

- **`new-window` names its session.** Without `-t`, tmux picks the
  most-recently-used one, which is wherever you happened to be looking five
  minutes ago.
- **`switch-client` names its client.** **Zero clients is not an error** —
  the view exists and is correct, and the next time you attach it is there.
  More than one client is the case worth naming: switch **all** of them. A
  jump is "put this in front of me" and you do not know which terminal "me"
  is sitting at.

**The listing is parsed by position with an explicit `|` separator.** An
unset user option prints as an empty string and whitespace-splitting
swallows it, so two possibly-empty fields cannot both go last. Neither a
ULID nor a `page:row:col` address can contain a `|`. Every read of the pane
listing goes through one function, which is why there is exactly one place
that knows the format — `back()` once parsed the same listing with its own
`split_whitespace`, and changing the separator would have made every origin
look closed and `◄` silently stop working.

### What the daemon's environment does not have

`tallyd` runs under launchd with `PATH=/usr/bin:/bin:/usr/sbin:/sbin`.
Neither `tmux` nor `anchor` is on it, so **the jump works when you run
`tallyd` by hand and fails as a service** — and a tmux server that `tallyd`
starts inherits that PATH, leaving its views unable to find `anchor` either.
The agent's `PATH` is therefore set in the LaunchAgent, declared in
`~/dotfiles/components/tally`.

Worth stating because it generalises: everything this file asks of tmux is
an `exec`, and an agent's environment is not a login shell's.

### The pane starts where the agent does

A session has **two** working directories, and only one of them is anchord's.

The agent's is: `tally spawn` reads the shell's cwd, hands it to anchord, and
anchord opens the PTY there. That one was always right.

The *pane's* is tmux's, and a view's command — `anchor attach <session>` —
tells tmux nothing about it. Left alone tmux falls back to the row session's
directory, and a row session's directory is the cwd of whoever first created
it. Under launchd that is `/`. The agent is in the right repo and the pane
around it is at the root of the disk, which you see as a `/` in
`#{pane_current_path}` and get handed again by every window or split you open
from the view.

So **every command that creates or respawns a view passes `-c`** —
`new-session`, `new-window` and `respawn-pane` alike. Per *window*, not per
row: one row is eight columns and they are routinely eight different repos,
so a directory set once on the row session would be right for the first pad
on it and wrong for the rest.

Two consequences worth stating:

- **The directory has to reach `jump`, and `jump` knows only tmux.** It rides
  on `Resolved` and on the queue's `Head`, the same way the conversation id
  does and for the same reason — the view has to be *made* with something the
  grid knows and the tmux layer cannot derive.
- **Not knowing is an ordinary answer.** A pane adopted from a layout has no
  anchord row behind it; it gets no `-c` rather than a guess. A directory that
  has since been deleted is not an error either — tmux falls back to its
  default rather than refusing the command, so the worst a stale cwd costs is
  the path you had before any of this.

### tmux is assumed, and checked rather than inferred

**This design assumes tmux is running and is where you work.** If it is not,
the jump has no target and the surface degrades to a status display.

On start, tallyd runs `has-session` and says plainly in the log if there is
no server. **It says so; it does not refuse to start.** A missing tmux
server is the normal state at login, and tallyd is explicitly required to
tolerate starting before everything else exists. The grid is still truthful
with no tmux running — it is the *jump* that has no target, and jump creates
its own server when it has to. Refusing to start would trade the half of the
product that works for the half that is temporarily unavailable.

The jump mechanism is therefore **one trait**, with the tmux implementation
being the only one. That is not speculative generality — it is the seam that
keeps a hard platform dependency out of the state layer.

```rust
pub trait Jump {
    /// Focus the view for `session`, creating or reviving it if it is gone.
    /// `cwd` is where the *pane* goes, not where the agent goes.
    fn jump(
        &self,
        session: &str,
        claude: Option<&str>,
        at: &str,
        where_: Place,
        cwd: Option<&str>,
    ) -> Result<(), Error>;

    /// Which of these views are wrong -- in the wrong window, or dead
    /// under a session that is not? Reports; moves nothing.
    fn audit(&self, want: &[Want]) -> Vec<Finding>;

    /// Move a view to the window its label names, and re-label it.
    /// `Ok(())` when there is no view. Nothing is switched.
    fn relocate(&self, session: &str, at: &str, where_: Place) -> Result<(), Error>;

    /// Put each client back where it was standing before it jumped.
    /// `Ok(false)` when there was nowhere to go — not an error.
    fn back(&self) -> Result<bool, Error>;

    /// Forget every origin whose pane has closed. Runs on the reconcile tick.
    fn prune_origins(&self);

    /// Is there anywhere for `back` to go? One predicate for `◄`'s light
    /// and its press.
    fn can_go_back(&self) -> bool;

    /// Close the view pane for `session`, if there is one.
    fn close(&self, session: &str) -> Result<(), Error>;

    /// The sessions currently in front of an attached client — what the
    /// queue excludes.
    fn viewing(&self) -> Vec<String>;
}
```

`back` takes no session: the origin table is per-client tmux state, keyed by
client, and the trait is the seam that owns tmux state.
`tally::act` never opens a terminal and `tally::jump` never opens a socket.

## A view pane outlives its command

Every pane tally owns gets `remain-on-exit on`, set at pane scope at the
moment it is tagged. Measured:

```text
command exits, no remain-on-exit   -> session DESTROYED   <-- the detach
command exits, remain-on-exit on   -> session SURVIVES, pane dead=1
```

A dead pane keeps its option and its content — `capture-pane` still returns
the lines the command printed. The corpse is readable, which is the whole
point of keeping it.

The consequences, in the order you meet them:

- **Kill a session you are watching: you stay attached, looking at the
  corpse.** The pane survives `anchor attach` exiting; the window survives
  the pane; the row session survives the window; your client survives all of
  it. The chain is broken at its first link, which is the only link tally
  owns.
- **An agent that exits on its own does the same thing.** Kill is just the
  reliable trigger.
- **Dark is not forgotten, literally.** Without this the scrollback stayed
  in *anchor* and the pane you were looking at vanished. Now both stay.

**A dead view pane does not mean a dead agent**, and this is the trap worth
naming: detaching from `anchor attach` also ends the attach. So
`pane_dead=1` says *nothing is attached here*, which can mean the agent is
gone, or that you pressed the detach key. The pad already answers which, and
the view layer must not form a second opinion.

**Who reaps the corpses: purge, and nothing else.** Purge already means
"make anchord forget it" and already acts on every purgeable row behind a
pad, so extending it to close those views keeps the mental model in one
sentence: **purge clears the pad, so it clears the view.** In particular a
jump does **not** tidy up panes it passes, because a dead pane is a thing
you are meant to be able to read, and a surface that cleans up while you are
walking towards it is a surface that loses your evidence.

The cost, stated: a row accumulates dead panes until you purge. That is
visible, bounded by how many agents you have killed, and reversible with a
gesture. The alternative — closing panes on a timer — trades a visible mess
for an invisible one.

### The dead pane names its conversation

**That line is the last thing the view ever tells you.** By default it is
boilerplate — `Pane is dead (status 0, …)` — and says nothing. Here it names
the conversation that died, as a command you can paste:

```text
tally · claude --resume c1567a67-d7e0-43f1-bbd4-63832361f292 · anchor attach 01M…
```

**Which id, and why that one.** anchor's session id names a PTY that is
already gone by the time you are reading the line — worth carrying, because
it is what `anchor logs` takes and it is always present, but it resumes
nothing. **Claude Code's session id is the conversation**, and it is the
argument to `claude --resume`. So the line carries both, conversation first.

Four hops, each something tally already does:

```text
hook (inside the agent)   tally attention blocked|done|off  — env var in hand
  └─ owning_session()     which anchor session is this
      └─ anchor label     tl.claude=<uuid>        — one write, caller-owned
          └─ Tmux::tag    @tally_claude           — pane option, on jump
              └─ tmux     remain-on-exit-format   — the dead line
```

Nothing new is asked of anchor: `tl.claude` is a label, and anchor never
interprets labels.

**The id rides the attention hook** rather than one of its own. That hook
already runs on four events and already pays the expensive half — a `ps -A`
ancestry walk plus an anchord `list`. A `tally claude` on `SessionStart`
would pay that again, per turn, for a value that changes approximately
never. The env var `CLAUDE_CODE_SESSION_ID` is read rather than the stdin
JSON: both carry the id on both events, so this is a choice, and reading an
environment variable is a `std::env::var` where reading the JSON is a stdin
slurp plus a parse in a process that runs on every turn of every agent on
this machine. What it costs: the id lands on the **first tool call or first
prompt**, not at startup.

The label is written only when the value differs from the one already there,
so a `/clear` refreshes it and a quiet turn writes nothing. It is **never
removed** — a conversation outlives the PTY that ran it, which is the entire
point of wanting the id.

**Tagged on jump, not on every tick.** `tag` is reached through `view_pane`
and `place`, and the reconcile loop calls neither — its one tmux call reads.
So a view that existed before a tag format changed keeps the old line until
you jump to its pad once, and **the pad you kill is the pad you just jumped
to**. Tagging on every reconcile would mean two `set-option` calls per view
every 2 s, forever, to fix something one press fixes.

```text
set-option -p -t <pane> remain-on-exit-format \
  'tally#{?#{@tally_claude}, · claude --resume #{@tally_claude},} · anchor attach #{@tally_session} · attach exit #{pane_dead_status}'
```

One constant string, set once per pane. **No work at kill time at all** —
nothing to race, no error path on the destructive gesture, nothing new that
can fail while a session is dying.

Four things in that format are decisions:

- **The conversation goes first, because truncation eats the tail.** At 80
  columns with the anchor id leading, you get `… claude --resume
  c1567a67-d7e0` — half a UUID, which resumes nothing. Half an anchor id
  costs nothing, because it names a PTY that is gone.
- **They are commands, not labels.** `claude --resume <uuid>`, not
  `conversation <uuid>`. The thing you want after reading an id is to use
  it.
- **`attach exit` is named for what it is.** The pane ran `anchor attach`;
  the status is the attach client's, not the agent's. Calling it `exit 0`
  would be a lie on every kill, since the agent is the thing that died and
  `attach` returned cleanly. The agent's own exit is on the pad and in
  `anchor ls`.
- **No `tl.killed` distinction, and no `cwd`.** tmux cannot know whether you
  asked for the kill, and the label is tally's opinion rather than a fact
  about the pane. A `cwd` would make the line wrap.

Measured on tmux 3.6b: `remain-on-exit-format` is a pane option and takes
`-p`; user options expand in it; the conditional takes the false branch on
an **unset** option, so a view of a plain shell keeps the anchor-only line;
the line is real text in the pane, so it is selectable in copy-mode — which
is the whole point of putting the id there rather than in `display-message`;
and it does not clobber output, being drawn on the pane's last row. ⚠️ **It
must be set before the pane dies** — setting it on an already-dead pane does
not repaint, which is why the option is set in `tag`, long before any death.
A `respawn-pane` clears it, correctly: the pane is a view again, and the ids
are in the tags.

**Should `>` offer `claude --resume` rather than a fresh agent?** The label
is on the dead row, so it *could*. It deliberately does not: two pads on one
conversation is the thing a new agent must not be
([05](05-interaction.md#the-right-column-launches)). Recorded because the
label is what makes it possible, and somebody will ask.

## Finding a view: three lookups

```text
by id    → alive: switch to it
         → dead:  respawn-pane in place, then switch
by slot  → a dead pane at this address: respawn onto the new session
otherwise→ create one under its row session, tag it, switch
```

**The by-id middle case is what keeps a pad's promise.** `respawn-pane`
reuses the pane and its id — `%109` before, `%109` after, `dead=1` →
`dead=0` — so a view comes back **where you left it**, in your layout, in
the pane you put it in, keeping its tag, its column and its window. Without
it, an agent dying leaves a corpse and the next press opens the view
somewhere else, so the pad quietly stops meaning the place you put it.

A pane you actually *closed* has nothing to respawn into and falls to the
third case. That is a behaviour difference rather than an accident: a dead
pane is a thing you are meant to read, and a closed one you chose to be rid
of.

### By slot: a corpse must not hold the slot

Kill the session on a pad whose view is open, then spawn a new one on the
same pad and press it. Nothing happens. The window keeps showing the dead
session's last output.

```text
tallyd: jump 1:1:4 -> 01M54P0VWK5HT3W3H00ZG79V4D
tallyd: tmux new-window: create window failed: index 4 in use
```

Four hops: the kill ends the PTY but `remain-on-exit` keeps the pane, so the
window stays, tagged with the **old** session. The new session is a new id,
so the by-id lookup misses. Miss means create. The index is in use, and the
grid has no way to show you an error.

**The identity lookup is right and stays.** What was missing is that the
*address* is also a lookup, and tally had been writing `@tally_slot` on
every pane without ever reading it back.

**Only the dead are reclaimed.** A **live** pane at the slot tagged with a
different session is the same collision — rebind the pad while its old view
is attached — and a running `anchor attach` is something you are looking at.

**Only a corpse that is actually there.** The lookup asks two questions,
not one: does this pane claim the address, *and* is it in the window that
address names. It has to, because the reclaim is **in place** — the corpse
is respawned where it stands and then tagged with the address, and nothing
on that path moves it. Matching on the tag alone was the opposite of the
repair below: a corpse that had drifted off its slot and died still carried
the claim, so the next session on that pad inherited the corpse's *window*
along with the pad's *address*, and the two disagreed from the first press.
That is [#19](https://github.com/alehatsman/tally/issues/19) coming back in
through the lookup meant to prevent it. A corpse claiming the slot from
somewhere else is left where it is: it is not holding this index, and the
pad it really sits on reclaims it through `new-window -k` (below).

**And every pane in that window, not the first one listed.** A window holds
more than one pane the moment you split a view, and `@tally_session` is
pane-scoped, so the sibling a split leaves carries no tag. Asking only the
first pane at the index finds the sibling, misses the corpse, and then
`make_view` is right to leave an untagged pane alone — `index N in use`, and
the pad wedges. Measured: `split-window -b` is enough to put the sibling
first. *Which* pane is first is not a question this has an answer for;
whether the window holds such a corpse is.

**Why not `kill-window` then create, or `new-window -k`.** Both work and
both are shorter. They destroy the dead line, which is the one thing a
corpse is still *for*; `-k` does not ask whether the occupant is dead, and
the decision has to live somewhere that is not a flag; and an empty index
between the two calls is a window for a reconcile tick to observe a row that
is missing a column.

Measured, tmux 3.6b. The bug verbatim, and then the reclaim on the same
corpse:

```text
%1|@1|1|OLDSESSION|1:1:4
$ tmux new-window -d -t r01:4 …
create window failed: index 4 in use      (exit 1)

%1|@1|0|NEWSESSION|1:1:4
```

Same `pane_id`, same window, same index. `pane_dead` back to 0, the tag
rewritten. Nothing created and nothing destroyed.

## A moved session takes its view with it

Move a session from row 3 to row 2. Press it: the agent is still in tmux
session `r03`. Then spawn on the pad it left and press that: nothing
happens.

One cause. Move is **one label write**, and a view's physical address is a
row session plus a window index. Nothing on the Move path relocated the
pane. The by-id lookup found the pane, **re-tagged** it with the new address
and renamed its window, and returned it — never asking where the pane *is*.
The view was re-labelled, not relocated. After that the old index is held by
a pane **no address claims**, which neither of the collision lookups above
matches.

```text
tallyd: Move 1:4:1 picked up 1:4:1
tallyd: Move 1:3:1 moved 1:4:1 -> 1:3:1
tallyd: jump 1:3:1 -> 01M57RW5Y6CS04K70291262S90
tallyd: tmux new-window: create window failed: index 1 in use
```

### Where it lives: tallyd, not `act.rs`

`put_down` stays exactly what its doc comment says: **one label write**.
`act.rs` knows nothing about tmux — *jumping is tmux's business and not
anchord's* — and that is a property the tests can see.

What changes is the caller. `Done::Moved` carries the session it moved, and
`tallyd` follows the write with one call, the way it already follows a tap
with `tmux.jump`:

```text
Chord Move → put_down → one tl.slot write → Done::Moved { session, from, to }
                                            └→ tallyd: tmux.relocate(session, at)
```

**And lazily, on the by-id jump**, between "respawn if dead" and `tag`. Both
paths call one function. The eager call is what makes the vacated pad usable
*before* anyone presses the moved one; the lazy one repairs everything the
eager call cannot see — a tallyd that was down or restarted when the move
was made, a relocation that failed, or a move made before this shipped. A
happy-path jump pays nothing: the pane's current session and window index
come from the listing the lookup already takes.

### `relocate`

```text
at the address already          → nothing. tmux refuses it anyway ("same index")
destination window holds…
  nothing                       → move-window
  a dead pane                   → move-window -k
  a live pane                   → swap-window
destination session missing     → new-session -d (placeholder), move-window -k,
                                  kill the placeholder by id
always, before the move         → set-option -t <source> detach-on-destroy off
no view exists for the session  → nothing. Not an error
```

Then `tag`, which rewrites `@tally_slot`, the pane title and the window name
— the half of the bug that made the squatter invisible.

**Why `-k` is allowed here and refused for a corpse collision.** There, `-k`
would decide something a flag should not. Here the decision lives in
`relocate`, from the listing, and `-k` is only ever passed after it has been
made. And the same argument carries: the corpse is only replaced because
*you asked this slot to hold something else*. Move's legal target is a pad
with no live session, so whatever the corpse viewed is either ended or alive
at another address, where the next press makes it a fresh view. It costs the
corpse's scrollback and its `claude --resume` line; the conversation id is
still in anchord.

`-k` is **atomic**, which `kill-window` followed by `move-window` is not:
when the corpse is the only window of its row, killing it destroys the row
session and the move then fails. Measured.

**Why `detach-on-destroy off` on the source.** Moving a row's last window
out destroys the row session — tmux does it, nothing here kills anything —
and with the default a client sitting in it is **detached**. In `jump` that
client is listed *after* the lookup, so it would be gone, the list would be
empty, and tally would open a second terminal. Set on the *session*, not the
server, and only on the one about to be left.

**Why a placeholder for a missing row.** `move-window` and `break-pane` both
need the target session to exist; tmux has no verb that adopts a pane into a
session it creates. The trick of creating the session *around* the first
view cannot apply, because the view already exists and a second `anchor
attach` is a different pane with a different id. So the row is created the
plain way — a stray shell, for the length of one `move-window` — and the
stray is killed by id. The shell never gets a prompt anyone sees.

**Failure policy: best-effort, both paths.** The move was already written.
Failing it for a tmux reason would have no way to show you; refusing the
*next jump* would take away a view that is perfectly reachable by id. The
eager path logs its error and carries on. The cost is stated: a failed
relocation leaves the vacated pad wedged until the moved session is next
pressed, which repairs it.

### When the destination is live

This case shipped knowingly broken and was fixed later. `relocation()`
returned *stay*, and the caller then tagged the pane where it stood — two
writes that disagree, producing the exact state this whole subsystem exists
to prevent: **a pane whose `@tally_slot` names an address other than its
window.**

It was deferred for a good reason and a stale one. The good one still
stands: **a live `anchor attach` is something somebody may be reading**, and
a relocation is not the gesture that closes it. The stale one was *"it can
only be there if an earlier relocation was skipped or failed"* — rare and
self-inflicted. `renumber-windows on` turned that into a **generator**:
closing any window pulls every sibling right of it down one index, so a
whole row of views is at the wrong address by tmux's doing, with no Move
involved. One refused move then makes the next one's destination live.

**So the two views swap.** `swap-window -d -s <pane id> -t rNN:col`. One
call, atomic, nothing created and nothing destroyed. Measured: a pane id as
`-s` resolves to its window; it works across two sessions; `-d` means the
active window does not change, because a relocation must not move anybody.

The invariant this buys is narrow, and worth stating exactly because the
obvious wider version is false:

> **The view being relocated always ends at the address it claims.**

It is *not* true that no pad is left wedged. The swap does not reduce how
many views are out of place — it chooses **which** one is, and always in
favour of the view that was just moved. The occupant lands at the moved
view's old window, which is itself an address some pad may want, so a wedge
can travel one address sideways rather than disappear. The occupant's own
`@tally_slot` is untouched, and the lazy path repairs it on the next press
of its own pad.

That is still strictly better in two checkable ways: the relocation
**accomplishes the move**, where before it accomplished nothing; and it
stops lying about it.

**What it costs.** A client sitting in the occupant follows it — tmux swaps
the windows' positions and a client attached stays attached, so somebody
reading it sees its tab move. That is the surprise the earlier spec was
unwilling to spend without a decision, and it is spent deliberately: the
alternative is a pad that cannot be pressed at all.

**Rejected: moving the occupant first, recursively, toward its own address.**
Same end state when it terminates, and it does not always terminate — two
views each holding the other's index is a cycle, and the fix would need a
visited set, a depth bound and a decision about what to do when the cycle
closes. A swap *is* the two-element cycle, resolved in one call.

### An address is not written unless it is held

Independent of the swap, and worth doing even if the swap is one day
replaced. The lookup used to discard the relocation's result and then tag
unconditionally, so a relocation that was refused **or that simply failed**
was followed by a write claiming the address anyway.

Now: **when the relocation did not succeed, `@tally_slot` is not written**,
and the refusal is logged. The pane keeps whatever address it last
legitimately held, the drift is visible to `list-panes` and to anything that
later looks for it, and the view is still reachable, because `view_for`
matches on `@tally_session`, which is always written.

The rest of `tag` is unchanged. The session id, the conversation,
`remain-on-exit` and the dead-pane format are facts about *the pane*, not
claims about where it is, and a pane that failed to move still needs all
four.

## Drift that nothing was looking for

A view can be out of its window for reasons **no Move caused** — a
renumbering, a hand-swapped tab, a swap displacing its occupant by design —
and for a while exactly one thing repaired that: **pressing the drifted
view's own pad.** The pad it was sitting on stayed wedged until somebody
happened to press a different one.

On 2026-10-18 pad `1:1:2` failed twice with `index 2 in use` because a pane
tagged `1:1:3` held `r01:2`. It cleared only when the owner pressed `1:1:3`
— the **other** pad's address. Someone who pressed only `1:1:2` would have
concluded that pad was permanently broken.

So drift was repaired by luck and was invisible until something collided
with it. Two halves change that: **the collision repairs itself**, and **the
drift gets counted**.

### Nothing holds a window it is not entitled to

`make_view` is the call that returns `index N in use`. Two kinds of squatter
sat behind it, and the first was never recognised at all:

| what holds the index | before | now |
|---|---|---|
| a **corpse** tagged with *this* address | reclaimed and respawned | unchanged |
| a **corpse** tagged with some *other* address | `index N in use`, forever | `new-window -k` replaces it |
| a **live view** tagged elsewhere | `index N in use`, forever | parked, then the window is made |
| a pane with **no tally tag** | `index N in use` | unchanged — not ours to move |

The second row is the one that is easy to miss. The corpse lookup finds a
dead pane **by the address it claims**, so a corpse claiming a *different*
address was never matched by it — the same blind spot as the original
collision, one case wider.

**Parked, not relocated**, and the reason is a boundary worth keeping. The
obvious repair is *resolve the squatter by its tag and move it where it
belongs.* **This module cannot do that.** A tag is an address, and page
names are **config**; `slot.rs` says it outright — *addresses know about
tmux geometry, and the tmux module knows nothing about pages.* So `jump`
cannot turn a tag into a `Place` without being handed the page table, and
handing it over to fix a collision would point that dependency the wrong
way.

It does not need to. **The squatter does not have to go home; it only has to
get out of the way.** It moves to the first free window index at or above a
number **above the addressable column range**, so no pad can ever name it
and a parked view cannot wedge anything else. It is still a live view, still
reachable by id, still jumpable — and the lazy path brings it home on its
next press.

> ⚠️ **The same argument applies to the swap and is not made there.** A swap
> puts the displaced occupant at the window the moved view vacated, which is
> inside the address space and may be a pad somebody wants — which is the
> wedge travelling sideways. Parking has no such cost. Whether the swap
> should park its occupant instead of trading with it is a real question and
> is **not decided**: that shipped, this is a different call site, and
> changing a shipped decision deserves its own argument rather than a side
> effect of this one.

### The sweep counts and does not touch

Every view's address is compared with the window it is in, and the
difference is **logged**. Nothing is moved.

```text
tallyd: sweep: 01M57SZ0... claims 1:1:3, sits at r01:2, drifted
tallyd: sweep: 01M5A1B2... claims 1:2:1, sits at r02:1, dead view
tallyd: sweep: ms=1761000000000 7 views, 1 drifted, 1 dead
```

`7 views` is **what was examined**, not what was asked about. A session
can hold a pad and have no view pane — never pressed, or its pane is
gone — and the sweep steps over it rather than faulting it. Counting
those would make the two numbers after it a rate over a denominator that
was never looked at.

**Two faults, not one**, and the second was found on 2026-10-20 by the
case that shows why counting was the right first move. When anchord
dropped its connections without restarting, seven view panes died while
their sessions stayed alive — **each at the window its address names.**
`relocation()` returns `None` for a pane already at its address, dead or
alive, so all seven counted as zero drift. The instrument said the grid
was clean and seven pads cost a wasted press each.

| fault | the predicate | what it costs |
|---|---|---|
| `drifted` | `relocation()` wants to move it | the wedge this section is about |
| `dead view` | it is where it belongs, the pane is dead, **and the session is not** | a press that respawns, not a wedge |

A dead pane under a dead session is neither. That is a corpse at its own
address, which is what a finished session looks like, and the purge chord
is what clears it.

**Liveness is the daemon's opinion, not jump's.** `jump` knows about tmux
and nothing else, so it cannot ask whether an agent is running; the
daemon hands it a [`Want`] per session carrying `alive`, and `audit`
reports the ones whose pane has died under it. The same boundary the page
table lives behind.

**It still counts, on purpose.** The acting version drains drift without a
press and is the obvious thing to build; it is held for the same reason
[the level](11-unbuilt.md#the-level) is held: *measuring the architecture
as specified is worth more than measuring it already patched.* **A sweep
that repairs silently would answer the question by erasing it.** The
decision it feeds is whether the acting version is worth its blast radius
— **every `tallyd` restart rearranging panes on a live grid, at the moment
the daemon knows least.**

### A week of lines, and what the first instrument could not give

The sweep ran **once ever**, after anchord first answered, and its line
carried no time and no build. Three things followed from that, and each
is why a week of its output could not have decided anything:

- **drift that heals lazily between restarts was invisible** — one sample
  per daemon lifetime, taken at the moment the grid is youngest
- **a line could not be placed in time.** `sweep: 1 of 7` says nothing
  about when, so two lines from two restarts cannot be ordered against
  anything else in the log
- **old collisions could not be attributed to a build.** 77 `in use`
  lines over a whole log were refused as a measurement for exactly this
  reason (`ROADMAP.md`), and nothing in the log said which binary wrote
  them

So the sweep now runs on three occasions, and says when and from what:

| | |
|---|---|
| the first `list` | unchanged — not at process start, where there is nothing to audit |
| **every reconnect** | anchord coming back is the event that killed seven panes. The sweep that would have seen it runs on that edge, but not twice inside 6s (reconcile x 3) — anchord flapping produces one edge per reconcile, and the damage is done by the first drop; the floor stays under what a human can do between two restarts |
| **every 15 minutes** | the slow clock, so lazy healing between restarts is sampled rather than missed |
| `ms=` on the summary | epoch milliseconds, the journal's own unit, so a sweep line and a rising edge can be read on one timeline |
| the startup line | `tallyd: tally 0.9.0 (ab12cd3)`, and `+` on the hash when the tree was dirty. A log without it cannot say which build it is about |

Fifteen minutes is picked against the decision, not against the clock: a
week is ~670 lines, which is enough to see a rate and few enough to read.
The sweep costs one `list-panes`, which the jump path already pays per
press.

**It runs in `tallyd`, not in `jump`**, because the comparison needs the
page table. `jump` is asked a question it can answer without knowing what a
page is — *here is a list of sessions, the `Place` each one's view should
be in, and whether each is still running; which are wrong?* — and `audit`
answers it with the same pure `relocation()` the move path plans with, so
the sweep and the repair can never disagree about what *out of place*
means.

Still open on [#22](https://github.com/alehatsman/tally/issues/22): the
acting sweep. It waits on a week of these lines, and its predicate is now
known to be **liveness and address**, not address alone — the seven dead
panes would have been stepped over by the sweep as originally specified.

## Edge cases

| situation | behaviour |
|---|---|
| view's agent killed | pane stays, dead, holding the scrollback; pad goes dark |
| pad pressed again after a kill | respawn in place — but the session is gone, so `anchor attach` fails there and the pane shows why |
| view's window closed by you | next press creates one again; the pad still works |
| you detach from a view | pane dead, agent alive, pad still green; next press revives it in place |
| a view's window split by you | only the tagged pane matches; the split is yours |
| a corpse holding the column's index | reclaimed by address, in place |
| a corpse claiming the pad from another window | left alone; a fresh view is made at the pad |
| a corpse behind the sibling of a split | still reclaimed; the sibling is not what is asked about |
| spawn refused | nothing tagged, nothing created, shell alive |
| two clients, one on the view, one not | the client already on the view records no origin and stays; the other is switched and can go back; every client is attempted in one press |
| origin pane closed before you return | `◄` goes dark at the next reconcile tick; a press inside that tick reports the failed switch |
| origin recorded, tallyd restarted | no origin; `◄` is dark and the press does nothing |
| jump A → jump B → press `◄` | returns to where the trip began, not to A |
| no tmux server at all | the grid is truthful; the jump starts a server |
| purge a pad with dead panes behind it | every purged session's view pane is closed |
| tally's server not running | a jump starts it |
| row session missing | created around its first view, so no stray window |
| last view in a row killed | tmux reaps the row session with its last pane; the next view on that row creates it again |
| a pane the user moved to another row's session | jumps correctly — identity is the tag, not the location |
| moved, never pressed, tallyd up | relocated at the move; the vacated pad is free at once |
| moved, never pressed, tallyd down or relocation failed | **still wedged** until the moved session is pressed |
| moved before any view existed | nothing. Not an error |
| moved within its row | `move-window`; the source session is the destination |
| destination holds a corpse | replaced (`-k`) |
| destination holds a live pane | the two trade places |
| view is itself dead (detached) | still moved; the next press respawns it in place |
| the pad was moved onto another page | page is not a tmux concept; the row is absolute |
| `TALLY_TMUX_SOCKET` = the default socket | supported and un-recommended; it is exactly the old behaviour, mixing included |
| **an art is playing** | **every pad press is swallowed** — press `►` first. A swallowed press looks exactly like a broken jump, and two sessions have lost time to it |

## Alternatives, and why not

**A fresh Alacritty window per session.** Simpler to spawn, worse to live
with: no shared scrollback search, no copy mode, and macOS window management
is not a thing to build a product on. Also, you already live in tmux.

**teleport's web UI as the jump target.** `localhost:7337` + a session id
opens the right view from anything with a browser. Genuinely useful, and it
is the *phone* story, not the desk story — clicking a pad on the desk should
not raise a browser. A `tally jump --web <slot>` is a five-line feature once
the session id is in hand; it is not the default.

**`anchor attach` directly in a new pane in the current window.** Tempting
and wrong: the current window is where you are working. Jumping should not
rearrange the thing you jumped from.

**`switch-client -l` for the return trip.** It returns to the last
*session* — not the window, not the pane, and not the case where you were
already inside tally's server, which is every case on this grid.
