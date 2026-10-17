# 24 — tally gets its own tmux server

Every view tally has ever made has lived on **your** tmux server, in a
session called `tally`, as a window. That was cheap and it was wrong, and
the cost is one line of shipped code:

```rust
// src/jump.rs:597
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
column is a pane.

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

✅ **Panes can be inserted in column order.** `split-window -h -d -t %0`
appends after; `split-window -hb -d -t %1` inserts before. Creating col 1,
then col 4, then col 2 gave `pane_left` 0 / 27 / 54 in the order
`1:7:1`, `1:7:2`, `1:7:4` — left-to-right by column, with gaps closed up.

✅ **`pane-border-format` is a pane option and reads a user option.**
`set-option -p -t %1 pane-border-format ' #{@tally_slot} '` took, and the
tag expands at draw time.

✅ **The whole path, driven by this code against socket `tally-check`.**
Jumping `1:4:1`, `1:4:4`, `1:4:2`, `1:7:1` in that order produced sessions
`r04` and `r07`, with `r04` holding `c1 · 1:4:1`, `c2 · 1:4:2`,
`c4 · 1:4:4` at `pane_left` 0 / 27 / 54 — **column order, not arrival
order**. A second jump to a session already in view made no second pane.
`tmux list-sessions` on the personal socket was unchanged throughout.

✅ **A view whose command dies keeps its pane, its tag and its column**,
and the next jump respawns it in place: same `%3`, same `pane_left`.

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

**Panes are created lazily and kept afterwards.** Column 4 gets a pane
the first time something runs on `x:y:4`, inserted in column order, and
`remain-on-exit` keeps it when the attach ends — which is already how a
view behaves ([15](15-views.md)). A row with two agents is a two-pane
split, not eight unreadable slivers.

⚠️ **Pane index is not the column.** `pane_index` renumbers when a pane
is killed, and a user can move panes by hand. The column lives in
`@tally_slot` and the ordering is a *placement* rule applied at creation,
not an invariant anything reads back. A row somebody rearranged still
jumps correctly; it just looks shuffled.

**`select-layout even-horizontal` on creation only.** The row re-evens
itself when a pane is added, and never again — resizing a pane by hand is
a thing you did and tally does not get to undo it.

## The home session

The Alacritty window attaches to `tally`, not to a row: a row session
might not exist yet, and `attach` to an empty server fails. The home
session is one window, running your login shell, created only if nothing
else is there.

It is a stray shell, and the earlier rule against those
(`new_session_args`, "`new-session -d -s tally` on its own leaves a stray
shell window that nothing asked for") still holds **for views**. This one
is asked for: it is the thing the terminal window shows when no agent is
in front of you, and the thing `◄` lands on when there is no origin.

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

**`tally spawn` stops adopting the pane you ran it from.** A pane on your
personal server cannot be a view on tally's server, and keeping both would
mean every jump probing two sockets and every pad carrying which one it
meant. So `spawn` creates the session, places the view on tally's server
at the pad's address, and leaves your shell exactly where it was standing.

That inverts the default settled in
[15](15-views.md#adoption-tally-spawn-from-a-pane). The section in `15`
is amended by this one, not quietly contradicted.

**`--detach` survives, and this document said it would not.** The
original claim here was that the flag becomes the only behaviour and so
retires. That is wrong, and it is worth saying why rather than quietly
deleting it: *spawn and come back* was only ever half of what `--detach`
meant. The other half — **do not put anything on the surface** — is
still a choice once a view is a pane in a row session rather than the
shell you are standing in. So the flag keeps its name and changes its
meaning:

| | before | now |
|---|---|---|
| default | the pane you ran in becomes the view | the view opens in the row session; your shell is untouched |
| `--detach` | do not hand this pane over | **open no view at all** — the session runs, the pad lights, the press is what opens it |

`--count N` above 1 implies it, because N sessions cannot be shown in
one place and the pads are where they are read.

What adoption bought was *the folder is where you are standing*, and that
is not lost: `spawn` still takes the cwd from the shell it ran in. What
is lost is *and the view is this pane*, which was only ever true inside
tmux and was never true for the 341 jumps that came from the grid.

**Views on the old server are orphaned, once.** They are not migrated and
not killed — a pad whose view tally can no longer find makes a new one on
the new server, which is the already-shipped *absent* case. The old
`tally` session on your personal socket is yours to close.

## What does not change

- the slot model, the addresses, the colours, the queue, the tempo
- `@tally_session` as the identity, pane-scoped, and every rule in
  [15](15-views.md) about why it is a pane option
- `remain-on-exit`, the dead-pane line ([19](19-the-dead-pane.md)),
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

⚠️ **Not this document's behaviour, and the first thing to trip over.**
While an art is on the surface, the grid is a **menu**: drums and the
eight launchers pick a piece, and every other press — including every
pad — is swallowed. `►` is the only way back.

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
