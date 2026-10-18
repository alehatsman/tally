# 33 — tally's server holds its own indexes

> **Amends [24](24-the-tally-server.md)**, whose measured ✅ *"indexes do
> not renumber when a sibling is killed"* was false under the owner's
> config. Fixes [#20](https://github.com/alehatsman/tally/issues/20).

## The bug

The address of a view is a row **session** plus a window **index**. tmux
has an option, `renumber-windows`, that rewrites indexes when a window
closes. tally's server is separate from the user's, but tmux reads
`~/.tmux.conf` for **every** server it starts, so tally inherits whatever
the user set. The owner sets `renumber-windows on` (also `base-index 1`,
`pane-base-index 1`; `detach-on-destroy` is tmux's default `on`).

Result, observed live: a tab in `r01` is closed, its right-hand siblings
slide down, and a view is at the wrong index with no Move involved.
`docs/31`'s lazy relocation then meets its one unrepaired case
([#19](https://github.com/alehatsman/tally/issues/19)) and a spawn onto
the shifted pad fails with `index N in use`.

### Measured (scratch socket, tmux 3.6b, config read from `~/.tmux.conf`)

| | `renumber-windows on` | `off` |
|---|---|---|
| close window 2 of `1 2 3 4` | `1 2 3` | `1 3 4` |
| `move-window` within a row (`2 → 6`) | not renumbered | not renumbered |
| `move-window` out of a row | source not renumbered | — |
| `new-session` then `move-window` to col 3 | stays at 3 | stays at 3 |
| `set-option -g … off` on a running server | applies to **existing** sessions | — |

So it is a **close** that renumbers, never a move. Everything tally does
with `move-window` was already safe; what is not safe is a close, and the
closes are the user's hand (a tab), `tally purge`, and tally's own
placeholder `kill-window`.

## The rule

**The server tally talks to holds `renumber-windows off`, set by tally,
and tally does not rely on the user's config for any property the address
scheme needs.**

## Where it is set

`set-option -g renumber-windows off`, as `Tmux::hold_indexes`, at every
point where the server could be wrong and nothing is yet at risk:

| site | why there |
|---|---|
| after each `new-session` tally issues (row, placeholder, home) | that call may have **started** the server, which has just read the user's config |
| top of `view_pane` | covers a server restarted, or made by someone else, between presses; one exec on a path that already runs several |
| before `Tmux::close` (`tally purge`) | the close is the act that renumbers, and `purge` runs from the CLI with no daemon press before it |
| `tallyd` start, when a server is already up | a tab closed before the first press would renumber a server this daemon never touched |

Alternatives, and why not:

- **Once, at daemon start.** Fails twice: the server is routinely not up
  at login (`server_running` is false and non-fatal), and a server that
  dies with its last session comes back reading the user's config again.
- **Per row session (`-t rNN`) at creation.** Covers only sessions tally
  made, only after they exist; the placeholder and any user-made row are
  missed, and a server restart loses it with the sessions.
- **Chained into `new-session` (`\; set-option`).** Atomic, but it changes
  three argument builders to save an exec, and still needs the
  top-of-`view_pane` call for a server we did not start.

It is best-effort and idempotent: with no server there is nothing to pin
and nothing to renumber, and tally's own server-creating calls pin it
immediately after.

## `base-index`: not pinned

`new-session` puts a row's first window at the user's `base-index`, and
`make_view` already follows it with a `move-window` to the column. That
move is the whole answer, and it was measured at the two edges
(`base-index 0` and `5`, move to column 3 and 1): the window lands at the
column. The one failure is `move-window` onto the index the window already
holds (`same index`, column equal to base-index), which is the state the
move was after, and `make_view` already treats it as best-effort.
Pinning `base-index` would change where a *user's* own sessions on this
socket start for no property tally needs.

## `detach-on-destroy`: out of scope

#17 sets it off **per session**, on the row about to lose its last window
to a move, so a client in it is not detached. A server-level `off` would
make that call redundant, but it would also change what happens when the
*user* closes the last tab of the row they are standing in (today: the
terminal detaches; then: it lands in another session). That is a
behaviour change for the user, not an address invariant, and nothing in
#20 needs it. Left where it is; if it is wanted it is its own decision.

## Does this make #19 unnecessary? No.

#20 removes **one generator** of drift. #19 is what the code does when
drift exists, from any source. Still generating it with the option off:

- a tab the user drags or swaps by hand (`docs/24`: *"a dragged tab still
  jumps correctly"* — by design, so the label and the window then
  disagree). Measured: with `renumber-windows off`, `swap-window` of two
  live tabs leaves each in the other's slot; `relocation()` sees a live
  occupant for both and returns `None` for both. No order of presses
  repairs a swap, and no renumbering was involved.
- drift that **already exists** on a server this change has not yet
  reached: the option stops new drift, it does not realign old
  ([#22](https://github.com/alehatsman/tally/issues/22)).
- the order-dependence #19 describes (a refused move makes the next
  destination live) needs only one refusal, whatever caused it.

The live evidence for #22 points the same way. On 2026-10-18 pad `1:1:2`
failed twice (`index 2 in use`) because pane `%31`, tagged `1:1:3`, held
`r01:2`; it cleared only when the owner pressed `1:1:3`, the **other**
pad's address, which moved `%31` home. Someone who pressed only `1:1:2`
would have concluded that pad was permanently broken. #20 makes that
rarer; it does not make a press of the right pad find the wedge.

## What this does not do

- It does not repair a server already drifted, and does not take effect on
  the owner's running `tallyd` until it is next started. The live server
  today has `renumber-windows on`.
- The owner's `~/.tmux.conf` is untouched: that file is theirs, and tally
  holding its own invariant is the point. If they want the option off for
  their personal server too, that is a separate change in their dotfiles.

## Validation

- `cargo test`: `indexes_are_held_for_the_whole_server_and_not_one_row`
  holds the argument shape.
- **Not run:** the call sites against a real tmux. Their effect is
  measured above at the tmux level, on a scratch socket; no end-to-end
  check exists, which is [#21](https://github.com/alehatsman/tally/issues/21)'s
  gap and why the original ✅ could be wrong at all.
- A fresh server with `renumber-windows on` in the config, one press, then
  `tmux -L tally show-options -g renumber-windows` → `off`; closing a
  middle tab leaves its siblings at their indexes.
