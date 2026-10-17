# 26 — The dead pane names its conversation

A view pane outlives its command (`remain-on-exit on`,
[15](15-views.md#a-view-pane-outlives-its-command)). So after a kill — or
after an agent exits on its own — you are left looking at a corpse: its
scrollback, and one line tmux draws at the bottom.

**That line is the last thing the view ever tells you.** By default it is
boilerplate and says nothing:

```
Pane is dead (status 0, Sat Oct 17 00:44:01 2026)
```

Here it names the conversation that died, as a command you can paste:

```
tally · claude --resume c1567a67-d7e0-43f1-bbd4-63832361f292 · anchor attach 01M…
```

## Which id, and why that one

Two ids are in reach and they are not equally useful.

**anchor's session id names a PTY that is already gone** by the time you
are reading the line. It is worth carrying — it is what `anchor logs`
takes, and it is always present — but it resumes nothing.

**Claude Code's session id is the conversation**, and it is the argument to
`claude --resume`. That is the thing you actually want after killing an
agent, and it is the reason this line exists rather than being left as
tmux's default.

So the line carries both, conversation first.

## Not by typing `^C` first

The obvious alternative is to send the agent a `^C` before the kill so it
shuts down cleanly and prints something itself. It cannot be done here, and
should not be:

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
- **Claude does not print its session id on exit anyway.**

So the line is written by the side that knows the ids, before anything
dies.

## Where the ids come from

Four hops, each of them something tally already does with `tl.attention`
or `@tally_session`:

```
hook (inside the agent)   tally attention on|off  — env var in hand
  └─ owning_session()     src/attention.rs        — which anchor session is this
      └─ anchor label     tl.claude=<uuid>        — one write, caller-owned
          └─ Tmux::tag    @tally_claude           — pane option, on jump
              └─ tmux     remain-on-exit-format   — the dead line
```

Nothing new is asked of anchor
([07](07-seams.md#what-we-must-not-ask-anchor-for)): `tl.claude` is a
label, and anchor never interprets labels.

### Hop 1 — the hook

Claude Code hands its session id to every hook two ways. Measured
2026-10-17 against `claude` 2.1.236, with a `SessionStart` and a `Stop`
hook dumping both:

| source | value |
|---|---|
| stdin JSON, `.session_id` | `c1567a67-d7e0-43f1-bbd4-63832361f292` |
| env `CLAUDE_CODE_SESSION_ID` | `c1567a67-d7e0-43f1-bbd4-63832361f292` |

**The env var, not the JSON.** Both are present on both events, so this is
a choice and not a constraint: reading an environment variable is a
`std::env::var`, and reading the JSON is a stdin slurp plus a parse in a
process that runs on every turn of every agent on this machine. The one
thing the JSON has that the env does not — `transcript_path` — is a file
nobody here opens.

**The id rides the attention hook** rather than one of its own. That hook
already runs on four events (`Stop`, `Notification`, `UserPromptSubmit`,
`PreToolUse`) and already pays the expensive half: a `ps -A` ancestry walk
plus an anchord `list` to find the session it is inside
([14](14-attention.md)). A `tally claude` on `SessionStart` would pay that
again, per turn, for a value that changes approximately never — and would
need a new line in `~/.claude/settings.json`, which on this machine is a
`~/dotfiles` change rather than an edit.

What that costs: the id lands on the **first tool call or first prompt**,
not at startup. An agent killed before it ever did anything has no
`tl.claude`, and the line says nothing about a conversation that holds
nothing.

**It is a second write in a function named for the first one**, and that is
the one judgment call here. It is accepted because the hook's real job is
*tell tally what this agent is*, and attention is one fact of two.

### Hop 2 — the label

```rust
/// The Claude Code session id, as its hook reported it. The argument to
/// `claude --resume`.
pub const LABEL_CLAUDE: &str = "tl.claude";
```

Written only when the value differs from the label already there, so a
`/clear` (new conversation, same agent) refreshes it and a quiet turn
writes nothing.

**Never removed.** A conversation outlives the PTY that ran it — that is
the entire point of wanting the id — so the label stays on the dead row,
where `anchor ls` and `>` can still see it.

### Hop 3 — the pane options

`@tally_claude` and `@tally_session`, both pane-scoped, both set in
`Tmux::tag` for the reason `-p` is load-bearing there ([15](15-views.md)).

**Tagged on jump, not on every tick.** `tag` is reached through
`view_pane` and `place`, and the reconcile loop calls neither — its one
tmux call reads. So a view that existed before a tag format changed keeps
the old line until you jump to its pad once, and **the pad you kill is the
pad you just jumped to.**

Tagging on every reconcile would mean two `set-option` calls per view
every 2 s, forever, to fix something one press fixes.

A view opened by `tally spawn` is tagged before the agent has a
conversation, so its `@tally_claude` is empty until the first jump. One
press fixes it, and the press is how you got there.

### Hop 4 — the line

```
set-option -p -t <pane> remain-on-exit-format \
  'tally#{?#{@tally_claude}, · claude --resume #{@tally_claude},} · anchor attach #{@tally_session} · attach exit #{pane_dead_status}'
```

One constant string, set once per pane, next to the two options already
set there. **No work at kill time at all** — nothing to race, no error path
on the destructive gesture, nothing new that can fail while a session is
dying.

Four things in that format are decisions:

- **The conversation goes first, because truncation eats the tail.**
  Measured at 80 columns with the anchor id leading:

  ```
  tally · anchor attach 01M54N0YHHENGSD9C4T1YRMTYE · claude --resume c1567a67-d7e0
  ```

  Half a UUID resumes nothing. Half an anchor id costs nothing — it names
  a PTY that is gone. So the conversation leads, and the same 80 columns
  now give:

  ```
  tally · claude --resume c1567a67-d7e0-43f1-bbd4-63832361f292 · anchor attach 01M
  ```

  A column is a full-width tab ([24](24-the-tally-server.md)), so nothing
  truncates on the real grid. This is for the narrow case, which is the
  only case where the order is a decision at all.
- **They are commands, not labels.** `claude --resume <uuid>`, not
  `conversation <uuid>`; `anchor attach <id>`, not `session <id>`. The
  thing you want after reading an id is to use it.
- **`attach exit` is named for what it is.** The pane ran `anchor attach`;
  the status is the attach client's, not the agent's. Calling it `exit 0`
  would be a lie on every kill, since the agent is the thing that died and
  `attach` returned cleanly. The agent's own exit is on the pad and in
  `anchor ls`; it is not tmux's to report.
- **No `tl.killed` distinction, and no `cwd`.** tmux cannot know whether
  you asked for the kill, and the label is tally's opinion rather than a
  fact about the pane
  ([05](05-interaction.md#kill-is-the-one-that-needs-the-care)). A `cwd`
  would make the line wrap, and the directory is on the row already.

## Measured, on tmux 3.6b

✅ **`remain-on-exit-format` is a pane option and takes `-p`.**
`set-option -p -t <pane> remain-on-exit-format …` is accepted; `list-panes`
shows the override on that pane and the default on its siblings.

✅ **User options expand in it.** `display-message -p
'#{E:remain-on-exit-format}'` returned the ids substituted in.

✅ **The conditional handles an absent id.** `#{?#{@tally_claude},…,…}` on
an **unset** user option takes the false branch and expands to nothing, so
a view of a plain shell keeps the anchor-only line.

✅ **The line is real text in the pane.** `capture-pane -p` returns it, so
it is selectable in copy-mode — which is the whole point of putting the id
there rather than in `display-message`.

✅ **It does not clobber output.** The line is drawn on the pane's **last
row**; prior output scrolls into history exactly as it does today. This
changes the text of a line tmux already draws, and nothing else.

✅ **End to end**, on a throwaway session: a pane running
`anchor attach <id>`, tagged, then `anchor kill`. The pane held, and said:

```
anchor: session exited (1)
tally · anchor attach 01M53EGV037VCGY3XGDX68ZMJF · attach exit 0
```

66 columns, so it survives an 80-column split whole. Note the line above
it: `anchor attach` prints the agent's exit on its way out, which is why
this line does not need to.

⚠️ **It must be set before the pane dies.** Setting it on an already-dead
pane does not repaint — the default text stays. That is why the option is
set in `tag`, which runs long before any death.

⚠️ **A revive clears it.** `respawn-pane` restarts the command and the dead
line is gone from the screen and from history. Correct: the pane is a view
again, and the ids are in the tags.

## Scope

- **No change to `src/act.rs`.** The kill stays three calls: label,
  terminate, report. No work at kill time is what makes this safe to put
  on the destructive path.
- No change to anchor, and no new op asked of it
  ([07](07-seams.md#what-we-must-not-ask-anchor-for)).
- No change to the grid, the colours or the chords.
- No new dependency. The env var is `std::env::var`.
- One field on `model::Resolved` (`claude: Option<String>`) and one
  argument on `Jump::jump`, because `src/jump.rs` knows about tmux and
  nothing else — it is told the id, it does not look one up.

## Edge cases

| case | behaviour |
|---|---|
| not a Claude session (a shell) | no env var, no label, no option; the line carries the anchor id alone |
| agent killed before its first turn | no `tl.claude` yet; same as above |
| `/clear` — new conversation, same PTY | next hook turn rewrites `tl.claude`; next jump re-tags |
| `claude --resume` from `>` | the resumed agent's hook reports whatever id it then has; the label follows the agent, not the pad |
| view created by `tally spawn` | tagged with an empty conversation until the first jump, which is the press that opens it |
| pane already dead | setting the option does not repaint; the pane keeps the line it died with |
| `tl.claude` on a dead row | kept. The conversation outlived the PTY |

## Validation

- **Unit**, mirroring the option-args tests in `src/jump.rs`: the args
  carry `-p` and name their target; the format expands `#{@tally_session}`
  and `#{@tally_claude}` rather than carrying a literal id, which would be
  stale the moment a pane is rebound; the conditional is present; the
  status is labelled `attach exit`.
- **Unit**, `src/bin/tally.rs`: the attention write carries `tl.claude`
  when the env var is set, and carries nothing extra when it is not.
- **Not tested**: that `tag` sets the tags before the format. The ordering
  is real — an unset option expands to an empty line — but `tag` runs tmux,
  and a test over a locally built array would assert its own fixture. It is
  held by a comment at the call site instead.
- **On the live grid**: spawn a pad, press it, let one turn happen, hold
  Stop, and read the dead tab.

## Open

1. **Does the tab name want it too?** A dead column's window name is
   `c4 · 1:1:4 · 7BQ9` — four characters of the *anchor* id. It could carry
   a conversation marker instead, which would make a dead row readable from
   the tab bar without opening a tab. Worth its own paragraph in
   [24](24-the-tally-server.md) if the dead line turns out not to be
   enough.
2. **Should `>` offer `claude --resume` rather than a fresh agent?** The
   label is on the dead row, so the launch column *could* reopen the
   conversation instead of starting one. It deliberately does not:
   [18](18-the-launch-column.md) drops arguments on purpose, because two
   pads on one conversation is the thing a new agent must not be. Recorded
   because the label is what makes it possible, and somebody will ask.
