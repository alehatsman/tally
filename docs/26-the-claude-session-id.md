# 26 — The dead pane says which *conversation* it was

**Status: spec.** It amends [19](19-the-dead-pane.md), which put anchor's
session id on the dead line, and [14](14-attention.md), which is where a
hook learned to find the session it is running inside.

[19](19-the-dead-pane.md) ends with a line that works:

```
tally · anchor attach 01M53RDBNZZ2S99ZRWVP5J7BQ9 · attach exit 0
```

Measured on the live grid, 2026-10-17: the tag, `remain-on-exit` and the
format are set on all four view panes, and a probe kill on a throwaway
socket printed that line whole. Nothing is broken.

It is also the wrong id. An anchor session is a PTY that is now gone;
what you want after killing an agent is **the conversation**, and that is
Claude Code's session id — the argument to `claude --resume`. anchor's id
resumes nothing.

> **Goal: the dead view names the conversation it was running, as a
> command you can paste.**

## Where the id is

Claude Code hands it to every hook, two ways. Measured 2026-10-17 against
`claude` 2.1.236, a `SessionStart` and a `Stop` hook dumping both:

| source | value |
|---|---|
| stdin JSON, `.session_id` | `c1567a67-d7e0-43f1-bbd4-63832361f292` |
| env `CLAUDE_CODE_SESSION_ID` | `c1567a67-d7e0-43f1-bbd4-63832361f292` |

**The env var, not the JSON.** Both are present on both events, so this
is a choice and not a constraint: reading an environment variable is a
`std::env::var`, and reading the JSON is a stdin slurp plus a parse in a
process that runs on every turn of every agent on this machine. The one
thing the JSON has that the env does not — `transcript_path` — is a file
nobody here opens.

`CLAUDECODE=1` is also set, which is how a non-Claude session tells itself
apart without the id having to carry that meaning too.

## The path the id takes

Four hops, each of them something tally already does with `tl.attention`
or `@tally_session`:

```
hook (inside the agent)   tally attention on|off  — env var in hand
  └─ owning_session()     src/attention.rs        — which anchor session is this
      └─ anchor label     tl.claude=<uuid>        — one write, caller-owned
          └─ Tmux::tag    @tally_claude           — pane option, on jump
              └─ tmux     remain-on-exit-format   — the dead line
```

Nothing new is asked of anchor ([07](07-seams.md#what-we-must-not-ask-anchor-for)):
`tl.claude` is a label, and anchor never interprets labels.

### Hop 1 — the hook, and which one

The hook that already runs is the attention one, on four events
(`Stop`, `Notification`, `UserPromptSubmit`, `PreToolUse`), and it already
pays for the expensive half of this: a `ps -A` ancestry walk plus an
anchord `list` to find the session it is inside (`src/attention.rs`,
[14](14-attention.md)).

**So the id rides the hook that is already there**: `tally attention
on|off` writes `tl.claude` alongside `tl.attention`, when
`CLAUDE_CODE_SESSION_ID` is set and differs from what the session already
carries.

The alternative — a new `tally claude` on `SessionStart` — is rejected,
and the reasons are worth keeping because it is the more obvious design:

- It doubles the per-turn cost (a second `ps -A` and a second `list`) for
  a value that changes approximately never.
- It is a **host change**: a new line in `~/.claude/settings.json`, which
  on this machine goes through `~/dotfiles` and `provision`, not a hand
  edit. The attention lines are already there.
- `SessionStart` is the only event that fires before the first turn, and
  the id is not needed before the first turn — it is needed at death.

What it costs: the id lands on the **first tool call or first prompt**,
not at startup. An agent killed before it ever did anything has no
`tl.claude`, and the dead line says nothing about a conversation that
holds nothing.

**This is a second write in a function named for the first one.** It is
accepted on the grounds that the hook's real job is *tell tally what this
agent is*, and attention is one fact of two — but it is the one judgment
call in this document, and it is called out in [Open](#open) rather than
buried.

### Hop 2 — the label

```rust
/// The Claude Code session id, as its hook reported it. The argument to
/// `claude --resume`.
pub const LABEL_CLAUDE: &str = "tl.claude";
```

Written only when the value differs from the label already there, so a
`/clear` (new conversation, same agent) refreshes it and a quiet turn
writes nothing.

Never removed. A conversation outlives the PTY that ran it — that is the
entire point of wanting the id — so the label stays on the dead row, where
`anchor ls` and `>` can still see it.

### Hop 3 — the pane option

`@tally_claude`, pane-scoped, set in `Tmux::tag` next to `@tally_session`,
for the reasons `-p` is load-bearing there already ([15](15-views.md)).

**Tagged on jump, not on every tick** — the same trade
[19](19-the-dead-pane.md) made and for the same reason: `tag` is reached
through `view_pane` and `adopt`, the reconcile loop calls neither, and the
pad you kill is the pad you just jumped to.

The gap this leaves is real and narrower than 19's: a view created by
`tally spawn` ([`place`](../src/jump.rs)) is tagged before the agent has a
conversation, so its `@tally_claude` is empty until the first jump. One
press fixes it, and the press is how you got there.

### Hop 4 — the line

```
tally · claude --resume <uuid> · anchor attach <anchor id> · attach exit 0
```

```
set-option -p -t <pane> remain-on-exit-format \
  'tally#{?#{@tally_claude}, · claude --resume #{@tally_claude},} · anchor attach #{@tally_session} · attach exit #{pane_dead_status}'
```

✅ **Measured, tmux 3.6b**: `#{?#{@tally_claude},…,…}` on an **unset** user
option takes the false branch and expands to nothing, so a view of a plain
shell keeps exactly the line 19 shipped. With the option set, the branch
expands with the id substituted.

- **Order: conversation, then anchor id, then status — and this is the
  one thing the first draft got wrong.** 19's rule is *payload early,
  because truncation eats the tail*, and the draft kept the anchor id
  first on the grounds that it is the half that is always there. Measured
  at 80 columns, with the conversation second:

  ```
  tally · anchor attach 01M54N0YHHENGSD9C4T1YRMTYE · claude --resume c1567a67-d7e0
  ```

  Half a UUID resumes nothing. Half an anchor id costs nothing — it names
  a PTY that is gone by the time you are reading the line. So the
  conversation goes first, and the same 80 columns now give:

  ```
  tally · claude --resume c1567a67-d7e0-43f1-bbd4-63832361f292 · anchor attach 01M
  ```

  A column is a full-width tab since [24](24-the-tally-server.md), so on
  the real grid nothing truncates either way. This is for the narrow
  case, which is the only case where the order is a decision at all.
- **`claude --resume <uuid>`, not `conversation <uuid>`.** Same rule as
  `anchor attach`: it is a command, not a label, because the thing you
  want after reading an id is to use it.
- **No `cwd`.** `claude --resume` wants to be run in the project the
  conversation belongs to, and that directory is on the row already
  (`anchor ls`, and `>` inherits it). Putting a `cd` on this line would
  make it wrap.

## Scope

- No change to `src/act.rs`. The kill stays three calls: label,
  terminate, report. **No work at kill time at all**, which is 19's rule
  and the reason this is safe to put on the destructive path.
- No change to anchor, the grid, the colours or the chords.
- No new dependency. The env var is `std::env::var`; `serde_json` is
  already in the tree and is not needed here.
- One field on `model::Resolved` (`claude: Option<String>`) and one
  argument on `Jump::jump`, because `src/jump.rs` knows about tmux and
  nothing else — it is told the id, it does not look one up.

## Edge cases

| case | behaviour |
|---|---|
| not a Claude session (a shell) | no env var, no label, no option; the line is 19's, unchanged |
| agent killed before its first turn | no label yet; same as above |
| `/clear` — new conversation, same PTY | next hook turn rewrites `tl.claude`; next jump re-tags |
| `claude --resume` from `>` | the resumed agent's hook reports whatever id it then has; the label follows the agent, not the pad |
| view created by `tally spawn` | tagged with an empty id until the first jump, which is the press that opens it |
| pane already dead | setting the option does not repaint — measured in 19; the pane keeps the line it died with |
| `tl.claude` on a dead row | kept. The conversation outlived the PTY |

## Validation

- **Unit**, mirroring the existing option-args tests in `src/jump.rs`:
  the dead format expands `#{@tally_claude}` through a conditional and
  carries no literal id; `tag` sets the claude option before the format,
  for the reason the session tag already goes first.
- **Unit**, `src/bin/tally.rs`: the attention write carries `tl.claude`
  when the env var is set, and carries nothing extra when it is not.
- **End to end**, on a throwaway socket and a throwaway anchor session —
  the shape 19 used and the shape this document's measurements already
  ran: tag a pane with both options, `anchor kill`, `capture-pane`, and
  read the line.
- **On the live grid**: spawn a pad, press it, let one turn happen, hold
  Stop, and read the dead tab.

## Decided

**The id rides `tally attention`** — asked and answered, 2026-10-17. The
alternative (a `tally claude` subcommand on `SessionStart`) buys the id
before the first turn and costs a second `ps -A` per turn plus a hook line
in `~/.claude/settings.json`, which on this machine is a `~/dotfiles`
change rather than an edit. Nothing on the host changes to ship this.

## Open

1. **Does the tab name want it too?** A dead column's window name is
   `c4 · 1:1:4 · 7BQ9` — four characters of the *anchor* id. It could
   carry a conversation marker instead, which would make a dead row
   readable from the tab bar without opening a tab. Out of scope here;
   worth its own paragraph in [24](24-the-tally-server.md) if the dead
   line turns out not to be enough.
2. **Should `>` offer `claude --resume` rather than a fresh agent?** The
   label is on the dead row, so the launch column *could* reopen the
   conversation instead of starting one. It deliberately does not:
   [18](18-the-launch-column.md) drops arguments on purpose, because two
   pads on one conversation is the one thing a new agent must not be.
   Recorded here because the label is what makes it possible, and
   somebody will ask.
