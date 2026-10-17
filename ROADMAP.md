# Roadmap

Each milestone retires one risk and is independently abandonable. No
milestone starts before the previous one's exit criteria are met.

Ordering principle, borrowed from anchor: **the risky subsystem first, the
ergonomics last.** Here the risk is in an unusual place — the hardware is
documented but unverified, and the *attention signal* is the thing the
product lives or dies by. The launcher is the easy half.

> **Read [What is next](#what-is-next) before anything else here.** The
> rule above stopped holding at M10: everything since has shipped with
> open exit criteria, and one of them has been open since M1. That section
> says which, and what closes them.

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

## M5 — `tally spawn`

Scope: the CLI half of spawning, because the directory comes from a shell
([docs/11](docs/11-cli.md)).

Exit:
- ✅ `tally spawn 1:1` in a repo claims the pad and runs `claude` there —
  verified against a live anchord: `cwd` is `$PWD` exactly, `login_shell`
  is set, and `tl.slot` is the only label
- ✅ the occupancy rule is the grid's, not a second opinion: a lit pad
  refuses and names what holds it, a dark pad with exited rows behind it
  spawns — both confirmed on real sessions
- ✅ an address off the grid is refused, never clamped — the same rule a
  label follows, and the reason it has tests
- ⬜ a week of claiming pads this way without reaching for `anchor create`

> **The config file was the wrong half for this.** `slots.toml` wants the
> folder declared in advance, which is a prediction about work you have not
> done — so the spawn-from-a-dark-pad feature built in M3 went unused from
> the day it shipped. Not broken: aimed at the wrong context. **The only
> thing that knows the directory is a shell standing in it.**
>
> Which makes this the fourth time the project has learned that a feature
> is settled by use and not by design, and the first time the lesson cost
> nothing to apply — M3's machinery is what `tally spawn` calls.
>
> The deferral in [docs/10](docs/10-open-questions.md) is what made that
> cheap. *"Defer until something is actually awkward"* meant the CLI was
> built around a measured gap rather than a guessed one, and it came out
> one subcommand wide.

---

---

## M6 — The amber exists

Not an ergonomics milestone. **M1's risk came back in the negative** and
this is the answer to it.

M1 said *if the attention signal is not trustworthy, stop*. It is not
untrustworthy — it is **absent**. 1012 `0x07` bytes across 6.6 MB of real
sessions, every one an OSC title terminator, **zero in GROUND state**. The
bell detector is correct, the tests are green, and there has never been
anything to detect ([#4](https://github.com/alehatsman/tally/issues/4)).
Every `needs_answer` line in `attention.jsonl` belongs to a synthetic
session.

Scope: `tally attention on|off`, resolving its session by walking its own
process ancestry because `$ANCHOR_SESSION` is set by nothing; hooks on
**both** edges; the journal read back at startup
([docs/14](docs/14-attention.md)).

Exit:
- ✅ a hook inside a slotted session turns its pad amber, and answering
  turns it green — confirmed end to end against the live daemon from
  inside a real session, with the first `"source":"declared"` line this
  grid has ever written
- ✅ not being under anchor is **success**: silent, exit `0`, no label.
  The hook fires on every turn of every agent on the machine and almost
  none are on the grid
- ✅ anchord unreachable is one line on stderr and exit `1`
- ⬜ a week of `attention.jsonl` with a number in it — the criterion M1
  has been carrying since it shipped, and the first week it is possible
  to run

> **The measurement that was not in the bug report.** #4 also claimed the
> stream never falls silent, so `Quiet` was unreachable and the 2 s window
> with it. That is false for Claude Code: a `claude` at its main prompt
> emits **0 bytes in 92 s**, and the live session on pad `1:1` held **42 s
> flat** between turns. The ~2 Hz repaint was `isayes`'s footer and cursor
> blink — the wrapper's UI, not the agent's.
>
> So `Working` versus `Quiet` is honest and was nearly deleted on the
> strength of a measurement taken through the wrong lens. The fifth time
> this project has learned that a surface fact only shows up on the
> surface, and the first time the lesson arrived as *we were about to
> throw away something that works*.

> **A second thing the code had outgrown.** `docs/04` promised tallyd would
> clear `tl.attention` on output. It was never built — a reader, a
> constant, a test, and nothing that removes it. A hook shipped against the
> old binary would have stuck a pad amber permanently. Found by grep while
> writing the spec, not by the suite, because there was no test for a
> feature nobody had written.

---

## M7 — Where views live

Not three fixes. **One sentence in `docs/06` was wrong**, and the three
open issues are what you get downstream of it: a view is a tmux *window*.
That was true when `tallyd` under launchd was the only thing creating
views, and it has been the definition ever since.

A view is a **pane** ([docs/15](docs/15-views.md)). Then
[#1](https://github.com/alehatsman/tally/issues/1) has a unit to adopt,
[#2](https://github.com/alehatsman/tally/issues/2) has a pane that does
not close, and [#3](https://github.com/alehatsman/tally/issues/3) has a
coordinate that is the whole address rather than most of it.

Scope, in dependency order. All three cleared by
[docs/16](docs/16-the-queue.md), which settled the layout question #8 was
holding them behind:

| step | closes | cleared |
|---|---|---|
| 1. identity to the pane, `remain-on-exit`, revive-in-place | #2 | ✅ |
| 2. adoption — `tally spawn` takes over the pane you ran it in | #1 | ✅ |
| 3. the return trip — `◄` on the top row | #3 | ✅ |

Exit, for step 1:
- ✅ kill a session while focused on its view: still attached, corpse
  readable, layout intact — killed a throwaway to check. The pane goes
  `dead=1` instead of vanishing, the agent's output stays in its history,
  and the visible line is `anchor: session exited (1)`, so the corpse says
  why it died as well as what it was doing
- ✅ kill the only live session and the `tally` session survives, with no
  stray shell window reintroduced — measured both ways on tmux 3.6b before
  the code was written: without `remain-on-exit` the session is destroyed,
  with it the session survives and the pane holds
- ⬜ a detach leaves the pad green and the next press revives the view in
  the pane it was already in — the one path nothing has exercised yet

> **Two tmux facts that argue against the obvious implementation**, both
> measured on a live 3.6b server rather than assumed. A window option is
> inherited by **every pane in the window** — so splitting a view window
> gives two panes claiming the same anchor session, today, with the
> current code. It does not bite only because nothing splits a view
> window, which is precisely what #1 makes normal. Same class as the
> missing `-w` that `docs/06` already records, one level down.
>
> And the obvious way to ask where a client is looking is wrong:
> `display-message -p -c <client>` resolves its target against an
> unrelated session, because `-c` chooses where a message would be
> *displayed*. `list-clients -F '#{client_name} #{pane_id}'` is the
> correct call, and the incorrect one looks more correct.

> **#2 goes first, and not as a compromise.** The issue asked whether to
> fix it before #1 or alongside it. It is the step the other two are built
> on: adoption needs a pane that survives its agent, and a return needs a
> pane-level coordinate to return to.

> **Steps 2 and 3 were written before anyone checked the layout work, and
> collided with it** — the second tap on a pad was already spoken for, and
> adoption was written against an address format #8 might have replaced.
> Both are released by [docs/16](docs/16-the-queue.md): the return gesture
> moves to its own button and addressing does not change. Recorded because
> a spec validated against the code and not against the other specs will do
> this again.

---

## M8 — The top row is a queue

The first hour of `attention.jsonl` answered the question M6 made askable,
and it is the one the whole layout turns on: **61% of the time more than
one agent is waiting, and 15% of the time exactly one is.** There is usually no
*the* amber pad. And the choice is not close — when two or more are amber
the gap between the longest and shortest wait is a median of 386 s.

So the bottleneck is not finding the blocked agent. It is **choosing among
several, and getting back afterwards** — and neither is a grid-layout
problem ([docs/16](docs/16-the-queue.md)).

Scope: `►` jumps to the longest-waiting amber, `◄` returns you, `▲▼` page,
and the other four top-row buttons go dark. The queue is derived from
state tallyd already holds — `Journal::asking` keys slots to their original
rise time, which #7 made durable and which turns out to be the queue.

Exit:
- ✅ `►` amber reads as *there is work* and not as *this page wants you*
- ✅ the round trip is ten seconds: `►`, answer, `◄` — measured at **1.1 s**
  between the `►` that moved you and the `◄` that brought you home
- ⬜ losing the current-page readout is not missed — and `▲▼` now scroll a
  row at a time rather than cutting to a page, which is most of why it is
  not
- ⬜ a week of `kind: "nav"` lines: how often `next` was used instead of a
  pad, how deep the queue was, and whether you answered something younger
  than the head

> **The idea this retires.** *Answer from the grid*, even narrowed to the
> one answer safe to give without reading, is still a button that answers
> an amber light rather than a command
> ([docs/12](docs/12-the-layout.md#closed-and-why-they-stay-closed)).
> Spending `►` on navigation retires it without needing a further
> argument — and navigation commits
> nothing, which is exactly why *answer-and-advance* was rejected.

> **What it costs.** `docs/05` spent four paragraphs on a rule learned in
> real confusion: exactly one page button is bright and it is the one you
> are on. This deletes it. The defence is one number — **pages used, 70 :
> 2** — and if pages ever become load-bearing this is the first decision to
> revisit.


> **The instrument caught the spec, not the code.** `◄` shipped dark: the
> origin rule said never record a view pane, which assumed you jump *from*
> nvim. Every pane this grid is used from is an anchor session, so nothing
> was ever recorded. Two `{"via":"back","ok":false}` lines an hour after
> shipping said so — written by the journalling added in the same commit,
> for exactly this. The spec was validated against the code and against the
> other specs; nothing but use could have checked it against the desk.

> **One number to watch.** Every `next` press so far reports `queued: 1` —
> one thing waiting, no choice to make. If that holds for a week, the 61%
> that justified this milestone was an artefact of one unusual hour, and
> #8's grouping argument deserves its second look.

---

## M9 — The tempo says how long

Forty-nine answered waits in `attention.jsonl` span two orders of
magnitude — p50 **187 s**, p90 **1,076 s**, max **2,403 s** — and the pad
looks identical at both ends. The grid says *something wants you* and has
never said *and it has been waiting eighteen minutes*, which is the thing
that decides whether you get up.

[M8](#m8--the-top-row-is-a-queue) answered it for the press: `►` goes to
the oldest. This answers it for the glance
([docs/17](docs/17-the-tempo.md)).

Scope: MIDI clock at 24 PPQN, 60 bpm when nothing is ageing and 140 bpm at
fifteen minutes, linear and clamped between. Plus `⌂` — the `Session`
button doing what its silkscreen says, scrolling back to row 1, which is
the one move `▲▼` make tedious.

Exit:
- ✅ **the device honours MIDI clock on the port we already open** —
  measured with `examples/clock.rs` against a live grid, 60/150/60 bpm
  between two silent baselines. *"It breathes slower and faster, it
  works."* The first time one of these perceptual gates has come back yes
- ✅ 60 bpm reads as calm and not as broken — 61% of a 10-hour day sits
  there, and it is not what anyone complained about
- ❌ **a day beside the quickening without it becoming tiring — FAILED.**
  *"the constant breathing of the buttons does make me tired"*. The same
  gate the hard amber flash failed, for the same reason: legible at a
  glance, wearing by the afternoon
- ⬜ a wedged `tallyd` is visibly frozen rather than quietly stale

**The fix was not in the tempo.** Time-weighted over 10.1 h the mapping
behaved as designed — 61% at the floor, 19% in the 120-140 band — so the
number of beats was right and the **number of pads** was wrong. `docs/17`
had explicitly defended driving four working agents at 140 bpm because one
unrelated pad had been ignored; that paragraph is the thing that made it
tiring, and it is now marked as such.

Green stopped moving. Confirmed on the device — *"green are not pulsing
no, orange yes"* — with four slots asking at the time, so the amber half
was live and checked rather than assumed. Motion is spent only on amber,
so the surface is still unless something is asking, and the one global clock now modulates
only the pads that are actually waiting. The cost: working and quiet look
identical, and *which agent is producing output* is no longer visible —
information it turned out you watch rather than act on
([docs/05](docs/05-interaction.md#nothing-blinks-and-only-attention-moves)).

> **It trades "set once and forget" for one byte, forever.** `docs/02`
> sells the palette animations on *no timer, no repaint loop, no drift*,
> and a clock gives one third of that back. The important third stays: the
> **ramp** still runs on the device, so a pulse is smooth under load and
> never stutters. Only the beat boundary becomes ours, and one byte at
> tens of hertz is cheaper than anything else on this surface.

> **Frozen breath is the point, not the risk.** A wedged `tallyd` today
> leaves a perfectly normal-looking grid showing a stale snapshot — the
> worst failure a status display can have, because it is indistinguishable
> from "nothing changed". With a clock it stops breathing mid-ramp and is
> visibly wrong from six metres. That is why the floor is 60 bpm and not
> 30: a very slow breath and a stopped one have to be tellable apart.

> **The range is narrow on purpose.** The device's own unsynced 120 bpm
> sits inside it. The failure this project has already paid for once is a
> surface that is correct and *exhausting* — the hard amber flash read as
> urgent on hardware and as stressful after a few hours. A tempo that
> sprints is that mistake with a different knob.

---

## M10 — The right column launches

A day of use said the column was one button doing work, one that could
not, and two nobody wanted:

```text
jump 341 · scroll 24 · next 4 · back 4 · kill 4 · respawn 0 · purge 0 · mute 0
```

**Respawn was unreachable, not unused.** It needs a resolvable spec and
there is no `slots.toml` on this machine — `tallyd` loads *8 pages, 0
slots* — so the preview could never light it. The same emptiness makes
tap-a-dark-pad inert.

The fact that changes the menu: **the grid knows the directory now.**
anchord keeps `cwd` and `command` on every row, dead ones included, so the
reason spawning lived in a shell stops being true for any row that has ever
held a session ([docs/18](docs/18-the-launch-column.md)).

Scope: `>` ×7 spawns into its row, inheriting the directory and command of
that row's newest session and **dropping its arguments**. Kill keeps `CC19`
and becomes the only modifier. Purge and mute move to `tally purge` and
`tally mute`.

Exit:
- ⬜ a column of seven dim buttons beside the rows reads as *launch here*
  and not as decoration
- ⬜ the row-8 rule is learnable, or is a daily irritation
- ⬜ a week in which `>` is used at all — if the shell still wins, the
  column has again been given work it does not want

> **Arguments are dropped on purpose.** A row running
> `claude --resume d37ee7e5…` describes one conversation; copying that
> would put a second pad on the same conversation, which is the one thing
> a new agent must not be. Keeping `command` keeps the choice that matters
> — a row running `isayes` gets another `isayes`.

> **Row 8 is shell-only, by rule.** Eight buttons, eight rows, and one must
> be the kill modifier. Moving kill means moving the one button that is
> 8 mm from the wrong pad, with four presses of muscle memory in the corner
> the silkscreen labels *Stop*. A grid where one row behaves differently is
> a grid you remember rather than feel — affordable only because the window
> scrolls, so row 8 is one `▼` from being row 7.

> **Purge and mute had to keep existing.** `anchor` has no `purge`
> subcommand (anchor#14), so deleting the gesture without replacing it
> would leave rows accumulating with no remedy anywhere. And mute is the
> starvation hatch for `►` and now the tempo: zero presses in a day is not
> an argument that it is unneeded, it is an argument that the signal has
> not lied yet.

---

## M11 — The overview

Seven rows of sixty-four have ever held a session, and they are already
discontiguous — row 13 sits five rows past the nearest neighbour, reachable
only by five `▼` presses across four rows of nothing:

```text
rows touched  1 2 3 4 6 8 13      of 64
```

The space is not a plane you survey, it is sparse islands you hop between,
and the surface has no gesture that shows the set. Sparsity is the argument
*for* a map: seven lit pads out of sixty-four is legible at a glance, and
every lit pad is a destination.

Scope: hold `Session` and the grid becomes the whole 64-row space, one pad
per absolute row in reading order, coloured by the worst state in that row.
Tap a pad and the window lands there. Release without tapping and nothing
happened. The window is marked by dim grey on its empty pads
([docs/20](docs/20-the-overview.md)). `⌂` moves off `Session` and onto
hold-`▲`: the press still scrolls one row, and holding it past 500 ms
snaps to row 1.

Exit:
- ✅ lit pads and a grey run reads as a map, without counting — *"the map
  reads well"* on the first hour, at four rows lit
- ⬜ row 1 to row 13 in one gesture — **unexercised**: nine presses, and
  every one of them landed inside the window it started in, because
  nothing has lived past row 8 since it shipped. The map earns its place
  as *jump to a row*; crossing the empty middle is still a claim
- ⬜ `Session` + kill disarms visibly, like two right-column modifiers do
- ⬜ a week of `via:"overview"` lines — is it reached for, and does `lit`
  move off seven
- ✅ 500 ms is unreachable by a lazy tap and does not feel like waiting —
  three `⌂` presses in the first hour, none of them apparently accidental

> **The clock stays out of `chord.rs`.** It is pure — presses in, gestures
> out, no `now` — and that is what lets the four chord rules be tested
> without a device or a timeline. The hold timer goes in `tallyd`'s 16 ms
> loop, which already has `elapsed` on the line that calls `feed`. The
> consequence: `Gesture::Home` survives and `chord.rs` never emits it.

> **A latched mode was rejected.** The top row is built on *dark does
> nothing, lit does one thing, there is no state to remember*. A latched
> overview makes the grid mean two things depending on something invisible,
> and it is the only proposal here that breaks that rule.

> **If `lit` sits at seven for a month, the follow-up is not a better map.**
> It is a smaller address space. The instrument is written to be able to
> say so.

---

## M12 — The corner carries two verbs

[M10](#m10--the-right-column-launches) closed with a rule: *"that is the
first time this surface has had nothing spare, and the bar for the next
idea is now a button taken from something that works."* Two ideas then
arrived and neither could have a button.

So `CC19` carries both, and a **double-tap** swaps them. That is not a
trick: Novation silkscreened *Stop Solo Mute* on that corner because in
Ableton it cycles modes by repeated presses. They put a mode switch there
and printed its menu on it ([docs/22](docs/22-the-mode-button.md)).

Scope: **Stop** — kill what is alive, purge what has ended, on one hold,
because the two target sets are disjoint and the preview already colours
by consequence. **Move** — pick a pad up and put it down, which is one
`tl.slot` write and the agent never learns it happened. Both thresholds
live in `tallyd`'s 16 ms loop, so `chord.rs` stays pure and emits no mode
change.

Exit:
- ✅ **red and dark red are tellable apart under a thumb**, on a grid, at
  arm's length — the one perceptual claim this design rests on, and it
  holds. Had it failed, the honest fix was splitting kill and purge back
  onto separate gestures and the merge would have been a mistake
- ✅ the dark pads lighting up under Stop read as *there is history here*,
  not as *I broke something* — which is what makes purge's targets
  affordable, since a pad hiding ended rows draws as nothing until you hold
- ✅ a double-tap is reachable without looking; 250 ms and 400 ms are
  untouched since, so the guesses were right
- ⬜ a week of logs: does the mode ever sit on Move, and does anything get
  purged from the grid that never got purged from the shell

> **This overturns [M10](#m10--the-right-column-launches) deliberately.**
> `docs/18` rejected a merged kill/purge because *"it widens the blast
> radius of the single gesture `05` spends a section protecting"*. It does.
> The cost is named rather than argued away: under the old chord the dark
> two-thirds of the grid was a free miss, and a thumb one pad off now
> forgets a dead row's log. What makes it affordable is that **the hold is
> what reveals them** — they stop being invisible at the exact moment they
> become targets.

> **The first press froze the surface, and no test could have caught it.**
> A double-tap completes on the *release*, and `tallyd` flipped the mode
> before `chord.rs` saw that release — so the button went down under one
> verb and came up under another, and a release matched by **verb**
> stranded the old one in `held` forever. `modifier()` then answered `Some`
> with nothing under the thumb: the whole grid sat in a held preview, in
> red or turquoise, with no sequence of presses able to recover. Only a
> restart cleared it.
>
> Every unit test fed presses through `Chords` alone, where the mode never
> moves. The toggle lives in `tallyd`, the stranding happened in
> `chord.rs`, **and the bug was in neither — it was in the order the two
> ran.** Fixed by matching a release on the *button*, which makes the
> pairing true by construction rather than dependent on two pieces of state
> staying in step.

---

## M13 — tally's own tmux server

**One line of shipped code was the whole bug.** `jump` switched *every
client on the server*, so pressing a pad while you were reading a diff in
`main` yanked your personal terminal into a view. A jump is supposed to put
an agent in front of you; it is not supposed to take your terminal away
to do it ([docs/24](docs/24-the-tally-server.md)).

The fix is a boundary, not a filter: **a named socket is a separate
server, and `switch-client` cannot reach a client it cannot list.**

Scope: `tmux -L tally`; a row is a session `r01`…`r64`, named by the
**absolute** row because a page name is config and config is editable; a
column is a window, so the window index simply *is* the column; a home
session for the terminal window to land on; and `tally spawn` stops
adopting the pane you ran it in, because a pane on your personal server
cannot be a view on tally's.

Exit:
- ✅ `list-sessions` on the personal socket is unchanged by any press, and
  a jump with a client attached to `main` leaves it on `main`
- ✅ a view whose command dies keeps its pane, its tag and its column, and
  the next jump respawns it in place
- ✅ a jump with no tally terminal open opens one and lands in the view
- ⬜ a week of living in a second terminal window: is a dedicated window
  for agents a relief or a thing you keep losing

> **The column was a pane, and eight of them was unreadable.** This
> document shipped pane-per-column, measured and correct, and a row of
> eight agents turned out to be eight 23-column slivers reflowing megabytes
> of scrollback. What was wrong was never the addressing — it was that a
> split shows you all eight at once when the thing you do with a row is
> read **one** of them. A column is a tab now. The pane rules are kept in
> `docs/24` rather than deleted, for the same reason `docs/15` keeps its
> adoption section.

> **And then a corpse held the slot shut.** `remain-on-exit` keeps a dead
> view's pane, which keeps its *window index* — and a window index is a
> namespace. Kill a session, spawn a new one on the same pad, press it:
> `create window failed: index 4 in use`, and the grid has no way to show
> you an error. The identity lookup was right; what was missing is that
> **the address is also a lookup**, and tally had been writing
> `@tally_slot` on every pane since this milestone without ever reading it
> back ([docs/27](docs/27-the-slot-collision.md)).
>
> Caught something else on the way: `back()` parsed the same pane listing
> with its own `split_whitespace`, so changing the separator would have
> made every origin look closed and `◄` silently stop working. Every read
> of that listing goes through one function now.

---

## M14 — `tally fork`, and four at a time

Two gaps in the CLI, both of them *the folder is the thing a shell knows*
pointed at a case `spawn` could not reach
([docs/11](docs/11-cli.md)).

Scope: `tally fork <src> <dst>` — spawn, with the directory coming from a
pad you can see rather than from `$PWD`, inheriting `command` and
`login_shell` from the newest session that pad has ever held, dead rows
included. And `--count N` on both, claiming N consecutive pads rightwards
in the address's row, refusing the whole command if any one of them is
taken.

Exit:
- ✅ the occupancy rule is the grid's, checked before anything is created,
  and a `--count` running past column 8 is refused rather than wrapped
- ⬜ a week in which either is used. `fork` is *another one of those* and
  `--count` is *spawn four on a goal*; both are guesses about how work
  arrives, which is the thing this project has been wrong about before

> **Arguments are dropped, for the third time and the same reason.** A pad
> running `claude --resume <id>` describes one conversation, and copying
> its argv would put a second agent on it. `>` learned this
> ([M10](#m10--the-right-column-launches)), `fork` inherits it, and the
> sentence is now in three places because it is the rule that keeps *same
> place, fresh agent* from quietly becoming *clone*.

> **There is no `--goal`, and there cannot be.** Spawning four agents on a
> goal is free — the goal is `argv`, after `--`, exactly as it always was.
> *Redirecting* four already-running agents onto a goal is out of scope by
> construction: tally attaches as an observer and never holds the control
> lease, so it cannot type into a live PTY and must not learn how.

---

## M15 — The dead pane names the conversation

[M7](#m7--where-views-live) gave a killed view a corpse you can read, and
`docs/19` put anchor's session id on it. **It is the wrong id.** An anchor
session is a PTY that is now gone; what you want after killing an agent is
the *conversation*, and `anchor attach <ulid>` resumes nothing
([docs/26](docs/26-the-claude-session-id.md)).

Scope: `CLAUDE_CODE_SESSION_ID` → `tl.claude` → `@tally_claude` → the
`remain-on-exit-format` line. Four hops, each of them something tally
already does with `tl.attention` or `@tally_session`, and **nothing new
asked of anchor.**

Exit:
- ✅ the id rides the attention hook rather than a `SessionStart` hook of
  its own: the expensive half is finding the session, and a second hook
  would pay a `ps -A` and a `list` again, per turn, for a value that
  changes approximately never
- ✅ an unset `@tally_claude` takes the conditional's false branch and
  expands to nothing, so a view of a plain shell keeps the line `19`
  shipped
- ⬜ on the live grid: spawn a pad, press it, let one turn happen, hold
  Stop, and read the dead tab

> **The order was wrong in the first draft, and 80 columns said so.**
> `19`'s rule is *payload early, because truncation eats the tail*, and the
> draft kept the anchor id first on the grounds that it is the half that is
> always there. Half a UUID resumes nothing; half an anchor id costs
> nothing, because it names a PTY that is gone by the time you are reading
> the line. The conversation goes first.

---

## M16 — The arts

The only milestone here that is not about work, and the second reason is
the real one.

**It is a demo.** The pitch is *the agents you are running are a physical
object on your desk*, and that pitch is made by a photograph. Eleven lit
pads is a true picture of a working day and a bad picture of what the
hardware can do.

**It is a pixel test nobody has to remember to run.** Every piece is a
*total* function of the surface — all 64 pads, every frame. A dead pad, a
mis-flipped row, a diff that stopped converging, a palette entry that is
not the colour the manual says: all visible inside two seconds of plasma,
and none of them visible on a grid where eleven pads are lit and the other
fifty-three are correctly dark ([docs/21](docs/21-the-arts.md)).

Scope: `Drums` enters; the right column becomes a menu of nine pieces;
`►` is the only way out and still goes to whatever is waiting; every other
press is swallowed. The logo keeps telling the truth, because while the
grid is a plasma field *all* attention is attention you cannot see.

Exit:
- ✅ the swallow is read from the **raw button**, before the corner's
  double-tap clock and before `chords.feed` — routing on the gesture let
  `CC19` toggle Stop/Move behind a plasma field, a mode change with nothing
  on the surface able to report it
- ⬜ it survives contact with someone who has not seen it before, which is
  the only test a demo has

> **Seventeen seconds to the punchline, and nobody waited.** The Uprising
> ends on a yellow smile. Everything worked, every test was green, the
> renderer drew the face correctly at frame 759 — and the first person to
> press the button never saw it, because nobody holds a button they have
> just discovered for eighteen seconds. The pacing is asserted in
> **seconds** now, not frames, because seconds are the unit the complaint
> was made in.

> **It is the largest file in the crate, and that is worth saying out
> loud.** `src/art.rs` is bigger than `model.rs` and bigger than
> `jump.rs` — sixteen percent of the code, for nine animations behind a
> button. Both arguments above are real and neither is an argument for it
> growing. **The reel is closed at nine.** A tenth piece is a button taken
> from something that works, and the bar `M10` set applies to pixels too.

---

## What is next

Nothing here is a build item. Everything that was one is done, and what is
left is the half only use can close.

**The gate, unchanged since [M1](#m1--the-grid-tells-the-truth):** a week
of `attention.jsonl`, and a number out of it — how many ambers sent you to
a session that did not want anything. [M6](#m6--the-amber-exists) is what
made the number possible by giving the signal a source that fires. The
journal currently holds a little over a day. **Nine milestones have shipped
on top of a signal whose trustworthiness has never been measured to the
standard this document set for it**, and that is the honest statement of
where the project stands.

Four other weeks are owed, and they are cheap because they are already
being recorded:

| what | read it from | what it decides |
|---|---|---|
| is `►` reached for, and how deep is the queue | `kind:"nav"`, `via:"next"` | if `queued` is almost always 1, the 61% that justified [M8](#m8--the-top-row-is-a-queue) was one unusual hour, and [#8](https://github.com/alehatsman/tally/issues/8)'s grouping argument gets its second look |
| is the map reached for, and does `lit` move off seven | `via:"overview"` | if `lit` sits at seven for a month, the follow-up is not a better map, it is a **smaller address space** |
| is `>` used at all | `tally purge` / shell history against nav lines | if the shell still wins, the launch column has again been given work it does not want |
| does the corner ever sit on Move | the mode in the daemon log | if it does not, [M12](#m12--the-corner-carries-two-verbs) spent a double-tap on nothing and solo gets its second look |

**Two specs are written and unbuilt**, and both are deliberately held:

- [28](docs/28-the-level-and-the-edge.md) — `claude agents --json` as a
  *veto* over a stale amber, never a source of one. Held on the week above,
  because a veto changes what gets recorded and measuring the architecture
  [14](docs/14-attention.md) specified is worth more than measuring it
  already patched.
- [25](docs/25-the-squad.md) — a squad is a ledger file, the row is its
  face. **It does not fit [01-scope](docs/01-scope.md)'s six bullets**, it
  spends the last free button, and it carries seven open questions. That is
  not a reason it is wrong; it is the reason it needs a decision about what
  this product is before it needs an implementation.

**And the rule this section exists to restate.** The sequencing principle
at the top of this file — *no milestone starts before the previous one's
exit criteria are met* — has not held since M9. M10 through M16 all shipped
with ⬜ in them. That was affordable while the ⬜ were perceptual gates a
day of use would close anyway. It stops being affordable at the one gate
that has been open since M1.

---

## Later, unscheduled

- ~~the `settings.json` line for the `Notification` hook~~ — **M6**. It was
  never *later*; it was the only thing that could make the attention half
  of this product exist
- teleport URL as a secondary jump target
- a second device — only with a second device in the room
- **peek** — *what is this pad?* A real hole: the surface cannot name a
  pad, and some share of 341 daily jumps are identification rather than
  work. It is the only idea in [22](docs/22-the-mode-button.md) that needs
  machinery it does not have — a sink to print into. Deferred, not dead;
  holding Stop answers part of it for free
- **solo**, if a week of Move shows people arranging and still being
  dragged away ([22](docs/22-the-mode-button.md))

---

## Sequencing

```
M0  prove the manual  ← is the hardware what the PDF says
M1  the grid tells    ← the only real risk: is the signal trustworthy
M2  jump              ← the payoff gesture
M3  spawn + chords    ← the launcher half
M4  pages             ← ergonomics
M5  tally spawn       ← the folder comes from a shell, not a file
M6  the amber exists  ← M1's risk, answered: the signal was absent
M7  where views live  ← a view is a pane; #1 #2 #3
M8  the top row queue ← 61% of the time there is no *the* amber pad
M9  the tempo         ← p50 187s, p90 1076s, and the pad looks the same
M10 the launch column ← the row already knows the folder
M11 the overview      ← 7 rows of 64, and already discontiguous
M12 the two verbs     ← there were no spare buttons left
M13 tally's server    ← a jump was taking your terminal away
M14 fork + --count    ← the folder can come from a pad you can see
M15 the conversation  ← anchor's id resumes nothing
M16 the arts          ← a demo, and a pixel test of all 64 at once
```

M6 is out of order on purpose. It is M1's work, and it is here rather than
there because the thing that proved M1 incomplete was a day of real use —
which is the only instrument that could have found it, and the only one M1
had not yet run.

**M12 through M16 are one night and a morning**, and the ordering above is
when each landed rather than a sequence of risks. By M11 the risky
subsystems were all retired, so the principle at the top of this file —
*the risky subsystem first, the ergonomics last* — had done its job and
stopped constraining anything. What replaced it as the constraint was the
surface running out of buttons, which is why M12 is the one that reads
like a trade.
