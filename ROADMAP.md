# Roadmap

Each milestone retires one risk and is independently abandonable. No
milestone starts before the previous one's exit criteria are met.

Ordering principle, borrowed from anchor: **the risky subsystem first, the
ergonomics last.** Here the risk is in an unusual place — the hardware is
documented but unverified, and the *attention signal* is the thing the
product lives or dies by. The launcher is the easy half.

---

## M0 — Prove the manual

Not a milestone of code. Plug the device in and make
[docs/02-surface.md](docs/02-surface.md) true.

Scope: a throwaway script. Enumerate ports; device inquiry; enter programmer
mode; reproduce the manual's three worked examples; dump every press.

Exit:
- the port-name substring that works on this macOS is written down
- the palette table in `02-surface.md` is corrected against what the pads
  actually look like
- a press round-trips in under ~10ms
- the device returns to Live mode and its Setup menu works

Risk retired: that any of this document is fiction.

---

## M1 — The grid tells the truth

The product, minus every verb. **No spawning, no killing, no jumping.** It
watches and it lights up.

Scope: `midir` surface layer (encode, diff, one-SysEx frames, programmer
mode in/out, signal-safe restore); anchor client ported from teleport;
observer watchers per session; signal derivation per
[04-signals.md](04-signals.md); slots from `tl.slot` labels; the colour
language; the logo health light; the reconcile loop.

Exit:
- sessions created from a shell — `anchor create --label tl.slot=…` — appear
  on the grid within one tick
- an agent that rings and goes quiet flashes amber, and stops when it is
  answered
- unplug / replug, kill `tallyd`, stop `anchord`: every row of
  [08-lifecycle.md](08-lifecycle.md#failure-modes-and-what-each-looks-like)
  behaves as written
- **a week of real use, counting every time the amber was wrong**

Risk retired: the only one that matters. If the attention signal is not
trustworthy, stop — the rest of the program is a worse fuzzy finder.

---

## M2 — Jump

Scope: tmux view windows (`@tally_session`), the jump, raising the terminal,
recreating a lost view, the tmux-server-missing check.

Exit: press a lit pad from anywhere, end up in that agent's window. View
windows close and reopen without the agent noticing.

---

## M3 — Spawn and the destructive chords

Scope: `slots.toml`; spawning an unbound slot; hold-then-tap modifiers;
kill / respawn / purge / mute; the highlight-while-held feedback.

Exit: a full day driven from the grid without opening the config file. No
accidental kills.

---

## M4 — Pages and the title scroll

Scope: top-row pages; cross-page attention on the logo; hold-to-scroll a
title.

Exit: eight pages of work, and you can tell from the logo that something
off-screen wants you.

---

## Later, unscheduled

- `tally` CLI, if it turns out to be needed ([docs/10](docs/10-open-questions.md))
- a `Notification` hook as the authoritative attention source
- teleport URL as a secondary jump target
- a second device — only with a second device in the room

---

## Sequencing

```
M0  prove the manual  ← is the hardware what the PDF says
M1  the grid tells    ← the only real risk: is the signal trustworthy
M2  jump              ← the payoff gesture
M3  spawn + chords    ← the launcher half
M4  pages + text      ← ergonomics
```
