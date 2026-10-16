# 01 — Scope

tally is **a control surface for anchor sessions**. The closed list:

1. **slot** — a durable binding from one pad to one thing you run
2. **spawn** — press an unbound slot, get a session
3. **jump** — press a bound slot, end up looking at its terminal
4. **state** — the pad's colour says what the session is doing
5. **attention** — a session that needs an answer says so, loudly
6. **a few destructive gestures** — kill, resume — behind a deliberate chord

Six bullets. Everything not on that list belongs somewhere else.

## How to reject a feature

Ask which of the six it is. If the answer needs a conjunction, it is not one
of them.

**"Let me bind arbitrary shell commands to pads."** No. That is a Stream
Deck, and the configuration surface it needs (per-pad command, args, icon,
confirmation, output handling) is larger than the rest of this program. A
slot runs an *agent preset*, and presets live in anchor's callers, not here.

**"Show the agent's output on the grid."** No. Eight by eight monochrome-ish
pads is not a display. The grid says *which* session wants you; your
terminal says what it wants. The one exception is
[text scrolling](05-interaction.md#the-title-scroll), which is a deliberate,
transient, user-requested overlay and not a rendering target.

**"Support the Launchpad X / Pro / APC / any other grid."** Not until there
is a second device in the room. The surface layer is already an internal
boundary (`docs/02-surface.md`); generalising it before a second
implementation exists is guessing at the shape of the abstraction.

**"Make it work without anchor."** No. The session model *is* anchor's
model. A tally that spawns its own processes would need a PTY layer, a
log, a lease and a restart story — all of which already exist, one
directory over, with a test suite.

**"Remote control over the network."** No. The device is on the desk, by
USB. If you are not at the desk, you are on your phone, and that is
[teleport](07-seams.md#teleport).

**"Let it hold the control lease so I can type from the pads."** No. One
writer per PTY is anchor's invariant, and the writer is the terminal you
jumped to. tallyd attaches as an **observer**, always
([04-signals.md](04-signals.md)).

## The boundary, stated plainly

**If `tallyd` dies, nothing happens.** No session is affected; the lights go
stale and then, when the next process takes the port, correct. That is the
property worth protecting: this is the most disposable component in the
stack, and every design decision that trades disposability for cleverness
is the wrong one.

**If the Launchpad is unplugged, nothing happens.** tallyd keeps reconciling
and repaints on reconnect.

**If `anchord` is down, the grid goes dark rather than lying.** An empty
session list and an unreachable daemon are different facts — anchor already
makes that distinction for teleport, and the surface must show it (a dim
red logo, not sixty-four dark pads that read as "nothing running").
