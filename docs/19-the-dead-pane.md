# 19 — The dead pane says which session it was

> **Amended by [26](26-the-claude-session-id.md).** The line still carries
> anchor's id, and now leads with the *conversation* —
> `claude --resume <uuid>` — because that is the id that resumes
> something. The order below is 19's; 26 measured it at 80 columns and
> moved the payload.

A view pane outlives its command (`remain-on-exit on`,
[15](15-views.md#a-view-pane-outlives-its-command)). So after a kill — or after an agent exits on
its own — you are left looking at a corpse that shows its scrollback and
one line of tmux boilerplate:

```
Pane is dead (status 0, Sat Oct 17 00:44:01 2026)
```

That line is the last thing the view ever tells you, and it tells you
nothing. The session id is not in it, the pad is dark, and the row is still
in `anchor ls` — findable, but only by going and looking.

**Goal: the dead view names its own session, in text you can copy.**

## Not by typing `^C` first

The question this came from was "why not send Claude a `^C` before the
kill, so it shuts down cleanly and prints something". It cannot be done
here, and should not be:

- **`^C` is a byte, not an op.** anchor's control protocol is nine ops
  (`anchor/docs/04-protocol.md:31-39`); none of them writes to a PTY. The
  only write path is an `attach` stream holding the control lease
  (`anchor/docs/04-protocol.md:210`), and tally attaches `control:false`
  on purpose ([07](07-seams.md#anchor)).
- **The lease is exclusive.** If the view pane is open, *it* holds the
  write lease. Typing into the session from `tallyd` means taking control
  away from the terminal you are looking at.
- **The kill is already graceful.** `terminate` is `SIGHUP` to the group,
  `SIGTERM` to the child, a grace window, then `SIGKILL`
  (`anchor/src/pty.rs:674`). `^C` would add an ambiguity, not a courtesy:
  in Claude Code one `^C` cancels a turn and two quit, so a kill built on
  it would sometimes be a cancel.
- **Claude does not print its session id on exit anyway.** The id we want
  is anchor's, and tally already has it in hand at kill time.

So the id gets written by the side that knows it.

## The design: one more pane option

tmux already draws that line. Since 3.2 its text is the pane option
`remain-on-exit-format`, and formats expand user options — so the line can
read the tag the pane already carries ([15](15-views.md), `@tally_session`)
and no per-session string has to be stored anywhere.

```
set-option -p -t <pane> remain-on-exit-format \
  'tally · anchor attach #{@tally_session} · attach exit #{pane_dead_status}'
```

One constant string, set once per pane, next to the two options that are
already set there. **No work at kill time at all** — nothing to race, no
error path on the destructive gesture, nothing new that can fail while a
session is dying.

### Where it goes

`Tmux::tag` (`src/jump.rs:441`) — the single choke point. It already sets
`@tally_session` and `remain-on-exit`, and it is idempotent, so:

| path | covered because |
|---|---|
| a view tally creates | `view_pane` tags it (`src/jump.rs:479`) |
| a view tally revives | `view_pane` tags it after respawn (`src/jump.rs:459`) |
| a pane `tally spawn` adopts | `adopt` is `Tmux::tag` (`src/jump.rs:515`) |
| a view created before this existed | re-tagged on the next **jump** to it |

**"Every resolve" means every jump, not every tick.** `tag` is reached
only through `view_pane` and `adopt`, and the reconcile loop calls neither
— its one tmux call is `viewing()` (`src/bin/tallyd.rs:553`), which reads.
So a view that existed before this shipped keeps the old line until you
jump to its pad once. Measured after a `tallyd` restart: seven live view
panes, all still on tmux's default format.

That is the right trade and not worth a per-tick repair: tagging on every
reconcile would mean two `set-option` calls per view every 2s, forever, to
fix something one press fixes — and you press that pad before you kill it
anyway.

Pane-scoped (`-p`), for the reason `remain_on_exit_args` already documents
(`src/jump.rs:189-199`): an adopted view sits among panes that are yours,
and tally does not get to change how those behave.

### The line

```
tally · anchor attach 01JABCDEF0123456789TALLY · attach exit 0
```

- **The id goes early.** The line is truncated at pane width — measured,
  80 columns cut `anchor attach` mid-word — so what a narrow split loses is
  the tail, never the payload.
- **It is a command, not a label.** The thing you want after reading an id
  is to use it.
- **`attach exit` is named for what it is.** The pane ran `anchor attach`;
  its status is the attach client's, not the agent's. Calling it `exit 0`
  would be a lie on every kill, since the agent is the thing that died and
  `attach` returned cleanly. The agent's own exit is on the pad and in
  `anchor ls`; it is not tmux's to report.
- **No `tl.killed` distinction.** tmux cannot know whether you asked for
  it, and the label is tally's opinion, not a fact about the pane
  ([05](05-interaction.md#kill-is-the-one-that-needs-the-care)).

## Measured, on tmux 3.6b

✅ **`remain-on-exit-format` is a pane option and takes `-p`.**
`set-option -p -t <pane> remain-on-exit-format …` is accepted; `list-panes`
shows the override on that pane and the default on its siblings.

✅ **`#{@tally_session}` expands in it.** `display-message -p
'#{E:remain-on-exit-format}'` returned the id substituted in.

✅ **The line is real text in the pane.** `capture-pane -p` returns it, so
it is selectable in copy-mode — which is the whole point of putting the id
there rather than in `display-message`.

✅ **It does not clobber output.** The line is drawn on the pane's **last
row**; prior output scrolls into history exactly as it does today. This
changes the text of a line tmux already draws, and nothing else.

⚠️ **It must be set before the pane dies.** Setting it on an
already-dead pane does not repaint — the default text stays. Consequence:
panes that died before this shipped keep the old line until they are
revived and die again. That is acceptable (a revive is one pad press) and
it is why the option is set in `tag`, which runs long before any death.

⚠️ **A revive clears it.** `respawn-pane` restarts the command and the
dead line is gone from the screen and from history. Correct: the pane is a
view again, and the id is in the tag.

## Scope

- No change to `src/act.rs`. The kill stays three calls: label, terminate,
  report.
- No change to anchor, and no new op asked of it
  ([07](07-seams.md#what-we-must-not-ask-anchor-for)).
- No change to the grid, the colours, or the chords.

## Validation

- **Unit**, mirroring the existing option-args tests: the args carry `-p`
  and name their target, the format expands `#{@tally_session}` rather
  than carrying a literal id — which would be stale the moment a pane is
  rebound — and the status is labelled `attach exit`.
- **Not tested**: that `tag` sets the tag before the format. The ordering
  is real (an unset option expands to an empty line) but `tag` runs tmux,
  and a test over a locally built array would assert its own fixture. It
  is held by a comment at the call site instead.
- **End to end**, on a throwaway session rather than the live grid: a pane
  running `anchor attach <id>`, tagged with the three options, then
  `anchor kill`. The pane held, and said:

  ```
  anchor: session exited (1)
  tally · anchor attach 01M53EGV037VCGY3XGDX68ZMJF · attach exit 0
  ```

  ✅ 66 columns, so it survives an 80-column split whole. Note the line
  above it: `anchor attach` prints the agent's exit on its way out, which
  is why this line does not need to.
