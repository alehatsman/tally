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

## The gate

**A week of `attention.jsonl`, and a number out of it** — how many ambers
sent you to a session that did not want anything.

Unchanged since M1. M6 is what made the number possible, by giving the
signal a source that fires. **The journal holds a little over a day.** Nine
milestones have shipped on top of a signal whose trustworthiness has never
been measured to the standard this document set for it, and that is the
honest statement of where the project stands.

Four other weeks are owed, and they are cheap because they are already being
recorded:

| what | read it from | what it decides |
|---|---|---|
| is `►` reached for, and how deep is the queue | `kind:"nav"`, `via:"next"` | if `queued` is almost always 1, the 61% that justified M8 was one unusual hour, and [#8](https://github.com/alehatsman/tally/issues/8)'s grouping argument gets its second look |
| is the map reached for, and does `lit` move off seven | `via:"overview"` | if `lit` sits at seven for a month, the follow-up is not a better map, it is a **smaller address space** |
| is `>` used at all | shell history against nav lines | if the shell still wins, the launch column has again been given work it does not want |
| does the corner ever sit on Move | the daemon log | if it does not, M12 spent a double-tap on nothing and solo gets its second look |
| how often does a view drift | `sweep:` lines | if it reads zero for a week, the acting sweep is not worth its blast radius ([06](docs/06-views.md#the-sweep-counts-and-does-not-touch)) |

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

Seven days of `attention.jsonl`, uninterrupted, and the number. Alongside
it, the five questions in [the gate](#the-gate), which are already being
recorded and cost only the reading.

**Exit:** a dated paragraph with the numbers in it. **If the false-amber
rate is bad, stop and fix the signal** — [04](docs/04-signals.md) said so
first, and nine milestones are stacked on the assumption it is fine.

### F1 — Close the jump subsystem ✅

Done. [#21](https://github.com/alehatsman/tally/issues/21),
[#20](https://github.com/alehatsman/tally/issues/20),
[#19](https://github.com/alehatsman/tally/issues/19) and the two shipped
halves of [#22](https://github.com/alehatsman/tally/issues/22) are in; the
acting sweep is deliberately held on F0's shape of argument.

**What remains:** [#13](https://github.com/alehatsman/tally/issues/13), the
silently deadlocked pad — document the `printf '\r' | anchor attach` recipe
today for free; the honest fix is its own decision. And **re-run M18's gate
on the live grid**: four ⬜ remain, and the two ❌ were caused by the
renumbering, which is now pinned and has never been re-tested against them.

### F2 — Make the documents true

Runs alongside everything.

- **[#23](https://github.com/alehatsman/tally/issues/23) first, and it is
  promoted.** Filed as *"not blocking anything"*; it now blocks the workflow
  actually in use — agents read `tally ls` and spawn their own children off
  it. `tl.attention` means *blocked* and *finished* with one bit, `Stop`
  fires every turn and `Notification` rarely, so the common case is the one
  the word is wrong about. **The amber does not change; the word does.**
- **[#14](https://github.com/alehatsman/tally/issues/14)** — a pass over the
  spawn-refusal text, because a claim that drifted has neighbours. Carried
  into [07](docs/07-cli.md#it-is-exactly-a-pad-press-with-a-folder) and worth
  re-reading against `frees_the_pad` rather than against prose.
- **This rewrite.** Thirty-five numbered documents became eleven stating
  current truth, plus [the record](docs/10-decisions.md). The amendment
  banners are gone because the thing they amended is gone.

### F3 — Release engineering

**The largest structural gap.** No `.github/`, `version = "0.0.0"`,
`publish = false`. `clippy.toml` and `deny.toml` exist and no automation
reads them.

- CI: `cargo test`, `clippy -D warnings`, `fmt --check`, `cargo deny`
- **the tmux test must run in CI, not skip there.** Skip-if-absent keeps it
  from breaking a contributor; a green run that did not run it is the exact
  pattern this file complains about everywhere else
- a version, and a decision: crates.io, or a tag plus
  `cargo install --git`. It drives one USB device on one OS, so the second
  is probably the right size
- an install story that **includes the `settings.json` hook lines**. The
  attention half of this product does not exist without them, and today they
  live only in prose ([04](docs/04-signals.md#4-a-hook--it-told-us))

### F4 — Close scope

A verdict written into each document. Not a build.

| | |
|---|---|
| [#8](https://github.com/alehatsman/tally/issues/8) — the layout | **Narrowed to one experiment**: light row 4 amber, stand where you stand, say "four" without counting. Then the sentence. The owner's read is that the current layout is final |
| the squad | **Closed.** Superseded by `tally ls` and an agent that spawns its own members ([10](docs/10-decisions.md#the-squad)) |
| [the level](docs/11-unbuilt.md#the-level) | Held on F0 **by design**. Once the number exists: build, or close |
| [the bridge](docs/11-unbuilt.md#the-bridge) / M17 | Specified, unbuilt, five probes never run, no Windows hardware in reach. **Deferred-until-hardware**, not pending |
| the acting sweep | Held on its own week of `sweep:` lines, for the same reason as the level |

### F5 — v1.0

Tag it. The README's Status section becomes *shipped, measured and closed*,
**with the false-amber number in it** — the sentence this project has owed
since M1, and the only one that was never a matter of effort.

### Sequencing

```
F0  measure the amber   ─────────────────────────────────▶  (7 days, in the background)
F1  the jump subsystem  ✅ except #13 and M18's re-run
F2  the documents       #23 → #14
F3  release eng         CI → version → install                 independent, start any time
F4  close scope         gated on F0's numbers
F5  v1.0                needs all four
```

**The honest risk in this ordering is F3.** F1 was specified work with
repros attached and got done because it hurt. F3 is the one that decides
whether this is a product or a personal rig, and nothing about using it
every day will ever make it urgent.
