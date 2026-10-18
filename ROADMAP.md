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
| **M17** | The bridge | **specified, not built** — see below |
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
2026-10-18. The journal is at **2.3 days of the seven** F0 asks for. Three
of the five questions below are closed by what is already in it, and the
headline one turns out not to be measurable at all yet.

### The instrument does not record the gesture it is measuring

F0 asks *how many ambers sent you to a session that wanted nothing.*
`Journal` logs a nav line for `next`, `back` and `overview`
(`jumped_next`, `jumped_back`, `jumped_to`). **A plain pad tap logs
nothing** — and that is the gesture the product exists for, running at a
reported 341/day against **62** `next` presses in 55 hours.

So the file records the ambers that were raised and almost none of the
answers. **Waiting four more days finishes nothing**; one
`journal.jumped_pad()` beside the existing three is the whole fix, and it
has to land before the week runs rather than after
([#43](https://github.com/alehatsman/tally/issues/43)).

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
| **the overview** | *"if `lit` sits at seven, the answer is a smaller address space"* | `lit` was **4**, all nine times, and never moved. All nine destinations were rows 1–8 | the map has **never once crossed an empty middle**, its motivating case. It earns its keep as *one gesture instead of seven scrolls* and nothing more — and the follow-up is the address space ([11](docs/11-unbuilt.md#512-slots-for-four-used-rows)) |
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

### F0 — Measure the amber

**Blocked on a five-line change, not on time.** Log a nav line for a plain
pad jump ([#43](https://github.com/alehatsman/tally/issues/43)), *then*
seven days, and read it against the restated question above: how many
ambers you went to, and how many aged out.

**Exit:** a dated paragraph with those two numbers in it. **If most ambers
age out, the signal is noise and the fix is in
[04](docs/04-signals.md#4-a-hook--it-told-us), not here** — `Stop` on every
turn is the hook that makes the amber frequent, and dropping it is a real
option the data has not yet earned.

### F1 — Close the jump subsystem ✅

Done. [#21](https://github.com/alehatsman/tally/issues/21),
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

**What remains:** re-run M18's gate on the live grid. Four ⬜ and two ❌,
and the ❌ were caused by the renumbering, which is now pinned and has
never been re-tested against them.

### F2 — Make the documents true

- **[#23](https://github.com/alehatsman/tally/issues/23)** is now the
  **keystone**, and it has acquired a second dependant. It already blocked
  the agents-read-the-grid workflow; it now also blocks #13's real fix,
  because *noticing a pad that consumed no input* would be a third origin
  for a bit that already carries two. **The amber does not change; the word
  does.**
- **[#14](https://github.com/alehatsman/tally/issues/14)** — ✅ fixed in
  the restructure: [07](docs/07-cli.md#it-is-exactly-a-pad-press-with-a-folder)
  takes its predicate from `frees_the_pad` rather than from the stale
  table.
- **The restructure** — thirty-five numbered documents became eleven
  stating current truth, plus [the record](docs/10-decisions.md).

### F3 — Release engineering

**CI shipped** ([#35](https://github.com/alehatsman/tally/pull/35)):
`cargo test`, `clippy -D warnings`, `fmt --check`, `cargo deny`, and the
tmux suite runs rather than skips.

What is left, and it is the half that decides product-or-personal-rig:

- a version, and a decision: crates.io, or a tag plus
  `cargo install --git`. It drives one USB device on one OS, so the second
  is probably the right size
- an install story that **includes the `settings.json` hook lines**. The
  attention half of this product does not exist without them, and today
  they live only in prose ([04](docs/04-signals.md#4-a-hook--it-told-us)).
  This is the single largest correctness gap in shipping it to anyone

### F4 — Close scope

| | verdict |
|---|---|
| [#8](https://github.com/alehatsman/tally/issues/8) — the layout | **Closed on data.** `queued==1` is 35%, not "almost always 1". The flat grid is final and grouping does not get a second look |
| the squad | **Closed.** Superseded by `tally ls` ([10](docs/10-decisions.md#the-squad)) |
| [the level](docs/11-unbuilt.md#the-level) | Held on F0, and **expected to close**: the pad it guards has never been observed in 544 rises, and it cannot see the one case that bit |
| [the bridge](docs/11-unbuilt.md#the-bridge) | **Cut to a seam and five questions**, deferred until hardware. M17 is not a pending milestone |
| peek, teleport-URL, squatter-resolution | **Closed** ([11](docs/11-unbuilt.md#the-graveyard)) |
| the acting sweep | Held on its own week of `sweep:` lines |
| **three open scope questions** | the dead spec system, 512 slots for four rows, and nine art pieces — [11](docs/11-unbuilt.md#open-scope-questions), issues [#40](https://github.com/alehatsman/tally/issues/40)–[#42](https://github.com/alehatsman/tally/issues/42). Each removes something that *works*, so each is a decision about what this product is |
### F5 — v1.0

Tag it. The README's Status section becomes *shipped, measured and closed*,
**with the two numbers in it** — how many ambers you went to, and how many
aged out. That is the sentence this project has owed since M1, and the only
one that was never a matter of effort.

### Sequencing

```
#43 log pad jumps       ◀── five lines, and F0 cannot start without it
F0  measure the amber   ─────────────────────────────────▶  (7 days, after #42)
F1  the jump subsystem  ✅ — M18's live re-run is all that remains
F2  the documents       #23 is the keystone; #14 ✅
F3  release eng         CI ✅ → version → install (with the hook lines)
F4  close scope         three open questions, #40-#42, gated on nothing
F5  v1.0                needs all four
```

**Two honest risks in this ordering.**

**F3** is the one that decides whether this is a product or a personal rig,
and nothing about using it every day will ever make it urgent. CI landing
is the easy half; the install story carrying the `settings.json` hook lines
is the half that makes the attention signal exist for anybody else.

**F0 can report that the amber is noise**, and nine milestones are stacked
on the assumption it is not. Ten rises an hour is a lot of light for a
surface whose whole claim is that you can ignore it until it speaks. If the
number says most of them age out, the fix is upstream of everything in this
file.
