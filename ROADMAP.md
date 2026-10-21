# Roadmap

**What shipped, in the order it arrived, and what is left.** The design is
in [docs/](docs/) and states current truth; [the record](docs/10-decisions.md)
is what each thing cost to find. This file is the sequence, and
[the finish line](#the-finish-line) is the only part of it that is a plan.

Each milestone retires one risk and is independently abandonable, ordered on
a principle borrowed from anchor: **the risky subsystem first, the
ergonomics last.** The risk sat in an unusual place here — the hardware was
documented but unverified, and the *attention signal* is the thing the
product lives or dies by. The launcher was the easy half.

> **The sequencing rule — no milestone starts before the previous one's exit
> criteria are met — stopped holding at M10.** Everything since has shipped
> with open criteria, and one has been open since M1. That was affordable
> while the open ones were perceptual gates a day of use would close anyway.
> It stops being affordable at the gate below.

---

## What shipped

| | | retired |
|---|---|---|
| **M0** | Prove the manual | is the hardware what the PDF says — and it is, twice over: the two things that were wrong were both ours |
| **M1** | The grid tells the truth | the only real risk: is the signal trustworthy |
| **M2** | Jump | the payoff gesture |
| **M3** | Spawn and the destructive chords | the launcher half |
| **M4** | Pages | ergonomics |
| **M5** | `tally spawn` | the folder comes from a shell, not a file |
| **M6** | The amber exists | M1's risk, answered: the signal was **absent**, not weak |
| **M7** | Where views live | a view is a pane, not a window |
| **M8** | The top row is a queue | 61% of the time there is no *the* amber pad |
| **M9** | The tempo says how long | p50 187 s, p90 1076 s, and the pad looks the same at both |
| **M10** | The right column launches | the row already knows the folder |
| **M11** | The overview | 7 rows of 64, and already discontiguous |
| **M12** | The corner carries two verbs | there were no spare buttons left |
| **M13** | tally's own tmux server | a jump was taking your terminal away |
| **M14** | `tally fork`, and four at a time | the folder can come from a pad you can see |
| **M15** | The dead pane names the conversation | anchor's id resumes nothing |
| **M16** | The arts | a demo, and a pixel test of all 64 at once |
| **M17** | The bridge | **built and run on a Launchpad 2026-10-20** — four of the five questions answered on the Windows desk and one from the protocol, then checklist steps 1–6 passed on a real Mini MK3; the kills, the unplug and a day beside it are still owed ([13](docs/13-bridge.md#the-checklist-when-the-launchpad-is-plugged-in)) |
| **M18** | A moved session takes its view with it | the address is a tmux window, and Move only wrote the name |

**M6 is out of order on purpose.** It is M1's work, and it is here rather
than there because the thing that proved M1 incomplete was a day of real use
— the only instrument that could have found it, and the only one M1 had not
yet run.

**M12 through M16 are one night and a morning.** By M11 the risky subsystems
were all retired, so *the risky subsystem first* had done its job and
stopped constraining anything. What replaced it as the constraint was the
surface running out of buttons, which is why M12 reads like a trade.

### After M18 — the jump subsystem, closed

Not milestones; four fixes in one subsystem, each the instrument for the
next. M18's own gate is what found them.

| | |
|---|---|
| `tests/relocate.rs` | a suite that can see what tmux does, under four server configs. First, because the other three could not be verified without it ([#21](https://github.com/alehatsman/tally/issues/21)) |
| `renumber-windows off` | tally's server holds its own indexes. A measured ✅ that was a config option ([#20](https://github.com/alehatsman/tally/issues/20), [D10](docs/10-decisions.md#d10--a-measurement-that-was-a-config)) |
| the swap | two live views trade places, and an address is not written onto a pane whose window does not hold it ([#19](https://github.com/alehatsman/tally/issues/19), [D19](docs/10-decisions.md#d19--a-live-pane-at-a-views-destination-was-left-alone)) |
| the drift | the collision repairs itself, and a sweep **counts** it. The acting sweep is held on its own number ([#22](https://github.com/alehatsman/tally/issues/22), [D25](docs/10-decisions.md#d25--a-corpse-was-only-found-at-the-address-it-claimed)) |
| the instrument | and then the sweep was found **blind to a pane that died in place**, sampling once per daemon lifetime, unstamped. Two faults, three occasions, `ms=` and a build id ([D30](docs/10-decisions.md#d30--the-sweep-answered-where-and-the-panes-had-died-in-place)) |

### What M18's gate found that its spec did not predict

Worth keeping, because it is the sharpest example of a rule this project
keeps paying for.

The owner's `~/.tmux.conf` sets `renumber-windows on`, and tally's server is
a separate *server* but **not a separate *config***. Closing one window in a
row shifted its siblings down, so a view was at the wrong index with no Move
involved, and the lazy relocation met the one branch the spec had knowingly
left broken. **Every unit test was green throughout, because a fixture
listing is typed by a human and cannot be wrong about what tmux does.**

Two ways of reporting that run were refused, and both are the same mistake:

- **"77 collisions, then none."** 77 was every `in use` the daemon ever
  logged, mostly a case fixed earlier — and the last of them predates the
  previous restart, so the quiet that follows says nothing about that build.
  **A count over a whole log is not a measurement of a change.** What the
  run supported is narrower: three moves, none followed by a collision,
  against a before where one move was followed by three.
- **"All panes aligned."** True at one moment and false within the hour, for
  a reason no move caused.

---

## The gate, and what the journal already answered

**55 hours, 1,162 lines, 544 rising edges, 78 navigation presses**, read on
2026-10-18. Three of the five questions below are closed by what was
already in it, the headline one turned out not to be measurable at all,
and fixing that is what started the clock.

### The instrument did not record the gesture it is measuring. It does now.

F0 asks *how many ambers sent you to a session that wanted nothing.*
`Journal` logged a nav line for `next`, `back` and the overview, and **a
plain pad tap logged nothing** — the gesture the product exists for,
running at a reported 341/day against **62** `next` presses in 55 hours.
So the file held the ambers that were raised and almost none of the
answers, and waiting four more days would have finished nothing.

**`journal.jumped_pad()` shipped on 2026-10-19**
([#43](https://github.com/alehatsman/tally/issues/43)), and **the window
runs from there** — not from the 2.3 days that preceded it, which can
answer the navigation questions and nothing about the amber. Seven days
were budgeted. **It took one**, because the density that day was far
above the 341/day this paragraph estimated from: 126 rises inside 17.3
hours, which is a week's worth of the original estimate and enough to
separate 74% from the coin-flip the gate was guarding against.

### And the question itself needs restating

When M1 set this gate, amber meant *the detector believes it rang and went
quiet*, so a false positive was a **detector error**. Since
[D5](docs/10-decisions.md#d5--the-bell-that-is-never-rung) the signal is a
hook, and `Stop` fires **every turn** — so amber now means *an agent
finished a turn*, about **ten times an hour**.

A hook-raised amber is essentially never *wrong*. What it can be is
**correct and ignored**. The question worth seven days is therefore:

> **of the ambers raised, how many did you go to, and how many aged out?**

Both are computable once pad jumps are logged; `after_ms` already carries
the second half. Precision and recall were the right instrument for a
signal that no longer exists.

### What 2.3 days already settled

| question | the test it was given | the data | verdict |
|---|---|---|---|
| **grouping** ([#8](https://github.com/alehatsman/tally/issues/8)) | *"if `queued` is almost always 1, the 61% was one unusual hour"* | `queued==1` on **35%** of `next` presses (22/62); depth 2+ is 65%, and 7-deep happened 13 times | **Closed.** The measurement that justified the queue holds at 24× the sample. The flat grid is final |
| **is `►` reached for** | — | **62 of 78** nav presses, 79% | the queue *is* the navigation. M8 vindicated |
| **the overview** | *"if `lit` sits at seven, the answer is a smaller address space"* | `lit` was **4**, all nine times, and never moved. All nine destinations were rows 1–8 | the map had **never once crossed an empty middle**, its motivating case. Read on 2026-10-18 as *earns its keep as one gesture instead of seven scrolls and nothing more*; **[#41](https://github.com/alehatsman/tally/issues/41) then cut it** on that same reading ([D29](docs/10-decisions.md#d29--the-overview-was-measured-out)) |
| **the amber's shape**, now n=540 | — | p50 **117 s**, p90 **983 s**, max **18.7 h**; 10% answered under 10 s | the tempo's 900 s knee lands on p90. That number was guessed well ([05](docs/05-interaction.md#the-tempo-says-how-long)) |

Still owed, and still cheap because they are already recorded: whether `>`
is used at all, whether the corner ever sits on Move, and how often a view
drifts (`sweep:` lines).

---

## The finish line

**Everything above is a record. This is the plan.**

Finished is **v1.0**, and it is four statements, none of which is "every
written spec is built":

1. the amber has a **measured number**, the one open since M1
2. a jump never wedges, and a test in this repo can see tmux
3. a stranger can clone, build, install and run it
4. the tracker holds **no undecided design**

The phases are not milestones. F0 runs in the background of all of them, and
F3 is the one that will be skipped if nobody writes it down.

### F0 — Measure the amber ✅

**Closed 2026-10-19. The amber is not noise.**

| | |
|---|---|
| ambers raised | **126** |
| you went to | **93 — 74%** |
| aged out | **33 — 26%** |
| time to reach one | p50 **212 s**, p90 **513 s**, max **1,534 s** |

Three in four sent you somewhere you went. The risk this file was built
around — nine milestones stacked on an assumption nobody had checked —
does not land.

**The window is one day, 17.3 hours, and that is the whole of the valid
data rather than a sample of it.** `via: pad` appears on 2026-10-19 and on
no earlier date: before
[#43](https://github.com/alehatsman/tally/issues/43) landed at 23:58 on
2026-10-18, pressing a pad to reach a session wrote nothing. The 2.3 days
of journal before it hold 554 more rising edges and the numerator for none
of them, so a rate computed across the whole file reads **17%** and means
nothing — it is three days of ambers divided by the one gesture the
instrument could not see. The short window is the honest one.

**What is not in doubt:** all 126 rises are `declared` — raised by a hook,
not inferred from a bell. The scanner defect that made a clipboard write
look like a ring (review w5 F1, fixed in
[#51](https://github.com/alehatsman/tally/pull/51)) reached **zero** of
them, so the number does not rest on the one path that was broken.

**What is:** 2026-10-19 was an unusually busy day on this grid — twelve
agent sessions spawned and watched inside seven hours — and a day spent
looking at the surface is a day with a high answer rate. `Stop` firing
every turn is still what makes the amber frequent. The number says it is
frequent **and** attended; it does not prove it stays attended at a
quieter density. Re-read it after a normal week, and if the rate falls,
dropping `Stop` is still the fix
([04](docs/04-signals.md#4-a-hook--it-told-us)) — it is just no longer the
expected outcome.

### F1 — Close the jump subsystem

**Code done; the gate is not.** [#21](https://github.com/alehatsman/tally/issues/21),
[#20](https://github.com/alehatsman/tally/issues/20),
[#19](https://github.com/alehatsman/tally/issues/19) and both shipped
halves of [#22](https://github.com/alehatsman/tally/issues/22) are in. The
third half — sending a squatter home rather than parking it — is **cut**,
not pending ([11](docs/11-unbuilt.md#the-graveyard)).

[#13](https://github.com/alehatsman/tally/issues/13) is **documented**
([07](docs/07-cli.md#a-detached-spawn-can-land-on-the-agents-own-first-question)),
which was the only option available: the honest fix needs a third meaning
on `tl.attention`, and what that bit means is
[#23](https://github.com/alehatsman/tally/issues/23).

**What remains:** re-run M18's gate on the live grid. Four of its six
checks never ran and two failed — three ⬜, plus one ❌ that was never
deliberately tried,
and the ❌ were caused by the renumbering, which is now pinned and has
never been re-tested against them. **The checklist is back in
[09](docs/09-testing.md#the-jump-subsystems-own-gate-and-what-is-still-owed-on-it)**
— the restructure deleted it along with the document that held it, and a
gate nobody can read is a gate nobody runs. It needs hands on the device
and is the only item here that does.

### F2 — Make the documents true ✅

- **[#23](https://github.com/alehatsman/tally/issues/23)** — ✅ **the
  keystone is in.** The amber did not change and the word did: presence
  still raises it, and the label's **value** now names the hook —
  `blocked` from `Notification`, `finished` from `Stop`, `1` from one that
  did not say. `tally ls --json` reports it as `wants`, and `state` still
  says `wants you` for all three, because the picture is for a human and a
  human wants recall. **This unblocked
  [#13](https://github.com/alehatsman/tally/issues/13)**, which needed a
  third origin on a bit that carried two meanings in one string; it can be
  a fourth word now ([04](docs/04-signals.md#one-bit-two-states--and-the-word-that-split))
- **[#14](https://github.com/alehatsman/tally/issues/14)** — ✅ fixed in
  the restructure: [07](docs/07-cli.md#it-is-exactly-a-pad-press-with-a-folder)
  takes its predicate from `frees_the_pad` rather than from the stale
  table.
- **The restructure** — thirty-five numbered documents became eleven
  stating current truth, plus [the record](docs/10-decisions.md).

### F3 — Release engineering ✅

**CI shipped** ([#35](https://github.com/alehatsman/tally/pull/35)):
`cargo test`, `clippy -D warnings`, `fmt --check`, `cargo deny`, and the
tmux suite runs rather than skips.

**And the half that decides product-or-personal-rig is now in too.**

- **a version and a channel** — `0.9.0`, and **a tag plus
  `cargo install --git`** rather than crates.io. One USB device, one OS;
  `publish = false` stays, and says why in `Cargo.toml` rather than only
  here
- **an install story carrying the hook lines** —
  [12](docs/12-install.md). This was the single largest correctness gap in
  shipping it to anyone: the attention half of this product does not exist
  without four lines in `settings.json`, and they lived only in prose in
  [04](docs/04-signals.md#4-a-hook--it-told-us). The document also carries
  how to **check** the hooks arrived, which is the part a stranger cannot
  work out — a pad that never goes amber looks exactly like a pad with
  nothing to say

**What is left is the tag itself**, because the version this ships as is
`1.0` and [F5](#f5--v10) says what has to be in the README first. F0 has
closed; what it waits on now is F1's live re-run.

### F4 — Close scope ✅

| | verdict |
|---|---|
| [#8](https://github.com/alehatsman/tally/issues/8) — the layout | **Closed on data.** `queued==1` is 35%, not "almost always 1". The flat grid is final and grouping does not get a second look |
| the squad | **Closed.** Superseded by `tally ls` ([10](docs/10-decisions.md#the-squad)) |
| [the level](docs/11-unbuilt.md#the-level) | **Closed by F0.** The pad it guards was never observed across 670 rises, and it cannot see the one case that bit |
| the words on `tl.attention` | **Closed by #23**, and in the direction that costs least: the bit is untouched and the *value* carries the origin |
| [the bridge](docs/13-bridge.md) | **Cut to a seam and five questions** on 2026-10-18, deferred until hardware. **Built 2026-10-20** once the Windows desk could answer them: two moved the design, and the device itself is a checklist, six of its eleven lines passed the same night ([13](docs/13-bridge.md#the-checklist-when-the-launchpad-is-plugged-in)) |
| peek, teleport-URL, squatter-resolution | **Closed** ([11](docs/11-unbuilt.md#the-graveyard)) |
| the acting sweep | Held on its own week of `sweep:` lines |
| [#40](https://github.com/alehatsman/tally/issues/40) — the spec system | **Cut.** `~/.config/tally/` was empty and `tallyd` loaded *8 pages, 0 slots*, so `[[slot]]`, `[[spec]]`, `tl.spec` and spawn-from-a-dark-pad were code nothing could reach. Scope bullet 2 is **reworded rather than kept** — both spawns in daily use were outside it as written ([01](docs/01-scope.md)) |
| [#41](https://github.com/alehatsman/tally/issues/41) — 512 slots | **Closed, in two halves.** The overview is gone: `lit` sat at **4** on all nine presses and the map never once crossed the empty middle its design argues from. `▲▼` and hold-`▲` home **stay** — they are how you move through the space and they are used. With those two judged on their own merits, nothing was left for the size to pay for: `ROWS` is a bounds check and a `MAX_OFFSET`, nothing iterates the space, and an empty row costs one comparison. **The address space stays at 64 rows** — shrinking it would migrate live `tl.slot` labels to reclaim nothing |
| [#42](https://github.com/alehatsman/tally/issues/42) — nine art pieces | **Kept, closed on taste.** 2,227 lines of pure, zero-coupling leaf code with its own tests, in nobody's way. Three pieces cover both stated jobs and six do not, and that is not a reason to delete something that works and that someone enjoys |

### F5 — v1.0

Tag it `v1.0.0`. The README's Status section becomes *shipped, measured
and closed*, **with the two numbers in it** — 93 ambers you went to, 33
that aged out. That is the sentence this project has owed since M1, and
the only one that was never a matter of effort.

One gate left, and it is the one that needs hands rather than time:

| | |
|---|---|
| **F0's number** | ✅ **126 ambers, 93 attended, 33 aged out** (2026-10-19) |
| **F1's live re-run** | four unrun and two ❌ on a real device ([09](docs/09-testing.md#the-jump-subsystems-own-gate-and-what-is-still-owed-on-it)). Twenty minutes, and only the owner's desk has the hardware |

**The tracker holds no undecided design**, which is the fourth statement,
and it is true as of this file: #40 cut, #41 closed in both halves, #42
kept, #23 shipped. What is open is
[#22](https://github.com/alehatsman/tally/issues/22) — the acting sweep,
held on its own week of `sweep:` lines — and that is a *measurement* that
has not finished, not a decision nobody has made. **It is the only one
open.**

**Its week had not started, and now it has.** On 2026-10-20 the sweep was
found blind to the case that motivated the question: seven view panes died
**at the addresses they claimed** when anchord dropped its connections, and
a sweep that answers *where* counted every one as zero drift. It also ran
once per daemon lifetime with no timestamp and no build id, so a week of
its lines could not have been ordered, attributed, or trusted. The
instrument now reports two faults, runs on every reconnect and every 15
minutes, and the daemon's first line names the commit
([D30](docs/10-decisions.md#d30--the-sweep-answered-where-and-the-panes-had-died-in-place),
[06](docs/06-views.md#a-week-of-lines-and-what-the-first-instrument-could-not-give)).
**The clock on #22 starts from this build**, which is what the build id is
for.

[#69](https://github.com/alehatsman/tally/issues/69) — a moved session
keeping the self-address its SessionStart note gave it — **closed
2026-10-20, and not by a change here.** The note is written by
`~/dotfiles`, so the fix was a wording one: the address is stated *as of
that message* rather than as identity, and the note points at
`tally ls --json` -> `.you`, which re-derives it on every call. Nothing in
`src/` ever held that text, and no `--json` contract moved. The
escalation, if a stale address ever causes a real misplacement, is a
`UserPromptSubmit` re-announce — worth building against evidence and not
before.

### Sequencing

```
#43 log pad jumps       ✅ 2026-10-19 — the clock could not start without it
F0  measure the amber   ✅ 74% attended, 26% aged out — the amber is real
F1  the jump subsystem  ✅ code — M18's live re-run is all that remains,
                           and it needs hands on the device (docs/09)
F2  the documents       ✅ #23 the keystone, #14, the restructure
F3  release eng         ✅ CI, version 0.9.0, install story with the hooks
                           — the v1.0 tag itself waits on F1's gate
F4  close scope         ✅ #40 cut, #41 closed in both halves, #42 kept
F5  v1.0                needs F1's gate run
```

**One thing stands between here and the tag, and it is not code:** twenty
minutes with the device.

**The original risk is retired, and it is the one that mattered.**

**F0 could have reported that the amber is noise**, with nine milestones
stacked on the assumption it is not. It reported the opposite: 74%
attended, at ten rises an hour, on a surface whose whole claim is that you
can ignore it until it speaks. Dropping `Stop`
([04](docs/04-signals.md#4-a-hook--it-told-us)) stays on the table and is
no longer the expected outcome.

**What replaces it is smaller and honest:** the number is one busy day,
not a week. It is enough to tag on, because the decision it was gating —
build on the amber, or rebuild it — now has evidence pointing one way
instead of none pointing either.

**The F3 risk is retired.** It was the one that decided
product-or-personal-rig, and nothing about using it every day was ever
going to make it urgent; it is closed by a document rather than by a
feeling that it mattered enough.
