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

Golden tests against the hex in
[02-surface.md](02-surface.md), lifted from the manual's own worked
examples — the manual gives three (`90 0B 05`, `91 51 13`, and the
three-colourspec SysEx) and each should appear verbatim as an assertion.
If our encoder cannot reproduce the manual's examples byte for byte, it is
wrong, and that is knowable without hardware.

The bottom-left-is-row-1 flip gets its own test, because it is the one
piece of arithmetic that is easy to get wrong in a way that still looks
plausible on screen.

### 2. The signals — a fake anchor

This is the part that is worth real tests, because it is the part with
opinions in it. Feed recorded PTY byte streams through the deriver:

- bell then 3s of silence → `NeedsAnswer`
- bell then immediate output → **not** `NeedsAnswer` (it rang and kept going)
- OSC title split across a chunk boundary → one title, not two fragments
- 10MB of output with no bell → `Working` throughout, no allocation growth
- exit frame → `Exited`, and the watcher ends
- declared signal (hook) overrides a derived one, and surviving its source
  going quiet

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
```

The last four are the ones that will actually fail.

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
  was wrong. Keep that count.

## The honest gap today

**Nothing in [02-surface.md](02-surface.md) has been executed against
hardware.** The Launchpad was not connected to this machine when these
documents were written (`system_profiler SPUSBDataType` found no Novation
device). The first milestone is therefore not code — it is
[M0](../ROADMAP.md#m0--prove-the-manual), plugging it in and making the
manual's own examples light a pad.
