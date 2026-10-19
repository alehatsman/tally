# 09 — Testing

The output of this program is light. That is not untestable, but it does
mean the tests have to be about bytes, and the thing that cannot be tested
has to be named.

## The shape the suite ended up in

386 unit tests in `#[cfg(test)]` modules beside the code, none of them
needing a device, a daemon or a socket — plus 17 in `tests/relocate.rs`,
which drive **real tmux** on a scratch socket, and 8 in `tests/bridge.rs`,
which put `Device` on a real loopback socket against the bridge's real
session with a fake Launchpad behind it ([13](13-bridge.md)). The bridge's
own codec and session tests live in `bridge/` and run on Windows too. The unit half is not
discipline;
it is a consequence of where the boundaries were drawn, and it is the
property to protect:

> **Every module that decides something is pure.** `chord` takes presses and
> emits gestures. `model` takes a session list and emits a frame. `queue`
> takes a map and emits a head. `tempo` takes an age and emits a number.
> `effect`, `render`, `slot`, `art`, `osc`, `signal` and `surface` are the
> same shape. The three that touch the world — `anchor`, `device`, `jump` —
> are tested on the **arguments they build**, not on the calls they make.

## 1. The surface — pure byte functions

Encoding is total and has no I/O in it:

```text
pad(row, col)                    -> index
index(CC99)                      -> Logo
frame(model)                     -> Vec<colourspec>
diff(prev, next)                 -> one SysEx, only changed pads
```

Golden tests against
[the three worked examples](03-surface.md#the-three-worked-examples-verbatim),
each appearing verbatim as an assertion. If our encoder cannot reproduce the
manual's own examples byte for byte, it is wrong, and that is knowable
without hardware.

Three more that are cheap and catch the mistakes that look plausible:

- **The row flip.** `pad("x:1:1")` is the *top* left in the label and the
  *bottom* left on the wire — logical row 1 encodes as `0x51`, logical row 8
  as `0x0B`. One assertion per corner, four total, because a flip that is
  wrong in one axis passes any test that only checks one corner.
- **Decimal-in-hex.** `pad` of logical (8,1) is `0x0B`, not `0x11`. The
  address space is decimal row-column and the wire is hex, and the two
  coincide for columns 1–9 of row 0 and nowhere else.
- **No byte ≥ `0x80` ever reaches a SysEx.** A property test rather than an
  example, because it is the failure that truncates a message mid-flight.

## 2. The signals — a fake anchor

The part with opinions in it. Recorded PTY byte streams fed through the
deriver:

- bell then 3 s of silence → `NeedsAnswer`
- bell then immediate output → **not** `NeedsAnswer` (it rang and kept going)
- **bell with trailing bytes in the same chunk** → not `NeedsAnswer`. This
  is the one the obvious implementation gets wrong, and getting it wrong
  makes the pad amber after every bell forever
- **bell followed by `\r\n` and nothing else** → `NeedsAnswer`, and the same
  whether the newline is in the bell's chunk or the next one. The mirror
  image of the above: treat the newline before the prompt as "it kept going"
  and the signal never fires at all. Two assertions, because which chunk it
  lands in is a buffering accident and both have to pass
- bell, 3 s, then output → back to `Working`, and the amber stops
- `NeedsAnswer` held for 10 minutes of continued silence → still
  `NeedsAnswer` (it is not a timer)
- OSC title split across a chunk boundary → one title, not two fragments
- 10 MB of output with no bell → `Working` throughout, no allocation growth
- exit frame → a terminal state, and the watcher ends
- bell then exit → terminal, **not** `NeedsAnswer` — the precedence table in
  one assertion
- declared (`tl.attention`) overrides derived, and survives its source going
  quiet
- `tl.mute` present → renders `Working`/`Quiet`, never amber, from either
  source

> ⚠️ **Every one of these passes and none of them fires in real life.**
> Claude Code emits zero ground-state bells, so rule 5 is unreachable
> against the agent this grid actually runs. The suite is green because the
> detector is *correct*; the amber comes from a hook. That is the most
> useful thing this file has to say about what a green suite means — **a
> working detector with nothing to detect looks exactly like a working
> feature** ([D5](10-decisions.md#d5--the-bell-that-is-never-rung)).

And one set that is not about bytes at all, against a fake `list`:

- a session with no `tl.slot` → no watcher, not on the grid
- `tl.slot` naming an undeclared page, row 0, row 9, or `"garbage"` →
  **ignored, logged once**, not clamped to an edge
- two sessions on one slot → newest renders, older logged
- an unreachable `list` → the model is unchanged and the logo goes dark
  red; the grid does **not** empty
- attention scrolled off the window → `►` pulses amber and the logo is
  untouched; health is the only question it answers

## 3. The gestures — pure, and the reason they are

`chord` takes a button and a direction and answers with a gesture or
nothing. It has no clock, no session list and no device, and the
[four chord rules](05-interaction.md#the-exact-rules-of-a-chord) are
asserted directly against it.

**The two timed gestures are deliberately not in there.** Hold-`▲` for home
and the double-tap on `CC19` both live in `tallyd`'s 16 ms loop, which
already has `elapsed` on the line that calls `feed`. The consequence is
stated so nobody goes looking: `Gesture::Home` is in the enum, `chord.rs`
never emits it, and a test asserts exactly that.

> **The bug that justifies the split is also where the split hurts.** The
> mode toggle froze the surface on its first press: `tallyd` flipped the
> verb before `chord.rs` saw the release, so the release was matched against
> a verb that no longer existed and the modifier stranded in `held` forever.
> Every unit test fed presses through `Chords` alone, where the mode never
> moves. **The toggle lives in `tallyd`, the stranding happened in
> `chord.rs`, and the bug was in neither — it was in the order the two ran**
> ([D9](10-decisions.md#d9--two-correct-modules-in-the-wrong-order)).
> Matching a release by *button* rather than by verb makes the pairing true
> by construction, which is the only kind of fix a unit test can hold onto.

Also asserted here: kill and purge targets are disjoint for every `Pad`; the
preview and the press read the same `effect`; the mode toggles on two taps
and not on one and not on an aborted hold (the sharpest one, because a
single-tap toggle would have broken chord rule 4 silently); a long press
clears a half-started double; the mode survives `Chords::clear`, because a
replug forgets the hold and keeps the verb; and **exactly one thing in a
move animates, and it is the pad in your hand** — which also asserts amber
was not dragged along with it.

## 4. The world-facing three

`anchor`, `device` and `jump` are tested on the arguments they construct and
never on the calls they make:

- every tmux invocation has an args test — `-p` and not `-w` on a pane
  option, an explicit `-t` on anything that would otherwise resolve to "the
  current one", `remain-on-exit-format` expanding `#{@tally_session}`
  through a conditional rather than carrying a literal id that would be
  stale the moment a pane is rebound, `swap-window` aiming at a pane and
  never a bare `rNN:`
- the pane listing is parsed by position with an explicit `|` separator, and
  the parse is asserted to hold with **two** empty fields. That test exists
  because a stray `split_whitespace` in `back()` would have silently broken
  `◄` when the separator changed
- `relocation()`'s plan is a pure function of the listing, and every row of
  its table is asserted: at the address, empty destination, dead occupant
  (`-k`), live occupant (swap), absent session (placeholder)
- `Jump` is a trait with one real implementation, so `tallyd`'s press
  handling runs against a fake that records what it was asked for
- `device` over the bridge is the one exception to *arguments, not calls*,
  because a socket can be real without hardware: `tests/bridge.rs` checks
  that the start-up bytes cross it in the same order a local port gets
  them, that two back-to-back presses arrive as two events, and that a
  hang-up fails the very next frame

## 5. Against real tmux

`tests/relocate.rs`, on a scratch `-L` socket with an explicit config, under
**four server configurations** — because `renumber-windows` is read from the
user's `~/.tmux.conf` for every server tmux starts, and assuming otherwise
is how a measurement became a config
([D10](10-decisions.md#d10--a-measurement-that-was-a-config)).

It was argued down once as *"a harness this repo does not have and this bug
does not justify"*, and that was true of one bug. There have since been four
in this subsystem whose evidence is a tmux measurement. The cost was also
lower than that paragraph assumed: `Tmux::with_anchor` was already `pub`,
`TALLY_TMUX_SOCKET` already redirected the socket, and the fake `anchor` is
six lines of `sh` passed in by path rather than put on `PATH`.

What it covers: a corpse claiming **another** address no longer wedges the
pad; a live view squatting on a pad is parked above the column range, the
pad works, and the squatter is still alive; a parked view still comes home
when its own pad is pressed; and `audit` reports a drifted view and is
silent about one in place.

The test that pinned the old live-occupant behaviour was **rewritten rather
than deleted**. It was written to fail the day the swap landed, and it did.

**Mutation is used where the assertion could be vacuous.** Emptying the
squatter-clearing path fails three of the tmux tests and **none** of the 386
unit tests, which is the measurement that says those three are doing the
work and the unit suite could not have.

**Not covered, and said rather than hidden:** the branch where a relocation
fails for an unforeseen reason. Every row of the plan table now succeeds
against real tmux, so there is no longer a *known* way to make one fail from
outside, and a test that forced one would be testing a tmux error nobody can
name. The guard stays as defence against the unforeseen, which is exactly
what it was reached for. Also not run: with a client actually attached to
the swapped-out view. The suite is headless, and what a client does on a
swap is tmux's documented behaviour rather than ours — the one claim in that
subsystem taken from the manual instead of from a measurement.

## 6. The whole thing — by hand, on a real device

There is no way around this, and pretending otherwise is how a surface ships
with its rows upside down. A written checklist, run before each milestone:

```text
[ ] plug in; tallyd finds the port by substring, confirms model via device inquiry
[ ] every pad pressed lands on the slot directly under the finger
[ ] bottom-left pad is slot (1,1), not (8,1)
[ ] ▲▼ slide the window one row; ▲ held past 500ms lands on row 1
[ ] ◄ returns you; ► goes to the longest-waiting amber and scrolls to it
[ ] hold Session: nothing happens, on the grid or anywhere else (#41)
[ ] hold CC19: alive pads red, pads hiding ended rows dark red, the rest dim
[ ] double-tap CC19: the corner goes turquoise, and the next hold previews
    Move rather than Stop
[ ] release any modifier without tapping -- nothing happens, every time
[ ] chord ordering: tap a slot, then grab CC19 -- nothing is killed
[ ] two modifiers down: nothing fires, and the preview disappearing says so
[ ] tap Drums: the grid is an art, pads are swallowed, ► is the way out and
    still goes to whatever is waiting
[ ] a jump leaves your personal tmux server alone -- list-sessions unchanged
[ ] kill a session you are watching: still attached, corpse readable, and
    the dead line names the conversation
[ ] move a session: its view follows, and the pad it left can be spawned onto
[ ] unplug mid-run: no crash; replug: full repaint, correct state
[ ] SIGTERM tallyd: device returns to Live mode, Setup menu works again
[ ] SIGKILL tallyd, restart: device recovers to the correct grid
[ ] start tallyd: logo goes green and stays green
[ ] stop anchord: logo goes dark red, grid does not clear
[ ] SIGTERM tallyd: logo goes dark — the only thing that means *not running*
[ ] scroll a waiting session off the window: the logo does **not** change;
    `►` is what pulses
[ ] with no tmux server at all: tallyd starts, grid is correct, a jump
    creates the server
```

**Press `►` before testing a pad.** While an art is playing every pad press
is swallowed, and a swallowed press looks exactly like a broken jump — two
sessions have already lost time to it.

### The jump subsystem's own gate, and what is still owed on it

M18's exit criteria were run against the live grid on 2026-10-18, and
**four of them did not run and two failed.** The restructure deleted the
checklist along with the document that held it, which is its own small
lesson: a gate nobody can read is a gate nobody runs. It is restored here
because it is the last open item in
[F1](../ROADMAP.md#f1--close-the-jump-subsystem-).

Run on the **live grid**, not a scratch socket — the distinction is the
whole point, and three of these had only ever seen one:

```text
[✅] a move within a row leaves the view at the window its label names
[✅] a move to another row that already exists (1:3:1 -> 1:2:1)
[✅] no `index N in use` caused by any of those moves
[⬜] a move to a row that does NOT exist -- the placeholder-session
     branch, the riskiest thing in the change, because tmux cannot adopt
     a pane into a session it creates
[⬜] a move onto a corpse, on purpose
[⬜] a spawn onto a pad a move vacated, on the live grid
[❌] a press of a pad whose window a drifted view holds: `index 2 in use`
     twice, then healed by pressing the view that was out of place
[❌] a move onto a live pane -- not tried; it happened anyway, and the
     view was re-tagged where it stood
```

**Both ❌ have had their cause removed and neither has been re-run.** They
were caused by `renumber-windows on` leaking in from the owner's
`~/.tmux.conf` — tally's server is a separate *server* but not a separate
*config* — which is now pinned
([D10](10-decisions.md#d10--a-measurement-that-was-a-config)), and by the
live-occupant branch, which now swaps
([D19](10-decisions.md#d19--a-live-pane-at-a-views-destination-was-left-alone)).
**A fix that has never met the case it fixes is a fix in one of the two
senses only.**

## What a green suite does not prove

Borrowed wholesale from anchor's testing doc, because it applies here
harder. [10-decisions.md](10-decisions.md) is the running list of times it
has been paid for; this is what the entries have in common.

- **That the colours are legible.** Nothing automated will tell you that
  amber-flashing and yellow-static look the same at 3 metres under a desk
  lamp. Only looking will. Three greens were perfectly legible and nobody
  could remember which was which; a white current page beside turquoise
  occupied ones failed the same way.
- **That the gestures are learnable.** A chord that tests fine and that you
  cannot perform without looking at the device is a failed design.
- **That a correct surface is a *bearable* one.** This is the one looking
  does not catch either, because it takes a **day**. The hard amber flash
  read as urgent on hardware and as stressful by the afternoon; the green
  working pulse was legible and tiring; a scrolling title was byte-perfect
  and unreadable at eight pads wide. Each was built, confirmed on the
  device, and then deleted by use.
- **That the modules run in the right order.** The frame is bytes and the
  bytes are tested; *the order the modules run in* has twice been wrong here
  and is still the half no unit test holds. Naming the loop's state turns
  two of those orderings into methods with one caller, which is an
  improvement and not a sufficiency.
- **That the attention signal is *right*.** Precision and recall of
  `NeedsAnswer` against real sessions is the actual quality metric of this
  program, and it is measurable only by using it and counting the times it
  was wrong. Keep that count **in a file**, not in your head:
  [`attention.jsonl`](04-signals.md#the-instrument) is one line per
  transition, which turns the gate from a feeling into a division.

The list has a shape, and naming it is why it is kept: **a surface fact only
shows up on the surface, and a surface you sit beside all day has a
constraint a surface you glance at does not.** Everything in it was green,
correct, and wrong.

## The honest gap today

**A week of `attention.jsonl`, and a number out of it.** It is the
criterion the first milestone shipped with, the one the hooks made
possible by giving the signal a source that fires, and the only thing on
the roadmap that no amount of building can close.

**It is running, and it was not running before 2026-10-19.** Reading 55
hours of the journal found something worse than being short: **it did not
log a plain pad jump**, so the answers to the ambers it recorded were
mostly absent, and the gate could not be computed from it at any length.
That was five lines ([#43](https://github.com/alehatsman/tally/issues/43),
shipped), and the window runs from the build that carries them — **not**
from the 2.3 days that preceded it, which can answer the questions about
navigation and nothing about the amber. It closed on 2026-10-19 with 126
rises in 17.3 hours: **93 attended, 33 aged out** (ROADMAP, F0).

**That belongs in this file rather than only in the roadmap**, because it
is the same failure as everything in the section above, one level up: the
suite was green, the journal was correct, every line it wrote was true —
and the instrument could not answer the question it was built for. **A
measurement that records the stimulus and not the response is not a
measurement**, and nothing about running it longer reveals that.

**Three of the five questions the journal was given are already closed**
by the 2.3 days, because they were about navigation rather than about the
amber: the queue's depth, whether `►` is reached for, and whether the
overview earned its address space. The last of those closed it
([05](05-interaction.md#the-overview-cut)). What the week is for is the
two numbers that are left — **how many ambers you went to, and how many
aged out** — and `jumped_pad` is what makes the first of them exist.

Everything else that was ever open here is not. The first draft of this file
ended *"nothing in the surface document has been executed against hardware"*
— that was true for about a day. `03-surface.md` now carries ✅ and ❌
against each claim, which is the form this section should take when the week
finally runs.
