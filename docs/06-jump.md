# 06 — Jump

Press a live pad; end up looking at that agent. This is the gesture the
whole device exists for, and it is the one piece that is not about MIDI at
all.

> ⚠️ **The unit is a pane, not a window — see [15](15-views.md).**
> Everything below is right about *why* and out of date about *what*. This
> document says a view is a tmux window, which was true while `tallyd`
> under launchd was the only thing creating views, and became the
> definition by accident. A pane is the unit you can adopt, the unit that
> can outlive its command, and the unit a return gesture can name; three
> open issues were all downstream of that one sentence.
>
> Read `window` as `pane` throughout, with four substitutions:
> `list-windows -a` → `list-panes -a`, `set-option -w` → `set-option -p`,
> a view now carries `remain-on-exit`, and a dead view is revived **in
> place** rather than recreated elsewhere. The arguments are unchanged —
> they were arguments about identity and targeting, and both survive the
> change of unit.
>
> ⚠️ **Jump is also no longer one-directional.** `◄` returns you to where
> you were standing and `►` goes to whatever has been waiting longest
> ([16](16-the-queue.md)).

## The problem

An anchor session is a PTY in a daemon. It is not a window. Something has
to be attached to it for there to be anywhere to jump *to*, and that
something has to be findable and focusable.

## The answer: a tmux pane per session

*(Written as "a window per session"; see the amendment above.)*

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
       └──▶ tmux new-window -d -t tally: -n "work.3.5" "anchor attach <id>"
            tmux set-option -t <win> @tally_session <id>
```

### Both tmux commands need an explicit target, and it is not obvious

tallyd runs under launchd. It has no controlling terminal, no `$TMUX`, and
possibly **no attached client at all** — so every tmux default that resolves
to "the current one" resolves to something arbitrary, or fails.

- **`new-window` goes to a named session, `tally:`,** created with
  `new-session -d -s tally` if it is missing. Without `-t`, tmux picks the
  most-recently-used session, which is wherever you happened to be looking
  five minutes ago. View windows must not land in the middle of the session
  you are working in.
- **`switch-client` names its client.** `tmux list-clients -F '#{client_name}'`,
  and if there is exactly one, switch it. **Zero clients is not an error** —
  the view window exists and is correct, and the next time you attach it is
  there; tallyd logs it and moves on. More than one client is the case worth
  naming: switch **all** of them. A jump is "put this in front of me" and
  you do not know which terminal "me" is sitting at.

The window name `work.3.5` is cosmetic and carries no meaning —
`@tally_session` is the identity, for the reason below.

The tmux window is a *view*, and that is the whole reason this composes:
closing it does not touch the agent, because anchor owns the PTY. Kill the
window, reopen it from the same pad, and the scrollback replays from offset
0. That is anchor's entire pitch, used as intended.

Jump is then:

```sh
# find the pane whose @tally_session option equals the session id
tmux list-panes -a -F '#{pane_id} #{window_id} #{pane_dead} #{@tally_session}'
tmux switch-client -c "<client>" -t "<pane_id>"
```

`switch-client -t` takes a pane directly — tmux(1): *"As a special case,
-t may refer to a pane … to change session, window and pane."* One
command, not three.

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

A view can disappear — closed, tmux server restarted, a reboot. The
session is fine; the view is gone. Pressing the pad then **recreates the
view and jumps to it**, rather than reporting an error. The pad means "put
this in front of me", and whether that needs a new pane is an
implementation detail of the press.

There are now three cases, not two ([15](15-views.md#recovery-respawn-in-place)):
alive, switch to it; **dead, respawn in the pane it already occupies**;
absent, create one. The middle case is what makes an adopted pane
survivable — the pad goes on meaning the place you put it, instead of
quietly relocating to a window under `tally` the first time its agent
died.

That is the second reason `@tally_session` is an option on the window rather
than a name convention: window names get renamed by programs, options do
not, and a jump that finds the wrong window is worse than one that creates
a new one.

**Set it with `set-option -p`.** This paragraph used to say `-w`, and the
bug it describes has a smaller twin one level down. Without `-w`, tmux
sets a *session* option and every window inherits it. With `-w`, every
**pane** in the window inherits it — so splitting a view gives two panes
claiming the same session, and a jump lands on either. Both were observed
rather than theorised; the second was measured on tmux 3.6b while writing
[15](15-views.md), and did not bite only because nothing split a view
window until `tally spawn` learned to adopt one.

✅ Both of these are confirmed: a pad press reaches the right window, and a
killed view leaves the agent `running` and is recreated by the next press
with its scrollback replayed from offset 0.

### What the daemon's environment does not have

`tallyd` runs under launchd with `PATH=/usr/bin:/bin:/usr/sbin:/sbin`.
Neither `tmux` nor `anchor` is on it, so **the jump works when you run
`tallyd` by hand and fails as a service** — and a tmux server that `tallyd`
starts inherits that PATH, leaving its view windows unable to find `anchor`
either. The agent's `PATH` is therefore set in the LaunchAgent, declared in
`~/dotfiles/components/tally` ([07-seams.md](07-seams.md#provision)).

Worth stating because it generalises: everything this file asks of tmux is
an `exec`, and an agent's environment is not a login shell's.

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
checked rather than inferred: on start, tallyd runs `tmux has-session` and
says plainly in the log if there is no server.

**It says so; it does not refuse to start.** A missing tmux server is the
normal state at login, before anything is attached, and tallyd is explicitly
required to tolerate starting before everything else exists
([08-lifecycle.md](08-lifecycle.md#who-starts-it)). The grid is still
truthful with no tmux running — it is the *jump* that has no target, and
jump creates its own server when it has to (`new-session -d -s tally` on a
dead server starts one). Refusing to start would trade the half of the
product that works for the half that is temporarily unavailable.

The jump mechanism is therefore **one function behind one trait**, with the
tmux implementation being the only one, so that a second one is possible
later without the rest of the program knowing. That is not speculative
generality — it is the seam that keeps a hard platform dependency out of
the state layer.
