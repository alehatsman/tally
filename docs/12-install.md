# 12 — Install

**What a stranger does, in order, to get a lit grid.** It is one of the
four statements [the finish line](../ROADMAP.md#the-finish-line) calls
finished, and the half of it that was missing longest was **the hook
lines** — the attention half of this product does not exist without them,
and they lived only in prose in [04](04-signals.md).

> **This machine does not follow these steps.** The owner's install is
> declared in `~/dotfiles` and applied with provision, which is also why
> there is no `tally service install` subcommand
> ([08](08-daemon.md#who-starts-it)). This file is for everyone else, and
> it is the only place the whole sequence is written down.

---

## What it needs first

| | |
|---|---|
| **a Launchpad Mini MK3** | the one device, by USB. No other grid ([01](01-scope.md#how-to-reject-a-feature)) |
| **[anchor](https://github.com/alehatsman/anchor)**, running | it owns the PTYs. tally is a client and does nothing without one ([01](01-scope.md#what-it-is)) |
| **tmux** | the jump target is a pane, on [tally's own server](06-views.md) |
| **Rust**, stable | five dependencies: `libc`, `midir`, `serde`, `serde_json`, `toml` ([03](03-surface.md)) |

**macOS and Linux.** It is developed on macOS; the MIDI layer is `midir`,
which speaks CoreMIDI and ALSA behind one API. Windows is
[the bridge](11-unbuilt.md#the-bridge), which is not built.

## 1. Build and install the binaries

```sh
cargo install --git https://github.com/alehatsman/tally --tag v0.9.0
```

Two binaries land in `~/.cargo/bin`: **`tallyd`**, the daemon that owns
the device, and **`tally`**, the CLI the hooks and your shell call.

**Not on crates.io, deliberately.** It drives one USB device on one OS and
`Cargo.toml` carries `publish = false`. A tag and `--git` is the size this
is.

```sh
tally --help          # it is installed
tally ls              # the grid as text; works with no device plugged in
```

`tally ls` printing an empty grid is success. **`tally: cannot reach
anchord`** is the one real failure here, and it means step 0 is not done.

## 2. The hook lines

**This is the step the product lives on.** Without it the grid is a
launcher with liveness and the amber never fires — the agent does not ring
the terminal bell, which is the finding the whole signal chapter is built
on ([D5](10-decisions.md#d5--the-bell-that-is-never-rung)).

In `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Notification": [
      { "hooks": [{ "type": "command", "command": "tally attention blocked || true" }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "command", "command": "tally attention done || true" }] }
    ],
    "UserPromptSubmit": [
      { "hooks": [{ "type": "command", "command": "tally attention off || true" }] }
    ],
    "PreToolUse": [
      { "hooks": [{ "type": "command", "command": "tally attention off || true" }] }
    ]
  }
}
```

**Four hooks and not two.** Two raise, two clear, and both halves of each
pair are load-bearing — the reasoning, including why `Stop` rather than
only `Notification`, is in [04](04-signals.md#4-a-hook--it-told-us).

**`|| true` is not superstition.** Claude Code gives exit 2 a meaning that
differs per hook: a `Stop` hook exiting 2 blocks the agent from stopping, a
`PreToolUse` hook exiting 2 denies the tool call. A mistyped word in this
file must not produce an agent that cannot finish a turn
([04](04-signals.md#-true-and-it-is-not-superstition)).

**`tally` has to be on the hook's `PATH`**, which is not your interactive
shell's. If the pads never go amber and everything else works, this is the
first thing to check — use an absolute path if in doubt.

### Checking it

Start an agent under anchor on a pad, let it finish a turn, and:

```sh
tally ls --json | grep wants      # "wants": "finished"
tail -1 ~/.local/state/tally/attention.jsonl
```

A line in `attention.jsonl` is the hook arriving. **No line is the hook
not running**, not a slow grid.

## 3. Start the daemon

```sh
tallyd
```

It tolerates being started before anything else exists — no device, no
anchord, no tmux server — and says which one is missing while it waits
([08](08-daemon.md#who-starts-it)). Plug the Launchpad in at any point and
it is picked up on a later tick.

**One `tallyd` at a time**, enforced by a lock at
`$XDG_RUNTIME_DIR/tally.lock` rather than by the OS: CoreMIDI will happily
let two processes open the same endpoint and interleave their frames
([08](08-daemon.md#one-process-no-socket)).

To run it at login, declare a launchd user agent (macOS) or a systemd user
unit (Linux) that runs `tallyd` and restarts it. There is no subcommand
that writes one for you, on purpose.

## 4. Claim a pad

```sh
cd ~/projects/something
tally spawn 1:1              # row 1, column 1, in this directory
```

The pad lights. Press it and you are looking at the terminal it is running
in. That is the product; everything else is
[05](05-interaction.md).

**`cwd` comes from the shell**, which is the measurement behind cutting the
config-file spawn path entirely
([D8](10-decisions.md#d8--slotstoml-as-the-spawn-path), #40).

## Optional: name the pages

`~/.config/tally/slots.toml`, and it holds one table:

```toml
[[page]]
name = "work"
[[page]]
name = "infra"
```

Then `work:3:5` is an address you can type. **No file is eight unnamed
pages** (`1`–`8`) and a fully working grid — on the machine this was
written on, the file does not exist
([02](02-model.md#pages-are-named-in-a-file)).

## The environment it reads

All optional, all with working defaults.

| | |
|---|---|
| `TALLY_TERMINAL` | the app a jump brings forward. Default `Alacritty` — **the one genuinely machine-specific string in the program** |
| `TALLY_TERMINAL_CMD` | how to start that terminal if none is running. Split on spaces, deliberately not a shell |
| `TALLY_RAISE_CMD` | how to bring it forward when one is. The test suite sets it, because otherwise every integration jump steals the screen of whoever ran `cargo test` |
| `TALLY_TMUX_SOCKET` | tally's own tmux server. Changing it does not touch yours either way ([06](06-views.md)) |
| `XDG_RUNTIME_DIR`, `XDG_STATE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME` | the lock, the journal, `slots.toml`, and where anchord's socket is looked for |

## Uninstall

```sh
rm ~/.cargo/bin/tally ~/.cargo/bin/tallyd
```

Then remove the four hook lines and whatever starts the daemon. **Nothing
else has to be undone**: tallyd owns no session, holds no lease, and
persists nothing that cannot be rebuilt from an `anchor list`
([01](01-scope.md#the-boundary-stated-plainly)). The labels it wrote stay
on anchor's sessions and are inert without it.

The journal at `~/.local/state/tally/attention.jsonl` and `slots.toml` are
yours and are left alone.

## When it does not work

| | |
|---|---|
| **grid is dark, `tally ls` has rows** | the device. `tallyd` logs which port it found; unplug and replug — there is no hot-plug notification in either direction ([03](03-surface.md)) |
| **the logo is dark** | `tallyd` is not running. A live daemon holds the logo **green**; dark red is a reachable surface with an unreachable `anchord` ([05](05-interaction.md#the-logo-is-the-health-light)) |
| **sixty-four dark pads** | *nothing is running*. It must never mean *I cannot tell* — an unreachable anchord takes the **logo dark red** and holds the last grid ([01](01-scope.md#the-boundary-stated-plainly)) |
| **pads never go amber** | the hooks. See step 2, and check `PATH` first |
| **a pad is amber and the agent is working** | a hook that did not fire — the one failure [the level](11-unbuilt.md#the-level) was specified against, and it has never been observed in 544 rising edges |
| **the Setup menu is dead** | programmer mode. `SIGTERM` to `tallyd` restores Live mode on the way out; a `SIGKILL` does not, and a replug does ([03](03-surface.md)) |
| **a jump opens nothing** | `TALLY_TERMINAL`. The default is `Alacritty` and it is almost certainly not what you run |
