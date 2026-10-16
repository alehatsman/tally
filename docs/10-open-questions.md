# 10 — Open questions

What these documents assert but have not proven, and the decisions still
open. Read this before M1.

## Verified against hardware (2026-10-16)

The device was connected and [M0](../ROADMAP.md#m0--prove-the-manual) run
against it. Firmware `00 03 04 01`. **The manual was accurate**; the two
things that proved wrong were both ours.

| # | question | outcome |
|---:|---|---|
| 1 | CoreMIDI port name on macOS | ✅ exactly as the manual names them. `LPMiniMK3 MIDI` matches under both `kMIDIPropertyName` and `kMIDIPropertyDisplayName` |
| 2 | which palette entry is which | ◐ **9 = amber** and **21 = green** confirmed; eight entries still unchecked |
| 3 | does flashing-against-0 read as "blink" | ✅ reads as urgent, by eye |
| 4 | SysEx throughput | ✅ a full-surface frame arrives and renders in one message; no chunking, no desync after 100 worst-case frames |
| 5 | latency of a press | ✅ ~1.9 ms round trip — 5× margin on the 10 ms the chording needs |
| 6 | note-off on release, incl. under chord | ⬜ **still open** — needs fingers on the device |

Two corrections went the other way, and both were errors in these documents
rather than in the manual:

- **The port is not exclusive on macOS.** CoreMIDI multiplexes; two
  processes drove the device at once and neither was told. `tallyd` needs
  its own single-instance lock.
  [02](02-surface.md#the-port-is-not-exclusive-and-we-assumed-it-was),
  [08](08-lifecycle.md#one-process-no-socket).
- **SysEx overhead is 8 bytes, not 7.** A full 81-LED frame is 251 B static
  / 332 B all-flashing, not 250 / 331.

### Still open

- **Note-off on release** (#6 above). The dangerous one: a modifier stuck
  down turns the next ordinary tap into a kill.
- **Eight palette entries** — 1, 3, 5, 7, 13, 23, 37, 45.
- **Whether dropping the clear fixes the Live-mode handover.** Restoring
  used to clear all 81 LEDs before sending Live mode, and that left the
  device looking wrong ("it restored, kind of"). The clear has been removed
  on the theory that Live mode does not repaint what we blanked; the
  replacement has not been looked at yet.
  [08](08-lifecycle.md#shutdown-is-not-optional).

## Decisions still open

### ~~The project's name~~ — settled

`tally`, after the tally light: the lamp on a live television camera that
tells the studio, from across the room, which of many is the one to look at.
See [README](../README.md#the-name). Binaries `tally` / `tallyd`, label
prefix `tl.`.

Recorded here rather than deleted because the name encodes a claim about
what this is — **a thing that tells you without being asked** — and a
feature request that fights the name is usually a feature request that
fights [01-scope.md](01-scope.md).

### ~~How tallyd finds its sessions~~ — settled

anchor's `list` filter is exact-match-on-value and `tl.slot`'s value varies
per slot, so there is no server-side query for "has a `tl.slot`". tallyd
lists **everything** and filters client-side, rather than adding a constant
`tl.owner` label purely to make a query work — because that label would
become mandatory and `anchor create --label tl.slot=… -- claude` from a
shell script would silently not light a pad.
[03](03-model.md#where-the-binding-lives), [07](07-seams.md#anchor).

### ~~The channel a `Notification` hook writes to~~ — settled

A label, `tl.attention`, on the session — not a socket and not a FIFO. It
needs no new protocol, it survives tallyd restarting, teleport can see it,
and it costs one reconcile tick of latency against a 2s budget.
[04](04-signals.md#4-a-hook--it-told-us), [08](08-lifecycle.md#one-process-no-socket).

What is still open is **whether the hook ships in M1 or later** — that
depends on how good bell+silence turns out to be in real use, which is
unknown until it is used. The label and the `Source` variant exist either
way; only the `settings.json` line is deferred.

### ~~Whether a pad shows that you are looking at it~~ — settled, cut

An earlier draft rendered the focused slot white. It is gone. tallyd would
have to poll tmux every tick to know, and white overwrites *working* on the
one pad whose working state matters most. The window you are focused on is
in front of your face; the grid is for the sixty-three that are not.
[05](05-interaction.md#the-colour-language).

### ~~What a slot address means~~ — settled

`page:row:col`, with **row 1 at the top**, col 1 at the left, and the page a
*name* declared in order in `slots.toml` which maps it onto `CC91`–`CC98`.
Humans type this string into shell scripts, so it reads the way text reads;
the bottom-left-first flip lives in the surface layer and nowhere else.
[03](03-model.md#the-address-pinned).

### Whether `tally` (the CLI) is needed at all

[08-lifecycle.md](08-lifecycle.md) argues it needs no socket, which makes it
cheap. It may also make it pointless: `anchor ls --label tl.slot=…` and
`$EDITOR slots.toml` cover most of it. Defer until something is actually
awkward.

### Multiple devices, multiple machines

A second Launchpad on the same USB needs a different device ID, set in the
bootloader. Out of scope. A Launchpad driving agents on a *different*
machine is teleport's problem, not this one — but if it ever is this one,
the jump breaks first, not the grid.

## Risks worth watching

**anchor is moving.** It is at M5, actively shaped by teleport
([07-seams.md](07-seams.md#the-one-thing-to-watch)). Pin a commit; read the
diffs.

**The OSC heuristics rest on Claude Code's undocumented output format.**
teleport's own scanner says so in its doc comment. Degrade silently, never
depend on it.

**The device could be the wrong input modality.** The honest failure case
for this project is not a bug — it is finishing it and finding that
`prefix C-j` and a fuzzy finder were faster all along, because your hands
are already on the keyboard. The thing a grid does that a fuzzy finder
cannot is **tell you without being asked**: peripheral, always-on,
no-query. If after a week the answer is "I look at the grid and then use
the keyboard anyway", the attention half was the product and the launcher
half was decoration — and that is worth knowing early, which is why M1 is
"it lights up correctly" and not "it spawns things".
