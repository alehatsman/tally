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
| 2 | which palette entry is which | ✅ all ten confirmed, manual accurate. 21 vs 23 and 1 vs 0 checked by eye and legible |
| 3 | does flashing-against-0 read as "blink" | ✅ yes — and then replaced by a pulse anyway, because sustained flashing is tiring in a way a one-off check cannot reveal ([05](05-interaction.md#nothing-blinks)) |
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

✅ #6 closed too: five taps and a three-pad chord under a held `CC89`
produced exactly one release per press, the modifier never stuck, and a
five-second hold sent no key repeat — which also confirms
[05](05-interaction.md#what-a-press-does-not-do)'s "no key repeat" claim.

Shutdown was verified end to end on SIGTERM, Setup menu included, with and
without the clear step; both work
([08](08-lifecycle.md#shutdown-is-not-optional)).

### Still open

- **Whether note-off arrives as `8n` or `9n`.** The M0 logger accepted both
  and did not record which. [02](02-surface.md#addressing) says handle both,
  which is correct either way, so this is a curiosity rather than a risk.
- **Whether 21 and 23 stay legible under other lighting.** They photograph
  almost identically and were confirmed by eye once, in one room. They carry
  *running* versus *done*, so a second look in daylight is cheap insurance.

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
*name* declared in order in `slots.toml`, which is the order the pages
stack into the sixty-four-row space the grid scrolls through
([16](16-the-queue.md)). It used to map the page onto a top-row button.
Humans type this string into shell scripts, so it reads the way text reads;
the bottom-left-first flip lives in the surface layer and nowhere else.
[03](03-model.md#the-address-pinned).

### ~~Whether `tally` (the CLI) is needed at all~~ — settled, and built

Deferred with *"until something is actually awkward"*, which is exactly how
it resolved: spawning into the right folder was awkward, because **cwd is
the one fact `slots.toml` cannot know in advance and a shell always knows**.
`anchor ls` and `$EDITOR slots.toml` did cover everything else, and still
do — `spawn` is the only subcommand, for the only job nothing else could do
([11-cli.md](11-cli.md)).

The deferral was worth it. Had this been built in M1 it would have been a
wrapper around guesses; built after M3 shipped and went unused, it was built
around a measured gap.

### Whether respawn should stop asking `slots.toml`

Respawn looks up what to run in the config (`src/act.rs:85`), so every pad
claimed with `tally spawn` has a dead respawn button —
[11-cli.md](11-cli.md) makes that the common case rather than the rare one.
anchor already records a dead session's `command`, `args`, `cwd` and
`login_shell`, so respawn could re-create from the row it is standing on
and work everywhere. That changes what the held preview promises, so it
waits for a week of use to say whether the button is missed at all.

### Multiple devices, multiple machines

A second Launchpad on the same USB needs a different device ID, set in the
bootloader. Out of scope. A Launchpad driving agents on a *different*
machine is teleport's problem, not this one — but if it ever is this one,
the jump breaks first, not the grid.

## Risks worth watching

**anchor is moving.** It is at M5, actively shaped by teleport
([07-seams.md](07-seams.md#the-one-thing-to-watch)). Pinned at `e1503c9`;
read the diffs. Two anchor behaviours tally already depends on were
discovered rather than documented — the exact-match-only `list` filter, and
the label write that used to drop keys (#12, fixed) — so the diffs are not
optional reading.

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
