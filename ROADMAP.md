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

> ✅ **Done, 2026-10-16.** Every exit criterion met against the real device.
>
> **The manual is accurate.** Port names, addressing, device inquiry, all
> three worked examples and the full palette came back exactly as written.
> Round trip **~1.9 ms**; a full-surface SysEx arrives and renders in one
> message; amber-flashing-against-0 reads as urgent; every press gives
> exactly one release, including under a held chord, with no key repeat;
> Live mode and the Setup menu restored on SIGTERM, verified by readback
> rather than assumed.
>
> **Two of our own claims were wrong**, and neither was Novation's fault:
> the port is **not** exclusive on macOS — CoreMIDI multiplexes, so
> `tallyd` needs its own single-instance lock — and the SysEx byte budget
> was off by one per frame.
>
> Residue, both harmless:
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
- ✅ sessions created from a shell — `anchor create --label tl.slot=…` —
  appear on the grid within one tick
- ✅ an agent that rings and goes quiet flashes amber, and stops when it is
  answered. Both directions confirmed on hardware, including the two cases
  that break the predicate: output in the bell's own chunk must not go
  amber, and a newline after the bell must not cancel it
- ✅ stop `anchord`: logo dark red, grid **frozen**, then dark red `lost`
  pads when it returns — all three states watched on hardware against a
  private daemon held down deliberately
  ([08](docs/08-lifecycle.md#failure-modes-and-what-each-looks-like))
- ✅ unplug / replug the device: the grid returns by itself, confirmed on
  hardware. Found that a failed write is *not* how a disconnect shows up —
  `CoreMIDI` sends into a void without error — so liveness is a readback
  heartbeat, which also catches the replugged device coming back in Live
  mode
- ✅ `SIGKILL` `tallyd` mid-run, change the sessions underneath it, restart:
  all pads rebuilt correctly from `anchor list` with no ghosts of the old
  frame. Nothing was carried across the kill
- ⬜ **a week of real use, and a number out of `attention.jsonl`** — how
  many ambers sent you to a session that did not want anything. Counted,
  not remembered ([09](docs/09-testing.md#what-a-green-suite-does-not-prove)).
  The journal is writing; the week has not been lived.

Risk retired: the only one that matters. If the attention signal is not
trustworthy, stop — the rest of the program is a worse fuzzy finder.

---

## M2 — Jump

Scope: tmux view windows (`@tally_session`) in a dedicated `tally:` session,
the jump with an explicit client target, raising the terminal, recreating a
lost view, starting a tmux server that is not there.

Exit:
- ✅ press a lit pad from anywhere, end up in that agent's window —
  confirmed from a real press through the launchd daemon
- ✅ view windows close and reopen without the agent noticing: a killed
  view leaves the session `running`, and the next press recreates it with
  the scrollback replayed from offset 0

> Two bugs worth keeping. `set-option` without `-w` sets a *session*
> option, so every view inherited the first session id ever tagged and a
> jump would land on the wrong window — the exact failure
> [docs/06](docs/06-jump.md) calls worse than creating a new one. And
> launchd hands an agent `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, which
> contains neither `tmux` nor `anchor`: the jump worked every time by hand
> and failed under the service manager, which is the only configuration
> that matters.

---

## M3 — Spawn and the destructive chords

Scope: `slots.toml` (`[[page]]`, `[[spec]]`, `[[slot]]`); spawning an
unbound slot; hold-then-tap modifiers per
[05](docs/05-interaction.md#the-exact-rules-of-a-chord); kill / respawn /
purge / mute, the last written to `tl.mute`; the highlight-while-held
feedback.

Exit:
- ✅ every gesture lands on hardware, through a real `slots.toml`: spawn
  from a dark pad, kill, respawn, and all four previews. Zero errors in
  the daemon log across the session
- ✅ **the preview reveals what the colour language cannot.** Holding
  purge lights a pad that renders as nothing, because clean exits keep
  their `tl.slot` behind a dark pad. Confirmed: dim, dark-red, dark-red
  across a running, a finished and a failed slot
- ✅ a fumble is inert *and* visible. Two modifiers down disarms the
  chord, and the preview disappearing is what tells you so
- ⬜ a full day driven from the grid without opening the config file. No
  accidental kills.

> The one bug worth keeping, and a green suite could not have found it.
> anchord records a terminate as an ordinary non-zero exit — there is no
> field naming who asked, and there should not be — so the pad you just
> deliberately killed rendered identically to an agent that crashed. Red
> stops meaning *go and look* the moment you can cause it on purpose.
> Fixed with `tl.killed`, written before the terminate so the pad never
> flashes red on its way to dark. Found by running the new verbs against
> a live anchord rather than against a reply captured by hand.

---

## M4 — Pages

Scope: top-row pages; cross-page attention on the logo.

Exit:
- ✅ a session on a page you are not looking at reports itself **twice**,
  deliberately: the page button says *which* page wants you, the logo
  says *that* some page does. Both confirmed pulsing amber together,
  from a bell derived on page 2 while sitting on page 1
- ✅ walking onto the page is how you stop being told about it — the
  logo goes dark the moment the attention is on-screen
- ✅ the top row is readable at a glance, which took a second attempt

> **The top row had to be rebuilt.** It lit the current page white and
> occupied pages turquoise: both bright, both legible, and no way to tell
> which was which without recalling a table. *"I am confused which one is
> selected, the white one or the turquoise one."*
>
> The same failure as the three greens, and it is now stated as a rule
> rather than a fix: **exactly one thing in the top row is bright, and it
> is always where you are.** Brightness answers *where am I* before hue
> answers anything; hue is then spent only on the thing that is urgent.

> **The title scroll was cut**, after being built and run. Eight pads is
> about two legible characters at a time, so a scrolling title is a
> two-character window you guess through. The verdict after one hold:
> *"impossible to read anything."*
>
> Third time this project has learned the same lesson — after the three
> greens and the hard amber flash — and the one worth stating plainly:
> **a surface fact only shows up on the surface.** The byte budget was
> fine, the sanitiser was correct, the tests were green, the feature was
> worthless. The build was not wasted; it bought the measurement, which
> is the only thing that could have settled it.
>
> So the pad says *that* something wants you, and never *what*. The what
> is one press away in a window with eighty columns and a font
> ([05](docs/05-interaction.md#the-title-scroll--tried-and-abandoned)).

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
M4  pages             ← ergonomics
```
