# Roadmap

**What shipped, what each one cost to find, and what is still open.** The
design is in [docs/](docs/); this is the record of the order it arrived in
and the measurements that decided it.

Each milestone retires one risk and is independently abandonable, ordered
on a principle borrowed from anchor: **the risky subsystem first, the
ergonomics last.** The risk sat in an unusual place here — the hardware
was documented but unverified, and the *attention signal* is the thing the
product lives or dies by. The launcher was the easy half.

> **Read [What is next](#what-is-next) before anything else here.** The
> rule that no milestone starts before the previous one's exit criteria
> are met stopped holding at M10: everything since has shipped with open
> criteria, and one of them has been open since M1. That section says
> which, and what closes them.
>
> **[The finish line](#the-finish-line) says in what order**, and what
> v1.0 is allowed to mean. It is the only part of this file that is a plan
> rather than a record.

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
> **Two things this project asserted are false**, and neither was
> Novation's fault: the port is **not** exclusive on macOS — CoreMIDI
> multiplexes, so `tallyd` carries its own single-instance lock — and the
> SysEx overhead is 8 bytes per frame, not 7.
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

> **Two perceptual gates cost a build each, and both answers are now
> rules in [docs/05](docs/05-interaction.md).** A row lighting the current
> page white and occupied pages turquoise was legible and unreadable —
> *"I am confused which one is selected"* — which is where *brightness
> answers where-am-I before hue answers anything* comes from. And a title
> scrolled across eight pads is about two legible characters at a time,
> which is where *the pad cannot say what it wants* comes from.
>
> Neither build was wasted: each bought a measurement nothing else could
> take. That is the milestone's real output
> ([09](docs/09-testing.md#what-a-green-suite-does-not-prove)).

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

> **The only thing that knows the directory is a shell standing in it.**
> `slots.toml` wants the folder declared in advance, which is a prediction
> about work you have not done — so M3's spawn-from-a-dark-pad went unused
> from the day it shipped. Not broken; aimed at the wrong context, and
> M3's machinery is exactly what `tally spawn` calls.
>
> **The deferral is what made that cheap.** *"Defer until something is
> actually awkward"* ([docs/10](docs/10-open-questions.md)) meant the CLI
> was built around a measured gap rather than a guessed one, and it came
> out one subcommand wide. A feature is settled by use, not by design, and
> this is the first time that cost nothing to act on.

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

> **A promise with no implementation behind it, found by grep.**
> `docs/04` had tallyd clearing `tl.attention` on output; the codebase had
> a constant, a reader, a test, and nothing that removed it. A hook
> shipped against that binary would have stuck a pad amber permanently.
> The suite could not have caught it — **there is no test for a feature
> nobody wrote.** Both edges are hooks now, and both ship here.

---

## M7 — Where views live

Not three fixes but one, and the three open issues are all downstream of
the same definition: **a view is addressed as a pane**
([docs/15](docs/15-views.md)). Calling it a *window* is only correct while
`tallyd` under launchd is the one thing creating views, and that special
case had become the definition.

Change the unit and the three collapse:
[#1](https://github.com/alehatsman/tally/issues/1) gets a unit it can
adopt, [#2](https://github.com/alehatsman/tally/issues/2) gets a pane that
does not close under you, and
[#3](https://github.com/alehatsman/tally/issues/3) gets a coordinate that
is the whole address rather than most of it.

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

> **The pane that survives its agent is what the other two are built
> on.** A view you can adopt needs it; a return gesture needs a pane-level
> coordinate to return to. So #2 is the step that goes first, and not as a
> compromise.

> **A spec validated against the code and not against the other specs
> will collide with them.** Steps 2 and 3 did: the second tap on a pad was
> already spoken for, and the address format one of them assumed might
> have been replaced. Both were released by
> [docs/16](docs/16-the-queue.md) rather than by rework — the return
> gesture has its own button and addressing never changed — but the
> collision cost a day of not knowing, which is what an open design
> question charges everything downstream of it.

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

> **What it costs: the current-page readout.** The rule it deletes was
> paid for in real confusion — *exactly one page button is bright and it is
> the one you are on* — and the defence is one number, **pages used,
> 70 : 2**. If pages ever become load-bearing this is the first decision to
> revisit ([docs/12](docs/12-the-layout.md)).


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

**The number of beats was right and the number of pads was wrong.**
Time-weighted over 10.1 h the mapping behaved exactly as designed — 61% at
the floor, 19% in the 120-140 band — so the tempo was never the defect.
One global clock driving *four unrelated green pulses* to 140 bpm on
behalf of one ignored pad was.

So **green stopped moving**, and motion is spent only on amber. Confirmed
on the device — *"green are not pulsing no, orange yes"* — with four slots
asking at the time, so the amber half was live and checked rather than
assumed. The surface is now still unless something is asking, and the one
clock modulates only the pads that are actually waiting. The cost: working
and quiet look identical, and *which agent is producing output* is no
longer visible — information it turned out you watch rather than act on
([docs/05](docs/05-interaction.md#nothing-blinks-and-only-attention-moves)).

> **A clock costs one byte, forever, and buys a liveness indicator.** It
> gives back one third of what [docs/02](docs/02-surface.md#colour) sells
> the palette animations on — *no timer, no repaint loop, no drift* — and
> keeps the important third: the **ramp** still runs on the device, so a
> pulse is smooth under load. What it buys is that a wedged `tallyd`
> stops breathing mid-ramp instead of leaving a normal-looking grid
> showing a stale snapshot, which is the worst failure a status display
> can have because it is indistinguishable from *nothing changed*.
>
> The floor is 60 bpm and not 30 for the same reason: a very slow breath
> and a stopped one have to be tellable apart. The ceiling is 140 because
> **a surface that is correct and exhausting is a failure this project has
> already paid for once**, and a tempo that sprints is that mistake with a
> different knob ([docs/17](docs/17-the-tempo.md)).

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

> **Arguments are dropped, and the rule outlives this button.** A row
> running `claude --resume d37ee7e5…` describes one conversation, and
> copying that would put a second pad on it — the one thing a new agent
> must not be. Keeping `command` keeps the choice that matters: a row
> running `isayes` gets another `isayes`. `fork` inherits the same rule in
> [M14](#m14--tally-fork-and-four-at-a-time).

> **Zero presses in a day is not an argument that a gesture is
> unneeded.** It is an argument that the thing it guards has not failed
> yet. Mute scored zero and keeps existing because it is the starvation
> hatch for `►` and the tempo; purge scored zero and keeps existing
> because `anchor` has no `purge` subcommand (anchor#14), so deleting it
> would leave rows accumulating with no remedy anywhere. Both moved to the
> shell rather than off the map ([docs/18](docs/18-the-launch-column.md)).

> **One row behaves differently, and it is a rule rather than a gap.**
> Eight buttons, eight rows, one of which must be the kill modifier — and
> moving kill means moving the one button that is 8 mm from the wrong pad,
> in the corner the silkscreen labels *Stop*. A grid you remember rather
> than feel is affordable here only because the window scrolls: row 8 is
> one `▼` from being row 7.

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

> **The clock stays out of `chord.rs`.** It is pure — presses in,
> gestures out, no `now` — and that purity is what lets the four chord
> rules be tested without a device or a timeline. Timed gestures live in
> `tallyd`'s 16 ms loop, which already has `elapsed` on the line that
> calls `feed`. The consequence, stated so nobody looks for it:
> `Gesture::Home` is in the enum and `chord.rs` never emits it.

> **The overview is momentary, not latched.** The top row is built on
> *dark does nothing, lit does one thing, there is no state to remember*,
> and a latched overview makes the grid mean two things depending on
> something invisible. ([M12](#m12--the-corner-carries-two-verbs) does
> latch one button, and the difference is that the grid does not change
> until you hold it.)

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

> **Merging kill and purge widens the blast radius, and that is the
> price.** Purge's targets include pads that draw dark, so a thumb one pad
> off forgets a dead row's log where before it did nothing. What makes it
> affordable is that **the hold is what reveals them** — they stop being
> invisible at the exact moment they become targets — and that the two
> target sets are disjoint, so every pad under the thumb has exactly one
> outcome and one colour. Priced at one order below killing a live agent,
> which is why purge wears the dark-red *consequence* colour rather than
> kill's red.

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

> **A column is a tab, because eight of them side by side is
> unreadable.** Pane-per-column shipped, measured and correct, and a row
> of eight agents was eight 23-column slivers reflowing megabytes of
> scrollback. The addressing was never wrong; the reading was. **A split
> shows you all eight at once when the thing you do with a row is read
> one of them**, and that sentence is the whole of it
> ([docs/24](docs/24-the-tally-server.md)).

> **A window index is a namespace, and a corpse occupies one.**
> `remain-on-exit` keeps a dead view's pane and therefore its index, so
> killing a session and spawning a new one on the same pad gives
> `create window failed: index 4 in use` — and the grid has no way to show
> you an error. The identity lookup was right; what was missing is that
> **the address is also a lookup**. `@tally_slot` had been written on
> every pane since this milestone and never read back
> ([docs/27](docs/27-the-slot-collision.md)).
>
> Fixing it caught a second thing: `back()` parsed the same pane listing
> with its own `split_whitespace`, so changing the separator would have
> made every origin look closed and `◄` silently stop working. **Every
> read of that listing goes through one function**, which is the only
> reason a format change is safe to make.

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

> **`fork` is *same place, fresh agent*, never *clone*.** It drops `args`
> for the reason `>` does ([M10](#m10--the-right-column-launches)): the
> argv of a resumed pad names one conversation, and two pads on one
> conversation is the thing a new agent must not be.

> **There is no `--goal`, and there cannot be.** Spawning four agents on a
> goal is free — the goal is `argv`, after `--`, exactly as it always was.
> *Redirecting* four already-running agents onto a goal is out of scope by
> construction: tally attaches as an observer and never holds the control
> lease, so it cannot type into a live PTY and must not learn how.

---

## M15 — The dead pane names the conversation

[M7](#m7--where-views-live) gave a killed view a corpse you can read.
What it needs to say is the **conversation**: an anchor session is a PTY
that is already gone by the time you are reading the line, and
`anchor attach <ulid>` resumes nothing
([docs/26](docs/26-the-dead-pane.md)).

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

> **The conversation goes first, and 80 columns is why.** The rule is
> *payload early, because truncation eats the tail*
> ([docs/26](docs/26-the-dead-pane.md)). Half a UUID resumes nothing; half
> an anchor id costs nothing, because it names a PTY that is already gone
> by the time you are reading the line. The obvious ordering — the id that
> is always there first — is the wrong one.

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

> **Nobody holds a button they have just discovered for eighteen
> seconds.** The Uprising ends on a yellow smile and the renderer drew it
> correctly at frame 759 — which is seventeen seconds in, and the first
> person to press the button never saw it. The pacing is asserted in
> **seconds**, not frames, because seconds are the unit the complaint was
> made in and frames are the unit that hid it.

> **It is the largest thing in the crate, and that is worth saying out
> loud.** `src/art/` is bigger than `model.rs` and bigger than `jump.rs` —
> sixteen percent of the code, for nine animations behind a button. Both
> arguments above are real and neither is an argument for it growing.
> **The reel is closed at nine.** A tenth piece is a button taken from
> something that works, and the bar `M10` set applies to pixels too.
>
> It became a directory in [29](docs/29-what-the-loop-holds.md) — one file
> per piece, three for what they share. That is a smaller unit to read, not
> a smaller amount of code, and it does not reopen the count.

---

## M17 — The bridge

The Launchpad is on a Windows desk and the agents are in WSL, and USB
does not cross that line without a kernel build. Cut the process instead:
`tallybridge` owns the device on Windows, `tallyd` owns everything else
in WSL, and raw MIDI runs between them
([docs/30](docs/30-the-bridge.md)).

**This milestone was specified on a Mac with no Windows hardware in
reach.** Five facts it rests on have never been run. That is why M17.0
exists and why it comes before a line of the codec.

### M17.0 — Answer the five, before building

No code. Each row is one command and one observation, and two of them can
move the design.

| probe | how | why it can move the design |
|---|---|---|
| the WinMM port name | `tallybridge --list` — a dozen lines of `midir` enumeration, the only Windows code written before the answers | [docs/02](docs/02-surface.md#two-ports-and-which-one-we-want) claims Windows decorates it with nothing. If `PORT_SUBSTRING` does not match, the substring rule is wrong on one platform and that is a `device.rs` change, not a bridge change |
| the bind address | `nc -l 127.0.0.1 7373` in WSL, reach it from Windows; repeat on `0.0.0.0` | if `0.0.0.0` is required the socket is on the local network, unauthenticated, driving the grid. **Stop and raise it** — that is a product decision, not a config line |
| interop | `powershell.exe -c "echo ok"` from WSL | if it is off, `raise` goes back to the reserved `0x10` control frame and the bridge grows a second job |
| `AppActivate` | raise the terminal from WSL while another window has focus | if Windows refuses the foreground steal more often than not, `raise` is not worth its code on this platform |
| clock over the hop | after M17.4, not before | if the breath stutters, the tempo moves to the bridge and the wire carries a BPM instead of pulses — the largest possible change here |

Exit:
- ⬜ five answers, written into [docs/30](docs/30-the-bridge.md) in place
  of the ⚠️ rows, each one a ✅ or a correction
- ⬜ the bind-address answer reported before anything is built on it

### M17.1 — The codec

`src/bridge.rs`: kind, big-endian length, payload; 64 KiB cap. One
definition, compiled into both binaries. Pure, no I/O, no sockets.

Exit:
- ⬜ every kind round-trips
- ⬜ a frame split across two reads reassembles — the test that matters,
  because TCP will do this and a local socket in a test usually will not
- ⬜ a truncated frame is not a message and does not block
- ⬜ an oversize `len` is a hang-up, not an allocation

### M17.2 — `Device` becomes a trait

`Local` is today's struct renamed and otherwise untouched. `poll` becomes
`-> Vec<Event>` because `impl Iterator` is not object-safe; `Link` holds
`Box<dyn Device>` and `Link::connect` picks the impl from
`TALLY_BRIDGE_LISTEN`.

Exit:
- ⬜ the suite is green with no test changed except the `poll` call shape
- ⬜ on a Mac with a Launchpad, behaviour is bit-identical — no listener,
  no socket, no new code on that path

### M17.3 — `Remote`, against a fake bridge

A fake bridge in-process, so this is testable in WSL with no hardware.

Exit:
- ⬜ the identity handshake completes over the pipe inside `REPLY_TIMEOUT`
- ⬜ a press arrives as exactly one `Event` — including two presses
  written back-to-back, which is where coalescing would eat one
- ⬜ `render` diffs exactly as `Local` does; the same frame sequence puts
  the same bytes on the wire
- ⬜ `{"device":"gone"}` lands `Link` in the same state `NotFound` does

### M17.4 — `tallybridge`

The Windows binary: enumerate, open, pipe both ways, `TCP_NODELAY`,
reconnect forever, and restore Live mode on every way out.

Exit:
- ⬜ a pad lights from WSL
- ⬜ a press in Windows reaches a WSL chord
- ⬜ **kill the bridge with `taskkill /F` and the Setup menu still
  works** — the restore is the one thing in this milestone that cannot be
  argued, only run
- ⬜ unplug and replug the Launchpad: `gone`, then `present`, then a full
  repaint, with `tallyd` never restarted
- ⬜ restart `tallyd`: the bridge reconnects on its own

### M17.5 — Raise, by interop

`raise()` gains `TALLY_RAISE_CMD`, split on spaces, default unchanged.
No bridge involvement.

Exit:
- ⬜ a pad press brings Windows Terminal forward from behind another window
- ⬜ a refused raise is silent and the pane still switched

### M17.6 — A day beside it

Not a formality, and the only instrument that has ever caught anything
here. The hard amber flash and the green working pulse both passed a
glance on real hardware and failed a day of use
([M9](#m9--the-tempo-says-how-long)), and this milestone adds a network
hop to the surface those were measured on.

Exit:
- ⬜ a working day on the Windows desk with no restart of either binary
- ⬜ the breath is still legible — or the clock moves to the bridge, which
  is M17.0's last row coming due

> **The bridge may not grow.** Every argument for putting one more thing
> on the Windows side is an argument for a second implementation of tally,
> and the second one has no tests. If something seems to need to live
> there, it is a sign the seam is in the wrong place — move the seam, in
> a document, before moving the code.

---

## M18 — A moved session takes its view with it

Move rewrote a label and left the pane where it was, so the agent stayed
at its old address and the pad it vacated failed with
`create window failed: index 1 in use`. One cause, two symptoms
([docs/31](docs/31-the-moved-view.md), [#15](https://github.com/alehatsman/tally/issues/15)):
the address is a tmux window and Move only ever wrote the name.

Scope: `tallyd` follows a move with `relocate`, and `view_pane` runs the
same check lazily by id, so a move made while `tallyd` was down heals on
the next press. `put_down` still writes exactly one label.

Exit — the first three were run on the **live grid** on 2026-10-18; the
fix had until then only ever seen a scratch socket:
- ✅ a move within a row leaves the view at the window its label names
  (`1:3:2 → 1:3:1`, `1:3:3 → 1:3:4`)
- ✅ a move to another row that already exists (`1:3:1 → 1:2:1`)
- ✅ no `index N in use` **caused by** any of those three moves
- ⬜ a move to a row that does **not** exist — the placeholder-session
  branch, the riskiest thing in the change, because tmux cannot adopt a
  pane into a session it creates. The cross-row move above went to a row
  tmux already held, so this has still only run on a scratch socket
- ⬜ a move onto a corpse, on purpose; whether the one above met one is not
  recorded
- ⬜ a spawn onto a pad a move vacated, on the live grid
- ✅ the lazy by-id relocation, on a view out of place: pressing `1:1:3`
  moved `%31` from `r01:2` home to `r01:3`, and no collision followed.
  **Inferred, not seen** — the relocation is not logged, so this is
  `list-panes` before and after
- ❌ a press of a pad whose window a drifted view holds: `index 2 in use`
  twice, then healed by pressing the view that was out of place — **a
  different pad's address**. It did not heal itself: pressing only the
  blocked pad fails every time, and nothing looks for the drift from
  there ([#22](https://github.com/alehatsman/tally/issues/22)). Not a
  Move, and the same wedge
  ([#20](https://github.com/alehatsman/tally/issues/20))
- ❌ a move onto a **live** pane — [#19](https://github.com/alehatsman/tally/issues/19).
  Not tried; it happened anyway, and the view was re-tagged where it
  stood. The spec had admitted this case and left it

> **The gate ran, and the most useful thing it found was not in the
> spec.** The owner's `~/.tmux.conf` sets `renumber-windows on`, and tally's
> server is a separate *server* but not a separate *config*. Closing one
> window in a row shifted its siblings down, so a view was at the wrong
> index with no Move involved, and the lazy relocation met the one branch
> the spec leaves broken. [docs/24](docs/24-the-tally-server.md) records
> *"indexes do not renumber when a sibling is killed"* as a measured ✅; it
> was measured on a default tmux and written down as a property
> ([#20](https://github.com/alehatsman/tally/issues/20)). Every unit
> test was green throughout, because a fixture listing is typed by a
> human and cannot be wrong about what tmux does
> ([#21](https://github.com/alehatsman/tally/issues/21)).

> **A count over a whole log is not a measurement of a change.** *"77
> collisions, then none"* is the sentence that wants writing, and it is
> wrong twice: 77 is every `in use` the daemon ever logged, mostly the
> corpse case [docs/27](docs/27-the-slot-collision.md) fixed earlier; and
> the last of them is before the *previous* restart, so the quiet that
> follows says nothing about this build. What the run supports is
> narrower: three moves, none followed by a collision, against a before
> where one move in the log was followed by three. And two collisions that
> no move caused, on the same grid, within the hour. The other shape of the same mistake is
> *"all panes aligned"*, which was true at one moment and false within the
> hour for a reason no move caused.

> **What is open is on the tracker, not in a PR description.** The
> blocked case ([#19](https://github.com/alehatsman/tally/issues/19)), the
> renumbering ([#20](https://github.com/alehatsman/tally/issues/20)), the
> harness that was never committed
> ([#21](https://github.com/alehatsman/tally/issues/21)) and drift nothing
> looks for ([#22](https://github.com/alehatsman/tally/issues/22)) were
> all named in #17 and filed nowhere, which is the failure this file's
> [What is next](#what-is-next) describes, one level down.

---

## What is next

Nothing here is a build item. Everything that was one is done, and what is
left is the half only use can close. [The finish line](#the-finish-line)
takes what follows and puts it in an order.

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

## The finish line

**What is above this line is a record. This is the only part that is a
plan.** [What is next](#what-is-next) says what is owed; this says in what
order it gets paid and what "finished" is allowed to mean.

Finished is **v1.0**, and it is four statements, none of which is "every
written spec is built":

1. the amber has a **measured number**, the one open since [M1](#m1--the-grid-tells-the-truth)
2. a jump never wedges, and a test in this repo can see tmux
3. a stranger can clone, build, install and run it
4. the tracker holds **no undecided design**

The phases are not milestones. F0 runs in the background of all of them,
F1 is the critical path, F3 is independent and is the one that will be
skipped if nobody writes it down.

### F0 — Measure the amber

The gate, unchanged since M1 and restated here only to give it a place in
a sequence. Seven days of `attention.jsonl`, uninterrupted, and the
number: **how many ambers sent you to a session that wanted nothing.**

Alongside it, the four questions in [What is next](#what-is-next)'s table,
which are already being recorded and cost only the reading.

Exit: a dated paragraph with five numbers in it. **If the false-amber rate
is bad, stop and fix the signal** — [14](docs/14-attention.md) said so
first, and nine milestones are stacked on the assumption it is fine.

### F1 — Close the jump subsystem

The only place the product breaks in ordinary use. Four issues, all with
repros, and the order matters because each step is the instrument for the
next.

| | | |
|---|---|---|
| 1 | [#21](https://github.com/alehatsman/tally/issues/21) | `tests/relocate.rs`: scratch `-L` socket, explicit `-f`, **both configs** — default and `renumber-windows on`. First, because the other three cannot be verified without it. The issue already has the shape, and `Tmux::with_anchor` is already `pub` |
| 2 | [#19](https://github.com/alehatsman/tally/issues/19) | decide the live-occupant branch. Whatever the verdict: **stop `tag()`-ing a pane with an address its window does not hold**, and log the refusal. Cheap, and it is what makes #22 observable |
| 3 | [#22](https://github.com/alehatsman/tally/issues/22) | detection — resolve-the-squatter-by-tag at `make_view`, and/or the startup sweep. The sweep is the only one of the two that yields a drift *rate* |
| 4 | [#13](https://github.com/alehatsman/tally/issues/13) | the silently deadlocked pad. Option 1 — document the `printf '\r' \| anchor attach` recipe — ships today for free; option 4 is the honest fix |
| 5 | — | **re-run [M18](#m18--a-moved-session-takes-its-view-with-it)'s gate on the live grid.** Four ⬜ and two ❌ remain, and the ❌ were caused by [#20](https://github.com/alehatsman/tally/issues/20), which is now fixed and has never been re-tested against them |

Exit: M18 carries no ⬜ and no ❌, and `cargo test` exercises real tmux
under both configs.

### F2 — Make the documents true

Cheap, and runs alongside F1.

- **[#23](https://github.com/alehatsman/tally/issues/23) first, and it has
  been promoted.** It was filed as *"not blocking anything"*. It now blocks
  the workflow the owner actually runs: agents read `tally ls` and spawn
  their own children off it, which is the habit that
  [closed the squad](docs/25-the-squad.md). `tl.attention` means *blocked*
  and *finished* with one bit, `Stop` fires every turn and `Notification`
  rarely, so the common case is the one the word is wrong about. The amber
  does not change; the word does.
- **`tally ls` output is an interface now.** Programs parse it. It needs a
  statement in [32](docs/32-the-grid-in-text.md) about what is promised and
  what may move, or every column rename is a silent break.
- **[#14](https://github.com/alehatsman/tally/issues/14)** — `11-cli.md`
  still says a failed pad refuses a spawn and points at a grid affordance
  for `purge` that does not exist. The issue asks for a pass over the
  surrounding text rather than the one line, because a claim that drifted
  has neighbours.
- **This file and the README** — both still say *seventeen milestones*, and
  the README still lists the squad as a live spec.

### F3 — Release engineering

**The largest structural gap, and nothing in the repo does any of it.** No
`.github/`, no `tests/` directory, `version = "0.0.0"`, `publish = false`.
`clippy.toml` and `deny.toml` exist and no automation reads them.

- CI: `cargo test`, `clippy -D warnings`, `fmt --check`, `cargo deny`
- **F1's tmux test must run in CI, not skip there.** Skip-if-absent keeps
  it from breaking a contributor; a green run that did not run it is the
  exact pattern this file complains about everywhere else
- a version, and a decision: crates.io, or a tag plus
  `cargo install --git`. It drives one USB device on one OS, so the second
  is probably the right size
- an install story that **includes the `settings.json` hook lines**
  ([M6](#m6--the-amber-exists)). The attention half of this product does
  not exist without them, and today they live only in prose

### F4 — Close scope

A verdict written into each document. Not a build.

| | |
|---|---|
| [#8](https://github.com/alehatsman/tally/issues/8) — the layout | **Narrowed to one experiment.** The 776 lines it tracks are gone: `12-layout-proposal` and `13-layout-review` were merged into [12](docs/12-the-layout.md) in `bad678c`. The owner's read is that the current layout is final. What remains to run is `13` §10 — *light row 4 amber, stand where you stand, say "four" without counting* — and then the sentence |
| [25](docs/25-the-squad.md) — the squad | **Closed.** Superseded by `tally ls` and an agent that spawns its own members. No button spent, no ledger, seven open questions moot |
| [28](docs/28-the-level-and-the-edge.md) — the level | Held on F0 **by design**: a veto changes what gets recorded, and measuring the architecture [14](docs/14-attention.md) specified is worth more than measuring it already patched. Once the number exists: build, or close |
| [30](docs/30-the-bridge.md) / [M17](#m17--the-bridge) — the bridge | Specified, unbuilt, five probes never run, and no Windows hardware in reach. **Mark it deferred-until-hardware** rather than leaving it reading as a pending milestone |
| [23](docs/23-gesture-brainstorm.md) — the brainstorm | Say plainly that it is a graveyard, or fold the survivors into #8's verdict |

### F5 — v1.0

Tag it. The README's Status section becomes *shipped, measured and closed*,
**with the false-amber number in it** — which is the sentence this project
has owed since M1 and the only one that was never a matter of effort.

### Sequencing

```
F0  measure the amber   ─────────────────────────────────▶  (7 days, in the background)
F1  the jump subsystem  #21 → #19 → #22 → #13 → M18 re-run     critical path
F2  the documents       #23 → ls-is-an-interface → #14 → README
F3  release eng         CI → version → install                 independent, start any time
F4  close scope         gated on F0's numbers
F5  v1.0                needs all four
```

**The honest risk in this ordering is F3.** F1 is specified work with
repros attached and will get done because it hurts. F3 is the one that
decides whether this is a product or a personal rig, and nothing about
using it every day will ever make it urgent.

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
