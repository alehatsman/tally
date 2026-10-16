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

Exit — one line per
[unverified assertion](docs/10-open-questions.md#verified-against-hardware-2026-10-16),
because that list is what this milestone exists to delete:

- the port-name substring that works on this macOS is written down
- the palette table in `02-surface.md` is corrected against what the pads
  actually look like
- **amber-flashing-against-0 reads as urgent from three metres** — if it
  reads as "dim" instead, the attention design needs a different colour
  before M1 commits to it
- **a 331-byte all-flashing SysEx arrives intact**, at a 2s cadence plus
  input bursts, through `midir`'s macOS backend — unchunked, or chunked and
  reassembled by the device
- a press round-trips in under ~10ms
- **every press produces exactly one note-off on release**, including under
  a chord — a stuck modifier on `CC19` turns the next tap into a kill
- the device returns to Live mode and its Setup menu works

> Run 2026-10-16 against the real device. **The manual is accurate** — port
> names, addressing, inquiry and the worked examples all came back as
> written. Round trip ~1.9 ms; a full-surface SysEx arrives and renders in
> one message; amber-flashing-against-0 reads as urgent; Live-mode restore
> on SIGTERM verified by readback rather than assumed.
>
> Two of our own claims were wrong: the port is **not** exclusive on macOS
> (CoreMIDI multiplexes — `tallyd` needs its own lock), and the SysEx byte
> budget was off by one per frame.
>
> Open: note-off-on-release under a chord, eight palette entries, and
> whether dropping the clear fixes the Live-mode handover.
> [docs/10](docs/10-open-questions.md#verified-against-hardware-2026-10-16).

Risk retired: that any of this document is fiction. It is not.

---

## M1 — The grid tells the truth

The product, minus every verb. **No spawning, no killing, no jumping.** It
watches and it lights up.

Scope: `midir` surface layer (encode, diff, one-SysEx frames, programmer
mode in/out, restore on signal and on panic); anchor client ported from
teleport; observer watchers per session; signal derivation per
[04-signals.md](docs/04-signals.md); slots from `tl.slot` labels; the colour
language; the logo health light; the reconcile loop; `attention.jsonl`.

**No `slots.toml` either.** M1 has no spawning, so it needs no specs, and
unnamed pages `1`–`8` are enough to address a slot. The config file arrives
with the gesture that reads it, in M3.

Exit:
- sessions created from a shell — `anchor create --label tl.slot=…` — appear
  on the grid within one tick
- an agent that rings and goes quiet flashes amber, and stops when it is
  answered
- unplug / replug, kill `tallyd`, stop `anchord`: every row of
  [08-lifecycle.md](docs/08-lifecycle.md#failure-modes-and-what-each-looks-like)
  behaves as written
- **a week of real use, and a number out of `attention.jsonl`** — how many
  ambers sent you to a session that did not want anything. Counted, not
  remembered ([09](docs/09-testing.md#what-a-green-suite-does-not-prove)).

Risk retired: the only one that matters. If the attention signal is not
trustworthy, stop — the rest of the program is a worse fuzzy finder.

---

## M2 — Jump

Scope: tmux view windows (`@tally_session`) in a dedicated `tally:` session,
the jump with an explicit client target, raising the terminal, recreating a
lost view, starting a tmux server that is not there.

Exit: press a lit pad from anywhere, end up in that agent's window. View
windows close and reopen without the agent noticing.

---

## M3 — Spawn and the destructive chords

Scope: `slots.toml` (`[[page]]`, `[[spec]]`, `[[slot]]`); spawning an
unbound slot; hold-then-tap modifiers per
[05](docs/05-interaction.md#the-exact-rules-of-a-chord); kill / respawn /
purge / mute, the last written to `tl.mute`; the highlight-while-held
feedback.

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
- the `settings.json` line for the `Notification` hook — the `tl.attention`
  label and the `Declared` variant ship in M1 either way
  ([04](docs/04-signals.md#4-a-hook--it-told-us))
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
