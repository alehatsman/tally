# 10 — The record

**Everything here was built, shipped, confirmed, and then changed by
evidence.** The rest of the documents state current truth only; this one is
why they say it.

It is kept on purpose, and the shape of the list is the reason: **a surface
fact only shows up on the surface, and a surface you sit beside all day has
a constraint a surface you glance at does not.** Most of what follows was
green, correct and wrong — tested, verified on hardware, and deleted by use.
None of it could have been argued out in advance.

Two instruments do almost all the work here, and they are different: **a
glance at real hardware**, which catches legibility, and **a day beside
it**, which catches everything a glance cannot. Two entries passed the first
and failed the second.

---

## Perceptual — only looking could settle these

### D1 — Three greens

**Built:** eight states, with separate colours for *working*, *quiet* and
*exited clean* — green 21, green 21 pulsing, and dark green 23.

**Found:** perfectly legible side by side on real hardware, and **nobody
could remember which was which**. 21 and 23 are distinguishable; that was
never the problem.

**Now:** one green, static, for anything alive. Hue answers the only
question worth asking first — *can I talk to it, or must I restart it?* — and
nothing is spent on a finer distinction inside that answer.

**The rule it produced:** *a colour language is a thing you recall, not a
thing you resolve.* That is stricter than legibility, and it is the sentence
that has closed the most proposals since. 23 is still in the palette, marked
unused, because what the device does with it is a fact that cost a device
and an afternoon.

→ [05](05-interaction.md#why-the-colour-language-is-one-sentence)

### D26 — The health light answered two questions

**Built:** `CC99` as a three-state light — dark when healthy, dark red for
an unreachable `anchord`, amber pulsing when a session *off the visible
window* needed an answer. Unreachable outranked amber. Both meanings were
argued for: health is visible from further away than `►`, so the logo
carried the half of attention the grid could not show.

**Found:** reported from the desk as a **bug**. The light sits dark all day,
then scrolling the window lights it — and the only honest reading of that is
*the scroll button leaked into the logo*. Nothing leaked; the second meaning
was simply unguessable, because the light's other state is a fault and the
eye files any change on it under *something is wrong*.

The dark idle was the other half. **Dark is also what a dead `tallyd` leaves
behind** — shutdown clears all 81 LEDs
([08](08-daemon.md#shutdown-is-not-optional)) — so *working* and *not
running at all* were the same picture, and the one question the health light
exists to answer could not be answered by looking at it.

**Now:** health only, and lit at rest. Green 21 while the stack is up, dark
red 7 for an unreachable `anchord`, off for *no daemon*. The amber went to
`►`, which already pulsed for the same fact and is the thing you press.

**The rule it produced:** *a light whose other state is a fault cannot be
lent out.* Whatever else it says will be read as alarm. And the corollary
that cost more: **off is a state, not a default** — a light that rests dark
is indistinguishable from a light nobody is driving.

**The cost, named:** while an art owns the grid, off-window attention is
reported by `►` alone, where it used to be on two lights. One light, and it
is the actionable one.

→ [05](05-interaction.md#the-logo-is-the-health-light)

### D2 — The hard amber flash

**Built:** attention as a 50% duty-cycle flash, channel 2. Checked on
hardware: reads as urgent from across the room. Correct.

**Found:** **stressful after a few hours.** A different measurement, and one
that only time beside it can take.

**Now:** a two-beat pulse. Re-checked both ways, because calmer is only an
improvement if it still works: calmer, and still catches the eye.

**The rule it produced:** *what reads well at a glance can wear badly by the
afternoon.* This is the first of the two entries that passed a glance and
failed a day.

→ [05](05-interaction.md#nothing-blinks-and-only-attention-moves)

### D3 — The title scroll

**Built:** the full OSC-title scroll — the SysEx message, the speed, the
ASCII sanitiser, the byte budget, the tests. Everything worked. The scroll
overlays without destroying the lighting underneath and leaves the top row
and logo alone, exactly as the manual says.

**Found:** held down once on real hardware. **Eight pads wide is about two
legible characters at a time**, and a title read through a two-character
moving window is guessing, not reading. The verdict from the desk: *"it is
impossible to read anything."*

**Now:** deleted, the same hour. There is no smaller version that works —
slower shows the same two characters for longer, and a shorter title still
arrives two at a time. The constraint is the width of the grid, and the grid
is the product.

**What survives:** the protocol stays documented, because the device does do
this and the next person will want to try it. `STOP_SCROLL` stays in the
code, because it is how a device left mid-scroll by anything else is put
back.

**The rule it produced:** the byte budget was fine, the sanitiser was
correct, the tests were green, and **the feature was worthless**. The pad
says *that* something wants you; *what* it wants is in the window, one press
away, rendered by a terminal that has eighty columns and a font.

→ [03](03-surface.md#text-scrolling), [01](01-scope.md#how-to-reject-a-feature)

### D4 — The green working pulse

**Built:** green pulsing for *producing output*, green static for *alive and
quiet*. Legible at a glance, and it says something true.

**Found:** two problems, both only visible over a day.

Five agents working put five pads in motion — **the surface busiest exactly
when nothing needed anything**. And because the tempo is one global clock, a
single pad waiting fifteen minutes drove every unrelated green pulse to
140 bpm, where a breath cycle is 0.86 s. Time-weighted over 10.1 h: 61% at
the calm floor, **19% in the 120–140 band**. Reported from the desk as *"the
constant breathing of the buttons does make me tired."*

**Now:** green does not animate at all. **Motion is spent only on amber.**

**What this corrected was an argument, not a number.** The tempo spec
defended exactly this behaviour: *"a lone eleven-minute amber speeds up the
green pulse of four agents that are working fine. That is correct — the room
is not calm."* The argument is not wrong about the *room*. It is wrong about
**where to say it**: one fact was being painted onto five pads, four of
which could do nothing about it. The fix was not a per-pad clock — there
must not be one — but to stop green from moving. The aggregate mood
survives, because the amber *is* the room's mood.

**The second entry that passed a glance and failed a day.**

→ [05](05-interaction.md#nothing-blinks-and-only-attention-moves),
[05](05-interaction.md#the-tempo-says-how-long)

### D11 — Two bright colours on one row

**Built:** the top row as eight page buttons, lighting the current page
white and occupied pages turquoise.

**Found:** *"I am confused which one is selected, the white one or the
turquoise one."* The three-greens failure in a different place.

**Now:** **brightness answers "where am I" before hue answers anything.**
One bright thing, some dim things, and the dim ones say *there is something
there*. Hue on the control rows is spent only on the thing that is urgent.

→ [05](05-interaction.md#the-layout)

### D12 — The Uprising reached its punchline at 17.4 seconds

**Built:** the joke piece, ending on a yellow smile. Every test green, the
renderer drawing the face correctly at frame 759.

**Found:** the first person to press the button **never saw it**, because
nobody holds a button they have just discovered for eighteen seconds.

**Now:** the face arrives at ten seconds, and the pacing is asserted in
**seconds, not frames** — because seconds are the unit the complaint was
made in.

→ [05](05-interaction.md#the-nine-pieces)

---

## The signal — the one the product is a bet on

### D5 — The bell that is never rung

**Built:** BEL detection, with the OSC state machine running *before* the
bell check so that a title terminator is not mistaken for a ring. A full
test suite: bell-then-silence, bell-then-output, bell-with-trailing-bytes,
bell-then-`\r\n`, bell-then-exit. All green.

**Found:** 1012 `0x07` bytes across 6.6 MB of one real session. **Every one
an OSC title terminator. Zero in GROUND state.** Same across four sessions,
including plain `claude` with no wrapper.

So the derived attention signal — the thing the entire product is a bet on —
**does not fire at all** against the agent this grid actually runs.

**This is not a tuning problem, and that distinction is the finding.** The
milestone that named it said *"if the attention signal is not trustworthy,
stop — the rest of the program is a worse fuzzy finder."* It did not come
back untrustworthy. It came back **absent**, which has a different answer.

**Now:** attention comes from hooks the agent calls, on both edges. The
heuristic is still the floor and still always runs; against Claude Code that
floor is zero.

**The code is correct and is why we can say *zero* rather than guess** — the
scanner rejected all 1012, where a naive `bytes.contains(&0x07)` would have
reported 1012 phantom rings. **A working detector with nothing to detect
looks exactly like a working feature.**

**What did not have to change:** `Source { Derived | Declared }` existed in
the enum from the first draft, so promoting the hook from *later,
unscheduled* to the only working source was a match arm and not a rewrite.

→ [04](04-signals.md#1-bel-0x07--it-wants-you), [09](09-testing.md#2-the-signals--a-fake-anchor)

### D13 — `$ANCHOR_SESSION` does not exist

**Specified:** the attention hook as `anchor label $ANCHOR_SESSION
tl.attention=1`.

**Found:** that variable is set by nothing. It appears nowhere in anchor's
source and is not in a live session's environment. The recipe expanded to an
empty argument and **had never worked**.

**Now:** `tally attention on|off`, which walks its own process ancestry to a
pid anchord recorded. The job is the lookup, not the label — and no lookup
by cwd, because two sessions in one repo is the normal case here. (The
rising word has since split by hook: `blocked` or `done`, with `on` kept —
[D28](#d28--one-bit-two-states-and-the-word-that-split).)

→ [07](07-cli.md#tally-attention-onoff)

### D14 — Nothing cleared the label

**Specified:** *"the clearing write is tallyd's, not the hook's — the hook
fires when the agent starts waiting and gets no second callback when you
answer."*

**Found:** it was never built. `LABEL_ATTENTION` appeared three times: the
constant, a reader, and a test. **Nothing removed it, anywhere.** A hook
installed against that binary would have stuck a pad amber permanently.

**Now:** both edges are hooks. Not because nothing else could work —
output-clearing *would* work under plain `claude` — but because it breaks
under exactly the wrapper the grid is driven through: `isayes` repaints
twice a second and would clear the label within half a second of the hook
setting it. **A signal with two writers disagreeing at 2 Hz is worse than
one writer that can be absent**, and a clearing rule that depends on which
binary you launched is a rule you cannot reason about from across the room.

→ [04](04-signals.md#clearing-attention)

### D15 — The journal was miscounting, and it was restarts

**Found:** three `needs_answer` lines against one `answered` for one slot.
Read at first as oscillation with a lost falling edge. It was neither:
`Journal::asking` was an in-memory map rebuilt empty on every start, so a
slot still asking when tallyd restarted had no record of having asked. The
evidence was in the file — `after_ms` measured from the **third** rise
exactly, and the earliest line missing a field later lines carried.

**Now:** the tail is read back at startup, restoring the map with its
original timestamps.

**It contradicted a sentence that had been written deliberately** — *"the
only piece of state tallyd writes that is not rebuildable, which is
acceptable precisely because nothing reads it back"* — and the contradiction
is the point: the file is the instrument the whole product is gated on, so
**it has to be right more than it has to be write-only.**

**And it built the queue by accident.** A map from slot to the moment it
began asking, surviving a restart, is exactly what `►` sorts on. No new
label, no new file, no new state in the daemon.

→ [04](04-signals.md#the-instrument), [05](05-interaction.md#the-queue-and-the-two-buttons-that-read-it)

### D28 — One bit, two states, and the word that split

**Built:** both rising hooks wrote `"1"`. Presence raised the amber, and
that was all anything needed — for a human at the grid, *blocked on a
dialog* and *turn finished* both mean *go look*.

**Found:** `tally ls --json` gave the grid its first readers that are
programs ([#23](https://github.com/alehatsman/tally/issues/23)), and the
two readers want opposite things from one bit. A human wants **recall**
and a wrong answer costs a glance; a program wants **precision**, to act
only on *blocked*, and a wrong answer is a wrong action. The bit could not
tell them apart, because the two hooks had already collapsed into one
string by the time anything saw them.

**Now:** the bit is untouched and the **value** carries the origin.
`Notification` writes `blocked`, `Stop` writes `finished`, and a hook that
did not say writes `1`; `tally ls --json` reports it as `wants`. `state`
still says `wants you` for all three, because the sentence is for a human
and a human wants recall. **`on` is kept and is not an alias**: it is the
third answer, and every label written before the split carries it, so a
`settings.json` nobody updated keeps raising ambers.

**What it bought:** [#13](https://github.com/alehatsman/tally/issues/13)
needed a third origin on a bit that carried two meanings in one string. It
can be a fourth word now.

**The rule it produced:** *fix a conflated signal at the word, not the
bit* — the amber, `SlotState` and every renderer stayed as they were, and
the one place that changed is the one that was lying.

→ [04](04-signals.md#one-bit-two-states--and-the-word-that-split),
[07](07-cli.md#what-this-output-promises)

---

## The environment — things that were not what the document said

### D6 — Eight panes in a row

**Built:** a row of eight agents as eight panes in one tmux window.
Correct, addressable, tested.

**Found:** on a real screen it was **eight 23-column slivers** reflowing
megabytes of scrollback, which nobody could read.

**Now:** a column is a **window** — a tab — each the full width of the
terminal. A split shows you all eight at once; the thing you do with a row
is read **one** of them.

→ [06](06-views.md#a-row-is-a-session-a-column-is-a-window)

### D7 — Pages as places you go

**Built:** eight pages on the eight top-row buttons, switching page by
cutting to a different screenful.

**Found:** **pages used, 70 : 2.** Page 2 had been visited twice, ever.
Eight dedicated buttons for a thing that was not happening — and a page
boundary is not something you can feel, where a row is.

**Now:** eight pages of eight rows stack into one sixty-four-row space, and
the grid is an eight-row window that `▲▼` slide **one row at a time**.
Addresses are unchanged — `work:3:5` is still `work:3:5` — but a page
boundary became an arithmetic detail.

**What it cost, stated:** there is no current-page readout any more, which
deletes the rule D11 was paid for. Defensible on 70 : 2 and not on much
else. **If pages become load-bearing later, this is the first decision to
revisit**, and the reason it was made is written down so it can be.

**What replaced the thing pages were protecting:** the original argument —
*switching away from a page means going blind to it* — is still true, and
the fix is no longer a per-page indicator. It is `►`. **You no longer
navigate to attention; attention navigates to you.**

→ [02](02-model.md#pages-are-a-naming-scheme-not-a-place-you-go)

### D8 — `slots.toml` as the spawn path

**Built:** spawning from a dark pad, resolving a spec out of a config file.

**Found:** unused, and then — more usefully — **unreachable**. There is no
`slots.toml` on this machine (`tallyd` loads *8 pages, 0 slots*), because
**cwd is the one fact a config file cannot know in advance and a shell
always knows**. A file that wants the folder declared up front is a
prediction about work you have not done.

That also made the respawn chord dead on every pad: it required a resolvable
spec, so its preview could never light and its press could never fire. Zero
presses in a day, and it *could not* have had any.

**Now:** `tally spawn` is the main path, taking its directory from `$PWD`,
and the `>` column is the other, taking it from the row.

**Since (#40):** the spawn half of the file is **cut**, not merely unused.
`[[slot]]`, `[[spec]]`, the `tl.spec` label and the dark-pad press are all
gone, and `slots.toml` is one table — page names. Leaving a measured-dead
path in place had already cost something: `Config::by_spec_name`'s doc
comment still described the respawn chord this entry deleted, so the code
outlived its own consumer without anyone noticing.

**And the record anchor keeps turned out to be the answer** — `cwd`,
`command`, `login_shell`, on dead rows too — which went to `>` rather than
to respawn. The grid knows the directory now, for any row that has ever held
a session.

**The deferral was worth it.** The CLI was deferred with *"defer until
something is actually awkward"*. Had it been built early it would have been
a wrapper around guesses; built after the config path shipped and went
unused, it was built around a measured gap.

→ [07](07-cli.md#tally-spawn--the-folder-is-the-problem),
[05](05-interaction.md#the-right-column-launches)

### D16 — The port is not exclusive

**Documented:** *"Exclusive access. One process owns the port. If Ableton
has it, we do not."*

**Found:** ❌ false on macOS. CoreMIDI multiplexes. Two processes opened the
same endpoint and both drove the device, and **neither was told.**

**Now:** `tallyd` takes its own `flock`, before the port is opened. The
daemon is still unavoidable — but for the ordinary reason, *something has to
hold the state*, rather than because the OS enforces it. A different app
holding the device is not excluded either, and that is a thing to notice and
quit rather than a thing to detect.

→ [03](03-surface.md#the-port-is-not-exclusive-and-we-assumed-it-was)

### D17 — A failed write is not how you learn the device went away

**Assumed:** the send would error once the Launchpad was unplugged.

**Found:** ❌ it does not. CoreMIDI accepts sends to a removed endpoint
without complaint, so `render` kept returning `Ok` into a void while the
host went on diffing against a frame nothing had. Observed: the Launchpad
was unplugged and replugged, came back dark, and `tallyd` never noticed.

**Now:** the only signal is **absence of a reply**. A mode readback every
two seconds costs ~2 ms; three missed probes means the device is gone. The
reply doubles as a mode check, which matters because **a replugged Launchpad
comes back in Live mode** — a host that kept its connection has to notice
and re-enter programmer mode, or the grid stays dark forever with nothing in
the model having changed.

→ [03](03-surface.md#what-the-hardware-cannot-do)

### D10 — A measurement that was a config

**Measured and written up as ✅:** *"indexes do not renumber when a sibling
is killed."*

**Found:** false in general. It held on the machine it was measured on
because that tmux had `renumber-windows` at its default, `off`. The owner
sets it `on`. **A named socket is a separate *server*, not a separate
*config*** — tmux reads `~/.tmux.conf` for every server it starts, so tally
inherited whatever the user set.

The consequence was not cosmetic: closing any tab pulls every sibling to its
right down one index, so a whole row of views lands at the wrong address
**by tmux's doing, with no Move involved** — which turned a rare,
self-inflicted state into a generator, and fed the one relocation case that
was knowingly left broken.

**Now:** tally sets `renumber-windows off` on its own server, at every point
where the server could be wrong and nothing is yet at risk. **tally does not
rely on the user's config for any property the address scheme needs.** The
user's `~/.tmux.conf` is untouched: that file is theirs, and tally holding
its own invariant is the point.

**The rule it produced:** a ✅ that was never re-run under a different
environment is a claim about one machine. This is also the entry that bought
`tests/relocate.rs` — a fixture listing cannot see a config option.

→ [06](06-views.md#tally-pins-renumber-windows-off)

### D18 — anchor's label write dropped the whole map

**Documented:** concurrent writes are safe, because `label` takes `set` and
`remove` per key rather than replacing the map.

**Found:** true of the *interface* and false of the implementation. Nothing
held a lock across the read and the write, so two clients could interleave —
and the loser did not lose its own key, it lost **the entire map**, which is
why namespacing could not help.

**Now:** fixed in anchor `e1503c9` (#12): one guard across read, edit, write
*and persist*. The persist being inside the guard is the part worth knowing,
because the obvious half-fix looks right and is not.

**And the follow-on correction is the more useful one.** A later revision
said *"the guarantee is real"* on the strength of having re-pinned tally to
that commit. That is a statement about what tally *builds against*. The
running `anchord` was built from an older commit. **Check the stamp, not the
checkout** — *the source has it* and *the thing answering my socket has it*
are different facts.

→ [01](01-scope.md#teleport-is-a-sibling-not-a-dependency)

### D27 — The bridge, specified on a Mac and answered on the desk

**Specified** (`docs/30-the-bridge.md`, deleted in `17a5b23`): `tallyd`
listens and the bridge connects, because WSL forwards Windows'
`localhost` into the VM and not the reverse; `Device` becomes a trait
with a `Remote` implementation and `poll` changes signature; the raise is
`powershell.exe` `AppActivate`, unverified.

**Found**, 2026-10-19 and 20, by running each claim on the Windows desk:
under mirrored networking loopback reaches **both ways**, so the reason
for the direction was gone; `AppActivate` from WSL **returns `True` and
raises nothing**, three times of three; and a systemd unit — the obvious
way to run `tallyd` in WSL — launches Windows programs that **exit 0 and
never reach the desktop**.

**Now:** the bridge listens, because then a bridge that is not running is
a refused connect, which is the `NotFound` `Link` already retries on the
reconcile clock — and a stranger on the socket can reach the device but
never the daemon. `Device` stays **one type** with two ways of moving
bytes, which is all the second implementation ever was; `poll` did not
change and nothing above `Link` can tell. The raise presses Alt first and
lands three times in four. `tallyd` on that desk runs under a `wsl.exe`
started at logon. **The cost**, named: it was finished before the device
was plugged in, so the device is a checklist. **Run 2026-10-20**: steps
1–6 passed on a real Mini MK3, and what they found was the desk rather
than the design — a bus-powered hub (*Code 10*, no port at all) and
Claude hooks never deployed in WSL (no amber). The kills, the unplug and
a day beside it are still owed.

→ [13](13-bridge.md)

---

## Architecture — reversals with the cost named

### D9 — Two correct modules in the wrong order

Not one bug. **The shape of every hard bug this project has had**, and it is
named here because a suite that tests each module alone cannot see it.

| | what happened |
|---|---|
| **the mode toggle** | `tallyd` flipped the corner's verb before `chord.rs` saw the release, so the press went down under one verb and came up under another. A release matched by *verb* stranded the old one in `held` forever: the whole grid sat in a held preview with no sequence of presses able to clear it. **The toggle lives in `tallyd`, the stranding happened in `chord.rs`, and the bug was in neither.** Fixed by matching a release on the **button**, which makes the pairing true by construction |
| **`CC19` behind a plasma field** | art-mode presses were routed on the *gesture* rather than the raw button, and `feed` answers `None` for a held modifier and for the release half of a double-tap — so `CC19` could still reach the double-tap clock and silently toggle Stop/Move behind an art, with nothing on the surface able to report it. Discovered only on coming out |
| **`▲` and the art** | during a refactor, collapsing two methods into one would have moved the hold-`▲` handler to the far side of the art, silently changing what the surface does under a no-behaviour-change claim. Caught by reading the code, not by a test |

**The structural answer:** the 425-line loop body holding 23 loose `let mut`
was the shape that produces these and cannot test them. Naming four clusters
— `Link`, `Beat`, `Waiting`, `Grip` — turns two of those orderings into
methods with one caller. **That is an improvement and not a sufficiency**,
and the document that made the change says so.

→ [08](08-daemon.md#what-the-loop-holds), [09](09-testing.md#3-the-gestures--pure-and-the-reason-they-are)

### D19 — A live pane at a view's destination was left alone

**Shipped knowingly broken.** When a moved session's destination window was
held by a **live** pane, the relocation returned *stay* — and the caller
then tagged the pane where it stood. Two writes that disagree, producing the
exact state the subsystem exists to prevent: **a pane whose `@tally_slot`
names an address other than its window.**

**Deferred for a good reason and a stale one.** The good one stands: a live
`anchor attach` is something somebody may be reading, and a relocation is
not the gesture that closes it. The stale one was *"it can only be there if
an earlier relocation was skipped or failed"* — rare and self-inflicted.
D10 turned that into a generator.

**Now:** the two views **swap**. One `swap-window`, atomic, nothing created
and nothing destroyed — and the original rule, *nothing live is closed*, is
kept better by a swap than it was by a refusal, because the refusal was
followed by a tag that lied.

**The invariant it buys is narrow, and the obvious wider version is false.**
*The view being relocated always ends at the address it claims.* It is not
true that no pad is left wedged: the swap chooses **which** view is out of
place, always in favour of the one just moved, so a wedge can travel one
address sideways rather than disappear.

**And the second half, worth doing even if the swap is replaced:** when a
relocation does not succeed, `@tally_slot` **is not written**, and the
refusal is logged. The pane keeps whatever address it last legitimately
held, and the drift becomes visible instead of being papered over.

→ [06](06-views.md#when-the-destination-is-live)

### D25 — A corpse was only found at the address it claimed

**Shipped:** a dead pane holding a column's index is reclaimed and respawned
onto the new session, which fixed the collision everybody hit.

**Found:** that lookup finds a corpse **by the address it claims**, so a
corpse claiming a *different* address was invisible to it and wedged the pad
forever. The original fix named one case of the blind spot and did not
generalise it.

**Now:** a corpse at the index is replaced whatever it claims, and a **live**
squatter is **parked** above the addressable column range rather than sent
home — because a tag is an address, page names are config, and `jump` must
not be handed the page table to fix a collision. **The squatter does not
have to go home; it only has to get out of the way.**

**And the instrument came with it, deliberately short of the obvious
feature.** A sweep counts drifted views and moves nothing, because there is
no number for how often a view drifts and **a sweep that repaired silently
would answer the question by erasing it.**

→ [06](06-views.md#drift-that-nothing-was-looking-for)

### D30 — The sweep answered *where* and the panes had died in place

**Shipped:** a sweep that compares every view's address with the window it
sits in, so drift stops being invisible — [D25](#d25--a-corpse-was-only-found-at-the-address-it-claimed)'s
instrument.

**Found 2026-10-20:** anchord dropped its connections without restarting,
and **seven view panes died while their sessions stayed alive — each at the
window its own address names.** `relocation()` answers *where a pane should
go* and returns `None` for one already in place, **dead or alive**, so the
sweep reported zero drift on a grid where seven pads cost a wasted press.
Sharing the move path's pure function was the right call and is still the
right call; it was answering a different question from the one the sweep
needed.

**Now:** two faults, not one. `drifted` is still `relocation()`. `dead view`
is *right window, dead pane, live session* — and that third clause is the
one that matters: a corpse at its own address under a session that has also
exited is what **finished** looks like, and counting it would bury the
signal under the ordinary end of every session. **The predicate is liveness
and address, not address alone.**

**Liveness is the daemon's opinion.** `jump` cannot ask anchord anything, so
the daemon hands it `alive` per session, the same way it already hands over
the `Place` it cannot derive without the page table. One boundary, now
carrying two facts.

**And the instrument's reach was the second half of the finding.** The sweep
ran **once per daemon lifetime**, with no time on the line and no build id
in the log. So drift that healed lazily between restarts was never sampled,
two sweep lines could not be ordered against anything, and an old collision
could not be attributed to a build — which is exactly why 77 `in use` lines
were refused as evidence in `ROADMAP.md`. It now runs on every reconnect —
**the event that killed the seven panes** — and every 15 minutes, stamps
`ms=` in the journal's unit, and the daemon's first line names the commit.

**What this cost to learn:** the counting sweep was built to decide whether
the acting sweep is worth its blast radius, and a week of its output would
have read *zero drift* while the case that motivated the question was
happening. **An instrument that samples once, unstamped, is not weak
evidence — it is evidence of the wrong thing.**

→ [06](06-views.md#the-sweep-counts-and-does-not-touch),
[#22](https://github.com/alehatsman/tally/issues/22)

### D20 — Identity at window scope

**Built:** `@tally_session` as a window option.

**Found:** a window option is inherited by **every pane in the window**, so
splitting a view gives two panes claiming one session and a jump lands on
either. Measured on 3.6b. The same bug one level up had already been found
once — a *session* option inherited by every window.

**Now:** pane scope, everywhere, for identity, address, `remain-on-exit` and
the dead-pane format. A pane is also the unit that can outlive its command
and the unit a return gesture can name.

**Migration was made self-healing deliberately:** a view tagged the old way
still matches, because a one-pane window resolves to its one pane — but the
inherited tag *is* the ambiguity, so a lookup re-tags at pane scope and
clears the window option. One extra command on the first jump per old view,
and the ambiguity is gone for good rather than lying dormant until someone
splits a window.

→ [06](06-views.md#the-problem-and-the-unit)

### D21 — A view is not a valid origin

**Shipped:** `◄` records where you were standing, *"and its current pane is
not a view pane"*.

**Found:** that assumed you jump *from* nvim. **Every pane this grid is used
from is an anchor session**, so every candidate origin was a view, nothing
was ever recorded, and `◄` was dark forever.

**Caught within an hour of shipping**, by the `kind:"nav"` lines added for
exactly that purpose: two `{"via":"back","ok":false}` in a row, which is
what pressing a dead button twice looks like in a log. **The instrument
caught the thing it was built to catch, on its first day, about its own
author's spec.**

**Now:** a view is a valid origin. What the exclusion was protecting —
walking back through agents one at a time — is held by *first jump wins*
instead, which does not care what kind of pane you started in.

→ [05](05-interaction.md#back-and-where-you-were)

### D22 — A jump took your terminal away

**Shipped:** views on *your* tmux server, in a session called `tally`.

**Found:** one line — `for (client, _) in &clients` — where `clients` is
every client on the server. Press a pad while you are reading a diff in
`main`, and your personal client is yanked into a view.

**Now:** tally has its own socket. **The socket is the real boundary**, and
a session name bounded nothing: `list-clients` is server-scoped and was
always telling the truth. `switch-client` cannot reach a client it cannot
list.

**The rule it produced:** *match on the identity that survives the boundary,
never on a derived one.* It appears a second time in the same document — a
row session is named by its **absolute row**, not its page name, because
page names are config and renaming one must not orphan sixty-four panes.

→ [06](06-views.md#tally-has-its-own-tmux-server)

### D23 — Kill was blue

**Shipped:** the action buttons resting blue, and `Session` proposed to join
them so that *blue means hold me* would be true.

**Found:** the kill button went **red** the same evening — the colour of its
own consequence, and the colour the pads it will touch are about to be. That
retired blue from the palette and took the rule with it, before the ink on
the `Session` draft was dry.

**Now:** dark grey for *available, not urgent* on every navigation control;
red and turquoise on the corner, for the two verbs' consequences. Blue stays
retired, in the palette, marked unused.

→ [03](03-surface.md#colour)

### D24 — Kill and purge may not share a gesture

**Decided:** *"a merged kill/purge modifier widens the blast radius of the
single gesture we spend a section protecting."*

**Overturned deliberately, with the cost named rather than argued away**,
when the surface ran out of buttons. What makes it affordable: **the two
consequences can never share a pad** — alive and ended are disjoint sets, so
every pad under the thumb has exactly one outcome and exactly one colour.

**The cost, stated:** purge's targets include pads that render dark, so the
dark two-thirds of the grid stopped being a free miss. What pays for it is
that **the hold is what reveals them** — they stop being invisible at the
exact moment they become targets. A thumb one pad off forgets a dead row's
log, which is a real regression priced at one order below killing a live
agent, and it is why purge gets the dark-red *consequence* colour rather
than kill's red.

→ [05](05-interaction.md#stop-does-kill-and-purge)

### D29 — The overview was measured out

**Built:** M11. Holding `Session` turned the grid into one pad per
absolute row, so row 13 was one gesture and not five `▼` across four rows
of nothing. The argument was sparsity: seven of sixty-four rows had ever
held a session, already discontiguous.

**Found:** nine presses over 55 hours. `lit` — the count of non-empty rows,
logged on every press — was **4** every time and never moved, and all nine
destinations were rows 1–8, inside the window they started in
([#41](https://github.com/alehatsman/tally/issues/41)). The map **never
once crossed an empty middle**, the case its whole design argues from.

**Now:** the overview is gone, with its gesture, its journal line and its
renderer. `Session` is dark and deliberately given no replacement. `▲▼`
and hold-`▲` home stay — they are how the space is moved through, and they
are used. **The address space stays at 64 rows**: with the overview cut,
nothing was left for the size to pay for, an empty row costs one
comparison, and shrinking would migrate live `tl.slot` labels to reclaim
nothing.

**The rule it produced:** *a feature that carries its own instrument can be
judged on it.* The nine lines it wrote are what closed the question, and no
further evidence is coming — the instrument left with the feature.

→ [05](05-interaction.md#the-overview-cut),
[11](11-unbuilt.md#512-slots-for-four-used-rows)

---

## Closed proposals

Ideas that were specified in full and then did not ship, kept because each
closed for a reason that still applies.

### The squad

**Proposed:** a squad is a ledger file, a row is its face, column 1 is a
master agent you talk to, `User` + `>` is the gesture. Fully specified,
including a measured spike proving a master agent *can* type into its
members through tmux — which also corrected an earlier claim that
broadcasting to running agents was forbidden by the observer invariant. It
is not: the bytes go to a tmux pane, `tallyd` is not in the path, and
anchord grows no write path.

**Closed by `tally ls`** and the habit on top of it: an agent reads the grid
as text, sees which pads are free, and calls `tally spawn` itself. That is
the payoff — several agents working in a row, started without a hand on each
one — with **no new button, no ledger file, no master, and no protocol.**

Two of the squad spec's own arguments made the substitution fair, and both
were written before `ls` existed: its unit of work was already a *file*, not
a session; and it said outright that if the preview cannot be drawn in one
glance, *the chord is wrong and the shell keeps the verb*. The shell kept
the verb. It also did not fit the six bullets of scope, and it spent the
last free button. `ls` spends neither, because a listing is not a gesture.

**What survives as a live concern:** a program now reads the grid, which is
a reader the attention design was not written for
([#23](https://github.com/alehatsman/tally/issues/23)), and `tally ls`
output is an interface someone parses
([07](07-cli.md#what-this-output-promises)).

### A focused colour

**Proposed:** render the slot you are currently looking at in white.

**Cut, and refused twice over:** tallyd would have to poll tmux on every
tick to know, and white would overwrite *working* on the one pad whose state
matters most. **The window you are focused on is in front of your face; the
grid is for the sixty-three that are not.**

The return gesture needs none either: you are looking at the thing, so the
pad you are on is the pad whose view fills your terminal. The tmux lookup
happens **on the press**, never on the tick — which is what keeps the
objection answered.

### Output-clearing as a floor under the hook

**Rejected deliberately**, and it is the argument [the level](11-unbuilt.md#the-level)
has to clear. Under `isayes` it clears the label within about half a second
of the hook setting it, so the floor does not back the hook up — it
**fights** it, and silently.

### A cycling verb key with four modes

**Proposed:** Launch / Stop / Solo / Mute on one button, Ableton-style.

**Closed:** four mode axes where the surface wants none. The corner *does*
carry two verbs, and the difference is the whole argument: two fit on a
toggle, the grid does not change until you hold, and at that moment it
repaints entirely in the mode's colour before anything can fire. **Three
needs a cycle, a cycle needs you to remember the order, and the next idea
after that is a menu.**

### Peek

**Proposed:** *what is this pad?* — a real hole, since the surface cannot
name a pad and some share of 341 daily jumps are identification rather than
work. It was the one idea left that needed machinery the surface does not
have: a sink to print into.

**Closed by `tally ls`**, the same way the squad was, and it took a while
to notice. *What is this pad* is a listing with the cwd in it, and the only
gap left is answering it **without a terminal** — for which this device has
exactly one mechanism, the title scroll, which [D3](#d3--the-title-scroll)
proved unreadable at eight pads wide. **No mechanism, no feature.**

Worth recording as its own entry because it is the second time a listing
closed a gesture, which is a pattern rather than a coincidence: **the
surface is for the one bit you need from across the room, and anything that
needs a word needs a terminal.**

### Two narrower verbs instead of a listing

**Proposed:** `tally here` to print the caller's address, and `tally spawn
next` to claim the next free pad in its row.

**Both dropped.** An agent on a pad is not short a verb — **it is blind**,
and two narrower verbs answer two phrasings of one question. `tally spawn
next` would also have required amending the *no pad-picking* rule, which had
been argued once already; a listing does not touch it, because it shows the
grid and never assigns an address. **Dissolving the conflict beat resolving
it.** And `tally here` had no honest answer for "nowhere": a verb whose
entire output is an address must choose between a silent exit 0 and a
failure, where a listing simply prints the grid with no `@` on it.

### Cutting the arts

**Proposed:** [#42](https://github.com/alehatsman/tally/issues/42) — `src/art/`
was about 12% of the crate for a mode behind a button, and three of the nine
pieces already did both its jobs, *demo* and *all-64-pads pixel test*.

**Kept, and closed on taste.** It was always the one of the three scope
questions that was taste rather than architecture. The code is pure, has
its own tests, couples to nothing and is in nobody's way, and *three
pieces cover both jobs* is not a reason to delete something that works.
The per-piece findings — Life reseeds, Rain never skips a pad, Pacman's
rotating ghost — would have survived a deletion only if someone remembered
to keep the prose.

→ [05](05-interaction.md#the-nine-pieces),
[11](11-unbuilt.md#the-reel-is-nine-pieces)
