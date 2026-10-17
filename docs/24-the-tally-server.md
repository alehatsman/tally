# 24 — tally gets its own tmux server

Every view tally has ever made has lived on **your** tmux server, in a
session called `tally`, as a window. That was cheap and it was wrong, and
the cost is one line of shipped code:

```rust
// src/jump.rs, Jump::jump
for (client, _) in &clients {
    Tmux::run("switch-client", &switch_args(client, &pane))?;
}
```

`clients` is *every client on the server*. Press a pad while you are
reading a diff in `main` and your personal client is yanked into a view.
A jump is supposed to put an agent in front of you; it is not supposed to
take your terminal away from you to do it.

This document separates the two. **tally gets its own tmux server, its own
terminal window, and a layout that is the grid**: a row is a session, a
column is a **window** — a tab in that session.

**A column is a window and not a split**, and the reason is about reading
rather than addressing. A row of eight agents split across one terminal is
eight 23-column slivers reflowing megabytes of scrollback. A split shows
you all eight at once; the thing you do with a row is read **one** of
them.

## What was measured

Run against a live tmux **3.6b** on this machine, on socket `tallyspec`,
with the personal server untouched beside it.

✅ **A named socket is a separate server, invisible to the default one.**
`tmux -L tallyspec new-session -d -s r07` then `tmux has-session -t r07`
on the default socket: `can't find session: r07`. `tmux list-sessions`
still showed `dotfiles`, `main`, `provision` and nothing else.

✅ **Clients are per server.** `list-clients` on the new socket returned
nothing while two clients were attached to the default one. This is the
whole fix: `switch-client` cannot reach a client it cannot list.

✅ **A window index is stable and is the column.** `new-window -t r07:3`
lands at 3 whatever else the row holds, and indexes do not renumber when a
sibling is killed. Jumping `1:4:1`, `1:4:4`, `1:4:2`, `1:7:1` produced
sessions `r04` and `r07`, with `r04` holding its columns at their own
indexes regardless of arrival order. A second jump to a view already open
made no second window.

✅ **A view whose command dies keeps its window, its tag and its index**,
and the next jump respawns it in place.

✅ **`pane-border-format` is a pane option and reads a user option.**
`set-option -p -t %1 pane-border-format ' #{@tally_slot} '` took, and the
tag expands at draw time. Unused while a column is a full window, and
recorded because it is what a border would read if one is ever drawn.

⚠️ **An empty server has nothing to attach to.** `attach -t r01` on a
fresh socket says `no sessions` and exits 1. The terminal window needs
somewhere to land before any agent exists — [the home session](#the-home-session).

## The rule underneath it

**Match on the identity that survives the boundary, never on a derived
one.** Three bugs found in one evening on this codebase had that shape,
and this is the fourth.

The derived thing here was *a tmux session as a stand-in for ownership*.
A session called `tally` looked like it bounded what a jump could touch.
It bounded nothing: `list-clients` is server-scoped and was always
telling the truth. The socket is the real boundary.

The same correction appears a second time in the naming below. A page
*name* is config and can be renamed out from under sixty-four sessions,
so a row session is named by the absolute row — the identity that
survives a config edit.

## The naming spec

Everything on tally's server is named by **address**, never by content.
A name that describes what is running in a pane goes stale the moment the
agent moves on; an address is true for as long as the pad is.

| thing | name | by what rule |
|---|---|---|
| socket | `tally` | `tmux -L tally`, overridable with `TALLY_TMUX_SOCKET` |
| home session | `tally` | the one session that is not a row |
| row session | `r01` … `r64` | the **absolute** row (`Slot::absolute_row`), zero-padded so `list-sessions` sorts |
| row window | `r07` | one window per row session, `automatic-rename off` |
| pane identity | `@tally_session` = anchor id | unchanged, pane-scoped, already shipped |
| pane address | `@tally_slot` = `page:row:col` | new; what the border draws and what a human reads |
| pane title | `c3 · work:7:3 · Z8YN` | set with `select-pane -T`; the column, the address, a tail of the session id |
| terminal window | title `tally` | one Alacritty window, attached to this socket |

**Zero-padded two digits, always.** `r07` not `r7`, because the only time
you read these is in a `list-sessions` dump and an unsorted one is a list
you have to search instead of scan.

**The row session name is the absolute row, not the page name.** Page
names are config and config is editable; renaming a page must not orphan
sixty-four panes. The page name still reaches the eye, through
`@tally_slot` and the pane title, which are rewritten on every resolve.

## The layout

```text
tmux -L tally
├── tally                      the home session, one shell, where the window lands
├── r07                        row 7 of the grid
│   └── window r07
│       ├── %4  @tally_slot=work:7:1  @tally_session=01K…A3
│       └── %9  @tally_slot=work:7:4  @tally_session=01K…Z8
└── r12
    └── window r12
        └── %11 @tally_slot=ops:4:2   @tally_session=01K…Q1
```

**Windows are created lazily and kept afterwards.** Column 4 gets a
window the first time something runs on `x:y:4`, and `remain-on-exit`
keeps it when the attach ends — which is already how a view behaves
([15](15-views.md)). A row with two agents is a two-tab session; a row
with eight is eight tabs, each the full width of the terminal.

**The window index *is* the column.** `new-window -t r07:3` lands at 3
whatever else the row holds, so there is no insertion order to maintain
and no placement rule to get wrong. `@tally_slot` still carries the
address and is still what a jump reads back — the index is how a human
reads the row, not how tally finds a view.

**A dragged tab still jumps correctly.** The index is how a human reads
the row; the address is `@tally_slot` and that is what a lookup matches.
Position is never read back — which is also why nothing resizes anything,
and why there is no placement rule to get wrong.

**Window-scoped options are aimed at a pane.** `-t r07:` means *the
current window of r07*, which was harmless when a row was one window and
is a bug now. `automatic-rename off` and the window name are both set
with the view's own pane as the target.

## The home session

The Alacritty window attaches to `tally`, not to a row: a row session
might not exist yet, and `attach` to an empty server fails. The home
session is one window, running your login shell, created only if nothing
else is there.

It is a stray shell, and the rule against those (`new_session_args`:
*a session created on its own leaves a shell window nothing asked for*)
holds **for views**. This one is asked for: it is what the terminal window
shows when no agent is in front of you, and what `◄` lands on when there
is no origin.

## The terminal window

```sh
alacritty --title tally -e tmux -L tally attach -t tally
```

**One window, and tally opens it when a jump has nowhere to land.**
`jump()` lists clients on its own socket; zero clients means the view was
created correctly and nobody can see it. Rather than succeed invisibly, it
launches the window — `open -na Alacritty --args --title tally -e …` —
and the next tick's jump finds a client.

Overridable as one string, `TALLY_TERMINAL_CMD`, for the same reason
`TALLY_TERMINAL` is overridable: this is the one genuinely
machine-specific line in the file.

⚠️ **Alacritty's own config is a managed file.** Anything this needs from
`~/dotfiles/components/alacritty/alacritty.toml` — a window class, a
padding, a font size for a row of panes — is declared there and applied
with provision, never hand-edited.

## What this costs

**`tally spawn` leaves your shell where it is.** A pane on your personal
server cannot be a view on tally's, and supporting both would mean every
jump probing two sockets and every pad carrying which one it meant. So
`spawn` creates the session, places the view on tally's server at the
pad's address, prints, and returns. The folder still comes from the shell
you were standing in, which is the half worth having
([11](11-cli.md#the-folder-is-the-problem)).

**`--detach` means open no view at all.** The session runs, the pad
lights, and the first press is what opens it.

| | what |
|---|---|
| `tally spawn 1:1` | creates it, opens the view in the row session, prints, returns |
| `tally spawn --detach 1:1` | creates it, opens nothing, prints, returns |
| `--count N`, `N > 1` | implies `--detach`: N sessions cannot be shown in one place, and the pads are where they are read |

**A view tally cannot find is simply made again.** That is the ordinary
*absent* case ([15](15-views.md#recovery-respawn-in-place)), and it is why
nothing had to be migrated when views moved sockets: the pads rebuilt
themselves on first press. Any `tally` session left on a personal socket is
yours to close.

## What does not change

- the slot model, the addresses, the colours, the queue, the tempo
- `@tally_session` as the identity, pane-scoped, and every rule in
  [15](15-views.md) about why it is a pane option
- `remain-on-exit`, the dead-pane line ([26](26-the-dead-pane.md)),
  respawn-in-place
- `back`: origin is still a pane id, still per client, still first-jump-wins
  — the clients are simply tally's now
- **the row is not a project.** [16](16-the-queue.md) killed declared
  `row = project` and this does not revive it. A row session is where a
  row's views are *kept*; it assigns no meaning and the daemon still
  invents no addresses. It is the same relationship `>` already has with
  a row ([18](18-the-launch-column.md)): the row holds what you put there.

## Edge cases

| case | behaviour |
|---|---|
| tally's server is not running | a jump starts it, same as today; the grid is truthful without it |
| row session missing | created around its first view, so no stray window |
| last view in a row killed | tmux reaps the row session with its last pane — measured. The next view on that row creates it again, which is the same path a first view takes |
| `TALLY_TMUX_SOCKET` set to the default socket | supported and un-recommended; it is exactly today's behaviour, mixing included |
| a pane the user moved to another row's session | jumps correctly — identity is the tag, not the location |
| two Alacritty windows attached | both switch, both show the view; not prevented, not a goal |

## Pressing a pad while an art is playing does nothing

⚠️ **This is [21](21-the-arts.md)'s behaviour, and it is the first thing
to trip over when testing this one.** While an art is on the surface the
grid is a **menu**: `Drums` and the eight launchers pick a piece, and every
other press — including every pad — is swallowed. `►` is the only way
back.

So a pad press cannot reach a jump at all while an art is up. Press `►`
first, or you are testing the swallow and not the server. Recorded here
because two sessions have already lost time to it, and because the
failure looks exactly like a broken jump.

## Validation

1. `tmux list-sessions` on the personal socket is unchanged by any press.
2. A jump with a personal client attached to `main` leaves it on `main`.
3. Pressing `x:7:1` then `x:7:4` yields `r07` with two panes, left to right.
4. Killing `x:7:1` leaves `x:7:4` in place and findable.
5. A jump with no tally terminal open opens one, and lands in the view.
6. `tally spawn` from a personal pane returns to that shell, and the pad
   opens the view on tally's server.
7. `►` first, then the pad — see above. A swallowed press is not a
   failed jump.
