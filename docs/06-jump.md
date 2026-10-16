# 06 — Jump

Press a live pad; end up looking at that agent. This is the gesture the
whole device exists for, and it is the one piece that is not about MIDI at
all.

## The problem

An anchor session is a PTY in a daemon. It is not a window. Something has
to be attached to it for there to be anywhere to jump *to*, and that
something has to be findable and focusable.

## The answer: a tmux window per session

You already have the shape of this on the keyboard —
`~/.local/bin/tmux-claude-jump`, bound to `prefix C-j`, fuzzy-picks a pane
running `claude` and does:

```sh
tmux switch-client -t "$window"
tmux select-pane -t "$target"
```

The Launchpad is that script with the fuzzy-finder replaced by a pad.

So: **spawning a slot creates two things** — an anchor session, and a tmux
window running `anchor attach <id>` in it.

```text
tap pad ──▶ anchor create --label tl.slot=work:3:5 -- claude
       └──▶ tmux new-window -d -n "w3.5" "anchor attach <id>"
            tmux set-option -t <win> @tally_session <id>
```

The tmux window is a *view*, and that is the whole reason this composes:
closing it does not touch the agent, because anchor owns the PTY. Kill the
window, reopen it from the same pad, and the scrollback replays from offset
0. That is anchor's entire pitch, used as intended.

Jump is then:

```sh
tmux switch-client -t "<window>"    # find by @tally_session == id
```

...plus raising the terminal emulator itself, because tmux switching a
client that is behind a browser window changes nothing you can see. On
macOS with Alacritty that is `open -a Alacritty`, which focuses the existing
app rather than starting a second one.

### Who holds the lease

The tmux pane running `anchor attach`. One writer per PTY is anchor's
invariant; the writer is where you type. tallyd attaches to every session
too, but always as an **observer**
([04-signals.md](04-signals.md#the-watcher)) — so the grid watching an agent
never competes with the window driving it.

If a view window dies while attached, the lease is released by the
connection ending, and the next `anchor attach` takes it. Nothing to
reconcile.

### Recovering a view

A window can disappear — closed, tmux server restarted, a reboot. The
session is fine; the view is gone. Pressing the pad then **recreates the
view and jumps to it**, rather than reporting an error. The pad means "put
this in front of me", and whether that needs a new window is an
implementation detail of the press.

That is the second reason `@tally_session` is an option on the window rather
than a name convention: window names get renamed by programs, options do
not, and a jump that finds the wrong window is worse than one that creates
a new one.

## The alternatives, and why not

**A fresh Alacritty window per session.** Simpler to spawn, worse to live
with: no shared scrollback search, no copy mode, and macOS window
management is not a thing to build a product on. Also, you already live in
tmux.

**teleport's web UI.** `http://localhost:7337` + a session id opens the
right view from anything with a browser. That is genuinely useful and it is
the *phone* story, not the desk story — clicking a pad on the desk should
not raise a browser. See [07-seams.md](07-seams.md#teleport).

**`anchor attach` directly in a new pane in the current window.** Tempting
and wrong: the current window is where you are working. Jumping should not
rearrange the thing you jumped from.

## The assumption worth naming

**This design assumes tmux is running and is where you work.** If it is
not, the jump has no target and the whole surface degrades to a status
display.

That assumption is true here — `.tmux.conf`, `tmux-resurrect`,
`tmux-continuum`, and an existing jump binding all say so — and it should be
checked rather than inferred: on start, tallyd verifies a tmux server is
reachable and says so plainly if it is not. A surface whose central gesture
silently does nothing is worse than one that refuses to start.

The jump mechanism is therefore **one function behind one trait**, with the
tmux implementation being the only one, so that a second one is possible
later without the rest of the program knowing. That is not speculative
generality — it is the seam that keeps a hard platform dependency out of
the state layer.
