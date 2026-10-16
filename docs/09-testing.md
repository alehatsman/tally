# 09 — Testing

The output of this program is light. That is not untestable, but it does
mean the tests have to be about bytes, and the thing that cannot be tested
has to be named.

## Three layers, three kinds of test

### 1. The surface — pure byte functions

Encoding is total and has no I/O in it:

```text
pad(row, col)                    -> index
index(CC99)                      -> Logo
frame(model)                     -> Vec<colourspec>
diff(prev, next)                 -> one SysEx, only changed pads
scroll("edit tests/pty.rs?", 37) -> Vec<u8>
```

Golden tests against
[the three worked examples](02-surface.md#the-three-worked-examples-verbatim),
each appearing verbatim as an assertion. If our encoder cannot reproduce
the manual's own examples byte for byte, it is wrong, and that is knowable
without hardware.

Three more that are cheap and catch the mistakes that look plausible:

- **The row flip.** `pad("x:1:1")` is the *top* left in the label and the
  *bottom* left on the wire — logical row 1 encodes as `0x51`, logical row 8
  as `0x0B`. One assertion per corner, four total, because a flip that is
  wrong in one axis passes any test that only checks one corner.
- **Decimal-in-hex.** `pad` of logical (8,1) is `0x0B`, not `0x11`. The
  address space is decimal row-column and the wire is hex, and the two
  coincide for columns 1–9 of row 0 and nowhere else.
- **Title sanitisation.** An em-dash, a box-drawing character and a 200-byte
  title in; printable ASCII ≤64 bytes out, and **no byte ≥ `0x80` ever
  reaches a SysEx** — that one is a property test, not an example, because
  it is the failure that truncates a message mid-flight.

### 2. The signals — a fake anchor

This is the part that is worth real tests, because it is the part with
opinions in it. Feed recorded PTY byte streams through the deriver:

- bell then 3s of silence → `NeedsAnswer`
- bell then immediate output → **not** `NeedsAnswer` (it rang and kept going)
- **bell with trailing bytes in the same chunk** → not `NeedsAnswer`. This
  is the one the obvious implementation gets wrong
  ([04](04-signals.md#stating-that-precisely-because-the-obvious-phrasing-is-unimplementable)),
  and getting it wrong makes the pad amber after every bell forever.
- **bell followed by `\r\n` and nothing else** → `NeedsAnswer`, and the
  same holds whether the newline is in the bell's chunk or the next one.
  The mirror image of the above: treat the newline before the prompt as "it
  kept going" and the signal never fires at all. Two assertions, because
  which chunk it lands in is a buffering accident and both have to pass.
- bell, 3s, then output → back to `Working`, and the amber stops
- `NeedsAnswer` held for 10 minutes of continued silence → still
  `NeedsAnswer` (it is not a timer)
- OSC title split across a chunk boundary → one title, not two fragments
- 10MB of output with no bell → `Working` throughout, no allocation growth
- exit frame → `Exited`, and the watcher ends
- bell then exit → `Exited`, **not** `NeedsAnswer` — the precedence table
  ([04](04-signals.md#precedence)) in one assertion
- declared signal (`tl.attention`) overrides a derived one, and survives its
  source going quiet
- `tl.mute` present → renders `Working`/`Quiet`, never amber, from either
  source

And one set that is not about bytes at all, against a fake `list`:

- a session with no `tl.slot` → no watcher, not on the grid
- `tl.slot` naming an undeclared page, row 0, row 9, or `"garbage"` →
  **ignored, logged once**, not clamped to an edge
- two sessions on one slot → newest renders, older logged
- an unreachable `list` → the model is unchanged and the logo goes red; the
  grid does **not** empty

teleport's fixtures are the starting point. Record a real Claude Code
session once — `anchor logs <id>` gives the exact bytes — and that
recording becomes the fixture that outlives the heuristics.

### 3. The whole thing — against a real device, by hand

There is no way around this, and pretending otherwise is how a surface ships
with its rows upside down. A written manual checklist, run before each
milestone:

```text
[ ] plug in; tallyd finds the port by substring, confirms model via device inquiry
[ ] every pad pressed lands on the slot directly under the finger
[ ] bottom-left pad is slot (1,1), not (8,1)
[ ] top row switches pages; right column holds as modifiers
[ ] kill chord highlights only killable pads while held
[ ] release without tapping does nothing
[ ] hold a pad: title scrolls, top row and logo stay lit, release restores
[ ] unplug mid-run: no crash; replug: full repaint, correct state
[ ] SIGTERM tallyd: device returns to Live mode, Setup menu works again
[ ] SIGKILL tallyd, restart: device recovers to the correct grid
[ ] stop anchord: logo goes dark red, grid does not clear
[ ] chord ordering: tap a slot, then grab kill -- nothing is killed
[ ] hold kill, hold a slot for two seconds -- it kills, it does not scroll
[ ] with no tmux server at all: tallyd starts, grid is correct, a jump
    creates the server
```

The last six are the ones that will actually fail.

## What a green suite does not prove

Borrowed wholesale from anchor's `docs/08-testing.md`, because it applies
here harder:

- **That the colours are legible.** Nothing automated will tell you that
  amber-flashing and yellow-static look the same at 3 metres under a desk
  lamp. Only looking will.
- **That the gestures are learnable.** A chord that tests fine and that you
  cannot perform without looking at the device is a failed design.
- **That the attention signal is *right*.** Precision and recall of
  `NeedsAnswer` against real sessions is the actual quality metric of this
  program, and it is measurable only by using it and counting the times it
  was wrong.

  Keep that count **in a file**, not in your head:
  [`attention.jsonl`](04-signals.md#measuring-whether-any-of-this-works) is
  one line per transition, which turns [M1's exit
  criterion](../ROADMAP.md#m1--the-grid-tells-the-truth) from a feeling into
  a division. A week of remembering produces an opinion; a week of
  transitions produces a number, and the number is the one deciding whether
  the rest of the roadmap is worth building.

## The honest gap today

**Nothing in [02-surface.md](02-surface.md) has been executed against
hardware.** The Launchpad was not connected to this machine when these
documents were written (`system_profiler SPUSBDataType` found no Novation
device). The first milestone is therefore not code — it is
[M0](../ROADMAP.md#m0--prove-the-manual), plugging it in and making the
manual's own examples light a pad.
