# 11 — The CLI

[10-open-questions.md](10-open-questions.md) deferred this one: *"`anchor ls
--label tl.slot=…` and `$EDITOR slots.toml` cover most of it. Defer until
something is actually awkward."* The deferral paid off — it is awkward now,
and we know exactly which part.

## The folder is the problem

Claiming a pad from a shell is this:

```sh
anchor create --cwd ~/projects/futurumlab/tally --login-shell \
  --label tl.slot=1:1:1 -- claude
```

Four facts, and **three of them the shell already knows.** The directory is
the one that stings, because it is the longest and the one that changes
every time.

The deeper reason is that **cwd belongs to where you were standing, not to
the pad.** `slots.toml` assumes otherwise: it wants the folder declared in
advance, which is a prediction about work you have not done. That is why
the spawn-from-a-dark-pad half of [M3](../ROADMAP.md#m3--spawn-and-the-destructive-chords)
sits unused — not because it is broken, but because the only context that
knows the folder is a shell sitting in it.

So the CLI is not a convenience wrapper. It is **the other half of
spawning**, for the case where the folder comes from you rather than from a
file.

## `tally spawn`

```text
tally spawn <row:col | page:row:col> [-- command [args…]]
```

```sh
$ cd ~/projects/futurumlab/anchor
$ tally spawn 1:1
spawned 01M53KQ… on 1:1:1

$ tally spawn work:3:4 -- cargo watch -x test
spawned 01M53KR… on work:3:4
```

| | | why |
|---|---|---|
| address | **required** | you choose the pad, so you know where to look without reading output |
| page | defaults to the first | two parts is `row:col`, three is `page:row:col` |
| command | defaults to `claude` | the thing this grid is for |
| cwd | **always `$PWD`** | the entire point; there is no `--cwd`, `cd` is the flag |
| login shell | always | you want your `PATH`, and a spawn that cannot find `claude` is not useful |

**No `--cwd` is deliberate.** A flag would re-admit the thing we are
removing — a directory typed by hand — and the shell has a perfectly good
one already. Scripts that want a different folder can use a subshell, or
call `anchor create` directly, which stays supported.

## It is exactly a pad press, with a folder

The occupancy rule is not invented here. `tally spawn` **refuses an address
the grid is drawing and allows one it is not**, which is the same predicate
`model::resolve` already applies (`src/model.rs:70`), run with no watchers.

That gives the behaviour you would guess from looking at the device:

| pad | `tally spawn` |
|---|---|
| dark | spawns |
| green, amber | refuses — something is alive there |
| red, dark red | refuses — something wants reading |
| dark, with exited rows behind it | spawns, and the old rows stay for `anchor logs` |

The fourth row is the interesting one and it is not a special case:
a clean exit holds its `tl.slot` in anchor but renders as nothing
([03](03-model.md#exited-rows-accumulate-and-purge-is-the-answer)), so the
pad is dark and pressing it spawns a second session. The CLI does the same,
by asking the same function. **One predicate, read by a finger and by a
shell** — if they ever disagree, the grid is lying about what a press does.

A failed or lost pad refusing is the deliberate half: red has one job, which
is *go and look*, and spawning over it would erase the only sign. Purge it
from the grid, then spawn.

## What it does not do

- **No socket to tallyd.** [08-lifecycle.md](08-lifecycle.md#one-process-no-socket)
  argued tallyd needs no control socket and that still holds — the CLI talks
  to **anchord**, writes a label, and tallyd notices within one reconcile
  (≤2 s). The grid is downstream of anchor's state, never of this process.
- **No `tl.spec`.** A spec label is a *name looked up in `slots.toml`*
  (`Config::by_spec_name`, `src/config.rs:131`). Writing one that no
  `[[spec]]` declares would be a label that lies, and worse, a respawn that
  found a spec would spawn in the **spec's** cwd rather than the folder you
  spawned from — the exact bug this document exists to fix.
  **Consequence: the respawn chord does nothing on a pad claimed this way.**
  Kill, purge and mute all work. See the open question below.
- **No pad-picking.** It will not find you a free slot. The address is how
  you build muscle memory, and a grid that assigns addresses is one you look
  up rather than know.
- **No other subcommands.** `anchor ls` already lists, `$EDITOR` already
  edits. `spawn` exists because nothing else could do it.

## Errors

Usage problems exit `2`, everything else exits `1`, and anything that
reaches anchord reports what anchord said.

| | |
|---|---|
| `9:1`, `a:b`, `1:2:3:4` | refused, **not clamped** — same rule as a label, and for the same reason ([03](03-model.md#limits)) |
| a page name no `slots.toml` declares | refused, and lists the ones that exist |
| the pad is drawn | refused, naming the session holding it and its state |
| anchord unreachable | refused — a surface that cannot see is not a surface that spawns |
| `--` with nothing after it | refused; an empty argv is a config error, not a spawn of the empty string |

## Open question — respawn on a shell-spawned pad

Respawn needs to know what to run again, and today it asks `slots.toml`
(`src/act.rs:85`). Every pad claimed from the CLI therefore has a dead
respawn button, and this document makes that the *common* case rather than
the rare one.

The fix is probably to stop asking the config: **anchor already records a
dead session's `command`, `args`, `cwd` and `login_shell`**, so respawn
could re-create from the row it is standing on and work everywhere,
`slots.toml` or not. That is a change to `effect`'s `pad.spec` predicate
(`src/model.rs:204`) and to what the held preview promises, so it is not
folded in here.
