# 11 — Specified, not built

Nothing here is waiting on a measurement any more: the one spec that was
is closed, and the rest of this file is the list of things that will not
be built and why. Everything in `01`–`09` is
shipped and running; nothing here is. The bridge left this file when it
was built ([13](13-bridge.md)), and keeps a short section here saying so.

**Read the verdicts as verdicts.** Each item says what would revive it, and
for most of them the honest answer is *nothing we expect to happen.*

---

## The level

**A veto over a stale amber, never a source of one.**

> ✅ **Closed by F0, 2026-10-19. Not built, and not going to be** — see
> [the roadmap](../ROADMAP.md#f4--close-scope-). F0 reported no stuck
> ambers, so this closed rather than shipped; what follows is the record
> of why it was not needed. It was held on 2026-10-18 on three facts that
> moved against it after it was written. The stuck pad it guards against
> has **never been observed** — 670 rising edges (544 over 55 hours of
> journal, then 126 in F0's own window), no report of one. It **cannot see the case that actually bit**: a session
> on the startup trust dialog is absent from `claude agents --json`
> entirely ([#13](https://github.com/alehatsman/tally/issues/13),
> [07](07-cli.md#a-detached-spawn-can-land-on-the-agents-own-first-question)).
> And it is the only proposal left that widens
> [08](08-daemon.md#one-process-no-socket)'s one-socket rule, which it
> admits below.

It was held on [the week of `attention.jsonl`](09-testing.md#the-honest-gap-today):
a veto changes what gets recorded, and measuring the architecture the hooks
actually specified was worth more than measuring it already patched.
The measurement came back with the amber real and attended (126 raised,
93 gone to) and no stuck pad in it, which was the condition for closing.

### Why it is worth building at all

Attention is on hooks, on both edges, because the agent never rings
([D5](10-decisions.md#d5--the-bell-that-is-never-rung)). That is right and
is not reopened here.

But **a stuck pad is not a tuning problem; it is the whole product failing
in the one direction it cannot afford** — the grid saying *answer me* about
an agent that is working. The falling-edge hooks make that unlikely. They do
not make it impossible: a hook that never fires because the agent was killed
mid-turn, a `settings.json` that drifts, a wrapper that is not Claude Code
at all.

**An edge that is missed is lost forever. A level needs no falling edge.**

### What was measured

Claude Code **v2.1.236**, 2026-10-17. `claude agents --json` is documented,
needs no TTY, and returns every live session at once.

```json
{ "pid": 75590, "cwd": "…/spike3", "kind": "interactive",
  "sessionId": "915b110e-…", "name": "spike3-92",
  "status": "waiting", "waitingFor": "permission prompt" }
```

| session is | `status` | `waitingFor` |
|---|---|---|
| generating | `busy` | — |
| blocked on a tool permission dialog | `waiting` | `permission prompt` |
| blocked on a plan approval | `waiting` | `permission prompt` |
| blocked on an `AskUserQuestion` | `waiting` | `input needed` |
| at a prompt, turn finished | `idle` | — |
| at a prompt, nothing ever sent | `idle` | — |
| **sitting on the startup trust dialog** | **absent from the array entirely** | — |

One call costs **≈550 ms** and covers every session, so this is one poll for
the whole grid rather than one per pad. A submit shows as `busy` within a
single poll, and `waiting` landed **1.15 s** after the `Enter` that led to
it.

**`waiting` is held, not emitted.** It stayed true for as long as the dialog
was up and went back to `idle` the moment it was cancelled. That one
sentence is the entire argument.

### The decision

**Hooks stay the source of attention. The status may clear a false positive
and nothing else.**

```text
tl.attention set  AND  status is busy    ->  the label is stale, drop the amber
tl.attention set  AND  status is waiting ->  amber, and it is corroborated
tl.attention set  AND  status is idle    ->  amber, unchanged — this is the Stop case
tl.attention unset                       ->  nothing, whatever status says
```

**One direction only.** A veto cannot invent an amber, cannot miss one, and
cannot disagree with a hook about a pad that is dark. The only thing it can
do is end a stuck one.

**Why this is not the floor that was rejected.** Output-clearing was thrown
out because *"a signal with two writers disagreeing twice a second is worse
than one writer that can be absent."* **The objection was two writers, and
this adds none.** The hook is still the only thing that writes
`tl.attention`; `claude agents --json` is a second *reader*, of a different
fact, consulted when the label is already set. The race that killed
output-clearing cannot occur, because there is nothing to race with.

And it fails the right way: if the status is unavailable — `claude` missing,
command slow, a session that is not Claude Code at all — the veto does not
fire and the amber stands. **Losing it costs exactly what we have today.**

**It does not replace `Stop`, and that is the headline.** A finished turn
reports `idle`, and so does a session that has never been given anything —
the two are the same string. `claude agents --json` covers the dialogs,
which the config mostly auto-answers, and is **silent about the case the
amber was built for**. Anyone reading this as *the signal arrived, delete
the hooks* has it backwards.

### What it costs

**A second reader**, and [08](08-daemon.md#one-process-no-socket) has a rule
about that: *the thing that already has a socket gets to keep being the only
one.* The squad spec held that line by saying only a squad would ever read
`claude agents` and `tallyd` never would. **That line does not survive this
document**, and saying so is the point of writing it down rather than
quietly widening the rule.

What it survives *as*: the roster is still `anchor ls` alone, the clock is
still the reconcile tick, and the new read answers exactly one question —
*is this agent working right now* — about a pad that is **already amber**.
It is not a source of slots, of liveness, or of identity.

**It cannot be a source of liveness**, and the trust-dialog row is the
proof: a session sitting on *do you trust this folder* is **not in the array
at all**. A session spawned into a fresh directory would be live, visible on
the grid, and invisible to this command. anchor knows it exists. Nothing
else does.

**The cadence is not the reconcile tick.** 550 ms per call against a 2 s
tick is a quarter of the duty cycle, forever, to fix a pad that has been
wrong for minutes. A veto every ten seconds corrects a stuck amber long
before a human walks back to the desk, at a twentieth of the cost. **The
failure is slow, so the check can be.**

### Open questions

1. **Is the vocabulary bigger than three values?** `permission prompt`,
   `input needed`, and absent-entirely are what one afternoon produced.
   Nothing documents the set, so the veto must treat *unknown* as *not
   busy* — the safe direction, and it should be stated in code.
2. **Does `busy` ever lie while an agent waits?** The veto's one job is
   trusting `busy` to mean *working*. Measured it is clean, but a long tool
   call that is secretly blocked on something would read `busy` and suppress
   a legitimate amber. That is the one way this can do harm.
3. **Who else is on the grid?** `isayes`, `codex` and bare shells have no
   `claude agents` row. The veto never fires for them, which is correct —
   but it means a stuck pad has no guardrail at all on anything that is not
   Claude Code.

---

## The bridge

**Built, and moved to [13](13-bridge.md).**

> ✅ **2026-10-20: out of this file.** It was cut here on 2026-10-18 to a
> seam and five questions, with the rule *answer those five before
> designing anything*. They were answered on the Windows desk — four by
> running something, one from the protocol — and two of them moved the
> design: the bridge listens rather than `tallyd`, and `Device` stays one
> type instead of becoming the trait [01](01-scope.md#how-to-reject-a-feature)
> forbids. The answers, what they changed, and the hardware checklist —
> steps 1–6 passed on a real Launchpad the same night, 7–11 still owed —
> are in [13](13-bridge.md).

---

## The graveyard

Ideas that were real, and are not coming back. Each is here so it is not
re-proposed as new.

| | why it is closed |
|---|---|
| **peek** — *what is this pad?* | **Closed by `tally ls`**, the same way the squad was, and nobody noticed for a while. *What is this pad* is a listing with the cwd in it. The only gap left is answering it **without a terminal**, and the only mechanism this device has for that is the title scroll, which [D3](10-decisions.md#d3--the-title-scroll) proved unreadable at eight pads wide. **No mechanism, no feature** |
| **teleport URL as a jump target** | Five lines once the session id is in hand, for a use case that has not come up in eighteen milestones. The desk has a grid; the phone has teleport; they need no integration because they are both anchor clients |
| **resolving a squatter to its own address** | The user-visible wedge is already gone: a squatter is **parked** above the column range, so no pad can name it, and the lazy path brings it home ([06](06-views.md#drift-that-nothing-was-looking-for)). Sending it home eagerly needs the page table inside `jump`, which points a dependency the wrong way, for no symptom anyone would see |
| **solo** | Closed in favour of Move — *arrangement beats state*. Returns only if a week of Move shows people arranging and still being dragged away |
| **a cycling verb key**, **a focused colour**, **output-clearing as a floor**, **`tally here` / `spawn next`**, **the squad** | All closed with their arguments in [10](10-decisions.md#closed-proposals) |

## Still genuinely open

| | status |
|---|---|
| **the acting sweep** | Held on its own number. The counting sweep ships; the one that repairs waits for a week of `sweep:` lines, because **a sweep that repaired silently would answer the question by erasing it**. Blast radius if built: every `tallyd` restart rearranging panes on a live grid, at the moment the daemon knows least ([#22](https://github.com/alehatsman/tally/issues/22)) |
| **global brightness / LED sleep** | Documented, unspent, orthogonal. The wrong axis to lean on, because the colour language is built on *bright means where you are*. A separate and smaller idea if ever specced ([03](03-surface.md#other-messages-worth-knowing)) |
| **a second device** | Only with a second device in the room |

## Open scope questions

**All three are answered.** They are larger than the
graveyard above because each removes something that *works*, and that is
a decision about what this product is rather than a cleanup.

**The spec system is cut.** `[[slot]]`, `[[spec]]`, `tl.spec` and
spawn-from-a-dark-pad are gone, `slots.toml` is one table, and [01](01-scope.md)'s second bullet is reworded to describe
the spawning that actually happens
([#40](https://github.com/alehatsman/tally/issues/40),
[D8](10-decisions.md#d8--slotstoml-as-the-spawn-path)).

### 512 slots for four used rows

`lit` — the count of non-empty rows — sat at **4**, every one of the nine
times the overview was opened across 55 hours, and never moved. Seven rows
of sixty-four have *ever* held a session.

The 64-row space is sized by the hardware, not by any observed need.

> ✅ **Answered 2026-10-19, in two halves. The space stays at 64 rows.**
>
> The three features the size paid for were `▲▼` scrolling, hold-`▲` home,
> and the overview. **The overview is cut** on its own evidence — nine
> presses, `lit` stuck at 4, all nine destinations inside the window they
> started in, the empty middle never once crossed
> ([05](05-interaction.md#the-overview-cut), #41). The other two are
> **kept**: they are how you move through the space, and they are used.
>
> Once those two were judged on their own merits rather than as a tax on
> the size, **nothing was left for the size to pay for.** The question
> stops being *is 64 rows too many* and becomes *what does an empty row
> cost* — and the answer is nothing. `ROWS` appears in `src/slot.rs` as a
> bounds check and as the derivation of `MAX_OFFSET`, and nowhere else.
> Nothing iterates the space, nothing allocates per row, and `Pages` holds
> eight strings. The grid is built from the sessions that carry a
> `tl.slot`, so fifty-seven empty rows cost one comparison.
>
> **An unused address is not an allocation.** The measurement that argued
> for shrinking also stopped accruing when the overview was cut — `lit`
> keeps the nine samples it has and will never get a tenth — so no further
> evidence is coming either way.

**And shrinking is not free, which is what settles it:** addresses live in
`tl.slot` labels on live *and* dead anchor rows, and in muscle memory, and
[02](02-model.md#the-address-pinned) pins the format deliberately so
humans can type it into shell scripts. A migration that renumbers live
labels or strands them, to reclaim one comparison, is the wrong trade in a
direction that cannot be undone.
[#41](https://github.com/alehatsman/tally/issues/41), closed.

### The reel is nine pieces

`src/art/` is **2,157 lines, about 12% of the crate**, for a mode behind a
button that did nothing. Its two jobs are *demo* and *all-64-pads pixel
test*, and three pieces already do both — Plasma exercises every pad every
frame, Claude is the photograph, Uprising is the joke.

> ✅ **Verdict, 2026-10-19: kept.** The other six are **not** cut. This
> was always the one of the three that was taste rather than
> architecture, and taste answered: the code is pure, has its own tests,
> couples to nothing, and is in nobody's way. The case for cutting was
> *three pieces cover both stated jobs*, which is true and is not a reason
> to delete something that works.
>
> The findings each piece carries stay where they are — Life reseeds
> because a frozen grid is indistinguishable from a crashed daemon,
> Rain's drop moves at most 15/16 of a pad so it can never skip one, Fire
> propagates into a fresh board, Pacman rotates the ghost because it is
> the detail that makes somebody look twice
> ([05](05-interaction.md#the-nine-pieces)). They would have survived the
> deletion only if someone remembered to keep the prose, which is one
> more argument against deleting.
> [#42](https://github.com/alehatsman/tally/issues/42).
