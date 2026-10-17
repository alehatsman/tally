# 26 — The amber is an edge; the status is a level

[14](14-attention.md) put attention on hooks, on both edges, because the
agent never rings. It is right, it is built that way, and this document
does not reopen it.

It names one prerequisite that is still open:

> **It was never built.** `LABEL_ATTENTION` appears in the codebase three
> times: the constant, a reader, and a test. Nothing removes it, anywhere.
> […] A `Notification` hook installed against today's binary sticks a pad
> amber permanently.

A stuck pad is not a tuning problem. It is the whole product failing in the
one direction it cannot afford — **the grid saying *answer me* about an
agent that is working.** This document is about a fact that was measured
while specifying something else ([25](25-the-squad.md)), and that happens
to make the stuck pad impossible to sustain.

## What was measured

Claude Code **v2.1.236**, 2026-10-17, this machine. `claude agents --json`
is documented, needs no TTY, and returns every live session at once.

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

Timing, same rig: one call costs **≈550 ms** and covers every session, so
this is one poll for the whole grid rather than one per pad. A submit shows
as `busy` within a single poll, and `waiting` landed **1.15 s** after the
`Enter` that led to it.

**`waiting` is held, not emitted.** It stayed true for as long as the
dialog was up and went back to `idle` the moment it was cancelled. That one
sentence is the entire argument below.

## The distinction this turns on

**A hook is an edge. `status` is a level.**

[14](14-attention.md) is an edge architecture by construction: `Stop` and
`Notification` raise, `UserPromptSubmit` and `PreToolUse` lower. Edges are
cheap, they push, and they are exactly right when both of them fire.

**An edge that is missed is lost forever.** That is not hypothetical here —
the falling edge was never implemented, so today the rising one would stick
a pad amber until the session died. Every edge system needs its falling
edge to be perfect. A level needs no falling edge at all: you ask, and the
answer is true *now*.

So the two are not competitors. They fail in opposite directions, which is
what makes one useful to the other.

## The decision: a veto, not a source

**Hooks stay the source of attention. The status is allowed to clear a
false positive and nothing else.**

```text
tl.attention set  AND  status is busy   ->  the label is stale, drop the amber
tl.attention set  AND  status is waiting ->  amber, and it is corroborated
tl.attention set  AND  status is idle    ->  amber, unchanged — this is the Stop case
tl.attention unset                       ->  nothing, whatever status says
```

**One direction only, and the direction matters.** A veto cannot invent an
amber, cannot miss one, and cannot disagree with a hook about a pad that is
dark. The only thing it can do is end the failure
[14](14-attention.md#3-nothing-clears-tlattention--a-gap-not-a-bug-report)
names as a prerequisite.

### Why this is not the floor that `14` rejected

`14` threw out output-clearing as a backstop, and the reason was precise:

> Under `isayes` it clears the label within about half a second of the hook
> setting it, so the floor does not back the hook up — it **fights** it, and
> silently. A signal with two writers disagreeing twice a second is worse
> than one writer that can be absent.

**The objection was two writers, and this adds none.** The hook is still the
only thing that writes `tl.attention`. `claude agents --json` is a second
*reader*, of a different fact, consulted when the label is already set. The
race that killed output-clearing cannot occur, because there is nothing to
race with.

And it fails the right way: if the status is unavailable — `claude` missing,
command slow, a member that is not Claude Code at all — the veto does not
fire and the amber stands. **Losing it costs exactly what we have today.**

### It does not replace `Stop`, and that is the headline

`14` is explicit about which moment matters:

> The moment this grid exists to report is **the turn finishing**: the agent
> said its piece and is waiting for the next instruction. That is `Stop`.

**`claude agents --json` cannot see that moment.** A finished turn reports
`idle`, and so does a session that has never been given anything — the two
are the same string. The field covers the dialogs, which `14`'s own config
mostly auto-answers, and is silent about the case the amber was built for.

So anyone reading this as *the signal arrived, delete the hooks* has it
backwards. **The hooks are load-bearing and this is a guardrail under
them.**

## What it costs

**A second reader, and [08](08-lifecycle.md) has a rule about that.**

> *The thing that already has a socket gets to keep being the only one.*

[25](25-the-squad.md#a-third-source-of-truth-named-out-loud) held the line
by saying only a squad would ever read `claude agents` and `tallyd` never
would. **That line does not survive this document**, and saying so is the
point of writing it down rather than quietly widening the rule.

What it survives *as*: the roster is still `anchor ls` alone, the clock is
still the reconcile tick, and the new read answers exactly one question —
*is this agent working right now* — about a pad that is **already amber**.
It is not a source of slots, of liveness, or of identity.

**It cannot be a source of liveness**, and the trust-dialog row in the table
is the proof: a session sitting on *do you trust this folder* is **not in
the array at all**. A member spawned into a fresh directory would be live,
visible on the grid, and invisible to this command. anchor knows it exists.
Nothing else does.

**The cadence is not the reconcile tick.** 550 ms per call against a 2 s
tick is a quarter of the duty cycle, forever, to fix a pad that has been
wrong for minutes. A veto that runs every ten seconds corrects a stuck
amber long before a human walks back to the desk, at a twentieth of the
cost. **The failure is slow, so the check can be.**

## What does not change

- hooks are attention: `Stop`, `Notification`, `UserPromptSubmit`,
  `PreToolUse`, with `|| true`, exactly as [14](14-attention.md) specifies
- the falling-edge hooks are **still a prerequisite**. A veto makes a stuck
  pad survivable; it does not make it correct, and a design that leans on
  the guardrail instead of the rail is the same bug wearing a hat
- `anchor ls` is the roster and the reconcile tick is the clock
- the colour language, the queue, the tempo, the precedence table in
  [04](04-signals.md) — a veto changes which pads reach rule 4, not what the
  rules are

## Open questions

1. **Is the vocabulary bigger than three values?** `permission prompt`,
   `input needed`, and absent-entirely are what one afternoon produced.
   Nothing documents the set, and a value nobody has seen is a value the
   veto has no rule for — it must therefore treat *unknown* as *not busy*,
   which is the safe direction but should be stated in code.
2. **Does `busy` ever lie while an agent waits?** The veto's one job is
   trusting `busy` to mean *working*. Measured it is clean, but a long tool
   call that is secretly blocked on something would read `busy` and suppress
   a legitimate amber. That is the one way this can do harm.
3. **Does the week of `attention.jsonl` change shape?** The
   [README](../README.md) is waiting on a number, and a veto changes what
   gets recorded. Better to land the falling-edge hooks first and measure
   the architecture `14` actually specified, rather than measure it already
   patched.
4. **Who else is on the grid?** `isayes`, `codex` and bare shells have no
   `claude agents` row. The veto simply never fires for them, which is
   correct, but it means the stuck-pad failure is still fully live for every
   non-Claude pad — and [14](14-attention.md)'s prerequisite is the only fix
   those have.
