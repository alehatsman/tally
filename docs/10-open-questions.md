# 10 — Open questions

What these documents assert but have not proven, and the decisions still
open. Read this before M1.

## Unverified against hardware

Everything in [02-surface.md](02-surface.md). The Launchpad was not plugged
into this machine when it was written — `system_profiler SPUSBDataType`
found no Novation device. Specifically unproven:

1. **The exact CoreMIDI port name on macOS.** The manual names the
   interfaces `LPMiniMK3 DAW In/Out` and `LPMiniMK3 MIDI In/Out`; what
   `midir` actually reports on macOS 15 is a different question, and the
   substring we match on depends on the answer.
2. **Which palette entry is which colour.** The entries in the colour table
   are read off a screenshot of the manual's palette grid plus its three
   worked examples. They are probably right. "Probably" is not a thing to
   build a colour language on.
3. **Whether flashing against colour 0 reads as "blink"** or as something
   dimmer and less urgent. The whole attention design rests on it.
4. **SysEx throughput.** 81 colourspecs is ~270 bytes; whether the device
   keeps up with one such message every 2s plus bursts on input is
   untested, as is whether `midir`'s macOS backend chunks it.
5. **Latency of a press.** It should be a few milliseconds. If it is not,
   hold-then-tap chording will feel wrong and the gesture design changes.
6. **Whether a held chord reads as held.** Hold-then-tap assumes the device
   reports note-off on release reliably and promptly. A missed note-off
   leaves a modifier stuck down, which on `CC19` means the next innocent tap
   kills an agent. [05](05-interaction.md#the-exact-rules-of-a-chord) is
   written as if this works; it is the one gesture assumption that is
   dangerous rather than merely wrong.

M0 exists to close all six.

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
