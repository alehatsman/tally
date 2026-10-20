//! `tallyd` — the daemon that owns the Launchpad.
//!
//! The reconcile loop: poll anchord, start watchers, render, repeat.
//!
//! An unreachable anchord lights the logo dark red and **leaves the grid
//! alone**, because "nothing is running" and "I cannot tell" are different
//! facts (`docs/01-scope.md`).

use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use tally::act::{Context, Done};
use tally::anchor::Client;
use tally::art::{Piece, Reel};
use tally::chord::{Action, Chords, Gesture};
use tally::config;
use tally::device::{Device, Error, Event};
use tally::journal::{self, Journal};
use tally::jump::{Jump, Tmux};
use tally::lock::Lock;
use tally::model::{self, View};
use tally::queue;
use tally::signal::SessionState;
use tally::slot::{self, Pages, Slot};
use tally::surface::{Button, Frame};
use tally::tempo;
use tally::watch::Watchers;

/// Set by the signal handler, read by the main loop. `0` means no signal.
static SIGNALLED: AtomicI32 = AtomicI32::new(0);

/// How often the loop asks anchord for the session list. teleport's
/// cadence for the same job; presses are not on this clock.
const RECONCILE: Duration = Duration::from_secs(2);

/// How often the drift sweep re-runs when nothing else prompts it.
///
/// Picked against the decision it feeds, not against the clock: a week is
/// ~670 lines, enough to read a rate off and few enough to read. Drift is
/// caused by renumbering, hand-moved tabs and swaps — none of which
/// happens on a sub-minute scale — and the sweep costs one `list-panes`,
/// which a single press already pays
/// ([#22](https://github.com/alehatsman/tally/issues/22)).
const SWEEP: Duration = Duration::from_mins(15);
/// The shortest gap between two sweeps when a reconnect asks for one.
///
/// The reconnect edge sweeps immediately, because that edge is the event
/// that killed seven view panes. But anchord flapping produces one edge
/// per [`RECONCILE`], and the damage is done by the first drop: re-flaps
/// inside this window report the same faults again and would spend a
/// week of [`SWEEP`]'s line budget in an hour. 30s is [`RECONCILE`] x 15,
/// which caps a restart loop at two sweeps a minute instead of thirty.
const SWEEP_FLOOR: Duration = Duration::from_secs(30);
/// How often the loop wakes. Input is drained at this rate so a press
/// feels immediate rather than waiting out a reconcile.
const TICK: Duration = Duration::from_millis(16);
/// How long without a reply before the device counts as gone.
///
/// Generous against the ~2ms warm round trip, because the cost of being
/// wrong is a needless reconnect. The probe rides the [`RECONCILE`] clock
/// (`Link::probe`), so three missed probes is `RECONCILE` × 3 = 6s, and
/// the seventh second is slack for a loop that was busy. Move `RECONCILE`
/// and this number buys a different number of probes.
const DEVICE_SILENCE: Duration = Duration::from_secs(7);

/// How long `▲` must be held before it means *all the way up*.
///
/// `⌂` lost its own button when `Session` became the overview modifier,
/// and it landed here rather than being folded into the overview because
/// the one jump the journal describes — the deep row back to row 1 — is
/// worth a single press (`docs/05-interaction.md`).
///
/// There is no tap-versus-hold ambiguity to resolve: the press scrolls
/// one row immediately and keeping the finger down does more, so nothing
/// is withheld waiting to find out which it was. 500 ms is short enough
/// not to feel like waiting and long enough that a lazy tap cannot reach
/// it — a hardware question, which is why it is a named constant.
const HOME_HOLD: Duration = Duration::from_millis(500);

/// How short a press has to be to count as a **tap** on `CC19` rather
/// than an aborted hold.
///
/// This number is the only thing standing between the mode toggle and
/// chord rule 4 — *releasing a modifier without tapping is a no-op* —
/// which is the escape hatch the whole hold-then-tap grammar rests on.
/// A hold long enough to read a preview is nowhere near 250 ms; a
/// deliberate tap is about 100 ms (`docs/05-interaction.md`).
const TAP: Duration = Duration::from_millis(250);

/// How close two taps on `CC19` have to be to toggle the mode.
///
/// Long enough to be comfortable, short enough that two unrelated
/// presses do not pair into a mode change you did not ask for.
const DOUBLE_TAP: Duration = Duration::from_millis(400);

/// Async-signal-safe: a relaxed store and nothing else.
///
/// The restore is four MIDI writes and a port close, none of which is
/// remotely signal-safe, so the handler does not do it. It wakes the loop,
/// the loop returns, and `Device`'s `Drop` runs on the normal path
/// (`docs/08-daemon.md`).
extern "C" fn on_signal(signum: i32) {
    SIGNALLED.store(signum, Ordering::Relaxed);
}

fn install_handlers() {
    for sig in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        let handler = on_signal as extern "C" fn(i32) as *const () as libc::sighandler_t;
        // SAFETY: `on_signal` only performs a relaxed atomic store, which is
        // async-signal-safe; registering it is `libc::signal`'s whole job.
        #[expect(unsafe_code, reason = "installing a signal handler needs libc")]
        let _previous = unsafe { libc::signal(sig, handler) };
    }
}

/// Re-raise with the default handler, so the process still looks killed to
/// whatever was waiting on it.
fn reraise(signum: i32) -> ! {
    // SAFETY: restoring the default disposition and raising it is exactly
    // what the default would have done had we never installed a handler.
    #[expect(unsafe_code, reason = "re-raising needs libc")]
    let _previous = unsafe { libc::signal(signum, libc::SIG_DFL) };
    // SAFETY: raising a signal whose disposition is now the default.
    #[expect(unsafe_code, reason = "re-raising needs libc")]
    let _rc = unsafe { libc::raise(signum) };
    // `raise` with SIG_DFL does not return for any signal we install.
    std::process::abort();
}

/// The `CC19` double-tap, tracked across presses.
///
/// A tap is a press and a release close together with **no chord fired
/// in between** — which is exactly what distinguishes it from an aborted
/// hold, the gesture chord rule 4 promises will do nothing. Two of them
/// inside [`DOUBLE_TAP`] toggle the mode.
///
/// The clock is here rather than in [`tally::chord`] ([`Grip`]).
#[derive(Debug, Clone, Copy, Default)]
struct Corner {
    /// When it went down, while it is down.
    down: Option<Duration>,
    /// A chord fired during this hold, so the release is not a tap.
    used: bool,
    /// When the last completed tap ended.
    tapped: Option<Duration>,
}

impl Corner {
    /// `CC19` went down or came up. Returns `true` when that completed a
    /// double-tap and the mode should flip.
    fn feed(&mut self, down: bool, elapsed: Duration) -> bool {
        if down {
            self.down = Some(elapsed);
            self.used = false;
            return false;
        }
        let held = self.down.take().map(|since| elapsed.saturating_sub(since));
        // An aborted hold, a hold that did something, or a release whose
        // press never arrived. None of them is a tap, and none may leave
        // a half-double behind for the next one to pair with.
        //
        // The third is not hypothetical: an art swallows the **press**
        // and not the release ([`art_press`] answers `false` for
        // `!down`), so while a piece is playing this is handed a release
        // with no matching press. `is_none_or` on `None` is what catches
        // it, and the test below is what keeps that true.
        if self.used || held.is_none_or(|h| h > TAP) {
            self.tapped = None;
            return false;
        }
        match self.tapped.take() {
            Some(first) if elapsed.saturating_sub(first) <= DOUBLE_TAP => true,
            // First tap of a possible pair, or a second one that came
            // too late — which starts a new pair rather than failing.
            _ => {
                self.tapped = Some(elapsed);
                false
            }
        }
    }

    /// A gesture completed while the corner may have been down. A chord
    /// means this hold did something, so its release is not a tap.
    fn saw(&mut self, gesture: Gesture) {
        self.used |= matches!(gesture, Gesture::Chord { .. });
    }
}

/// The gestures that need a clock, and the pad one of them is carrying.
///
/// Together, because they are all here for one reason: [`tally::chord`]
/// has no `now`, so a gesture that is *timed* or spans more than one
/// press is the loop's to hold (`docs/08-daemon.md`).
#[derive(Debug, Clone, Copy, Default)]
struct Grip {
    /// The `CC19` double-tap.
    corner: Corner,
    /// The pad a held `Move` is carrying.
    hand: Option<Slot>,
    /// Set while `▲` is down, taken when the hold fires.
    home: Option<Duration>,
}

impl Grip {
    /// `▲` went down or came up.
    ///
    /// **Before the art gets a say**, which is deliberate and is the one
    /// ordering here that differs from the corner's: a held `▲` scrolls
    /// the window, the window is not what an art is covering, and the
    /// grid underneath is where you want to land when you leave it.
    fn aim(&mut self, button: Button, down: bool, elapsed: Duration) {
        if button == Button::Top(1) {
            self.home = down.then_some(elapsed);
        }
    }

    /// `CC19` went down or came up, after `chord.rs` has been given the
    /// chance to swallow it.
    ///
    /// Matched on the **raw button** rather than on a gesture, and
    /// deliberately: `feed` answers `None` for plenty of presses, and a
    /// double-tap is completed by exactly such a release. Routing this on
    /// the gesture instead is the bug that let `CC19` reach the clock
    /// from behind a plasma field (`docs/05-interaction.md`).
    ///
    /// Returns whether the surface changed.
    fn feel(&mut self, chords: &mut Chords, button: Button, down: bool, elapsed: Duration) -> bool {
        if button != Action::Stop.button() {
            return false;
        }
        let toggled = self.corner.feed(down, elapsed);
        if toggled {
            chords.toggle_mode();
            eprintln!("tallyd: corner {:?}", chords.mode());
        }
        // Letting go empties the hand ([`tally::act::Done::Picked`]).
        if !down {
            self.hand = None;
        }
        toggled
    }

    /// Whether `▲` has been held long enough to mean *all the way up*.
    ///
    /// Taking the press out latches it: one `⌂` per press, no autorepeat,
    /// and nothing more until the finger lifts and lands again.
    fn home_due(&mut self, elapsed: Duration) -> bool {
        if self
            .home
            .is_some_and(|since| elapsed.saturating_sub(since) >= HOME_HOLD)
        {
            self.home = None;
            return true;
        }
        false
    }

    /// The cable came out. Everything a finger was holding went with it.
    ///
    /// One function rather than a line per field, because the chord state
    /// is split across two types and the halves drifted: the departure
    /// branch cleared [`Chords`] and left the hand behind, so a hold-tap
    /// interrupted by a replug put the carried pad down wherever the next
    /// finger landed.
    fn unplugged(&mut self, chords: &mut Chords) {
        chords.clear();
        self.hand = None;
        // `▲` down when the cable went leaves a hold that would fire a
        // spurious `⌂` on the replug. Same event, same answer.
        self.home = None;
    }
}

/// What a press did to the hand under a held `Move`.
///
/// Returned rather than mutated because [`perform`] has no business
/// owning surface state — it carries gestures out and says what
/// happened, and the loop that owns the hold owns the hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hand {
    /// Not a move, or a move that wrote nothing.
    Unchanged,
    /// A pad came into your hand.
    Took(Slot),
    /// It landed. The hand is empty again.
    Emptied,
}

impl Hand {
    /// Fold this into the loop's hand. A method and not a `match` in
    /// `main`, which is at its cognitive-complexity cap.
    fn apply(self, hand: &mut Option<Slot>) {
        match self {
            Self::Unchanged => {}
            Self::Took(at) => *hand = Some(at),
            Self::Emptied => *hand = None,
        }
    }
}

/// Carry out one gesture and say what happened.
///
/// A jump is tmux's and everything else is anchord's, which is the same
/// split as `docs/06-views.md`: `tally::act` never opens a terminal and
/// `tally::jump` never opens a socket.
///
/// Errors are printed and dropped ([`tally::act`]).
fn perform(
    ctx: &Context<'_>,
    gesture: Gesture,
    at: Slot,
    pages: &Pages,
    tmux: &impl Jump,
    journal: &mut Journal,
    queued: usize,
) -> Hand {
    let (row, col) = (at.row, at.col);
    match gesture {
        // A bare tap is a jump or it is nothing. It was also the
        // spawn-from-a-dark-pad gesture until #40 cut the spec system
        // that fed it, and with that gone there is no anchord call on
        // this path at all -- jumps are tmux's business.
        Gesture::Tap { .. } => {
            if let Some(r) = ctx.pads.grid().get(&at) {
                eprintln!("tallyd: jump {}:{row}:{col} -> {}", at.page, r.session);
                // The address names the window. Unnamed pages cannot
                // happen for a slot that parsed, but an empty string
                // is a legal name and falls back to the old form.
                let label = at.label(pages).unwrap_or_default();
                if let Err(e) = tmux.jump(
                    &r.session,
                    r.claude.as_deref(),
                    &label,
                    at.place(),
                    r.cwd.as_deref(),
                ) {
                    eprintln!("tallyd: {e}");
                }
                // The gesture the product exists for, written down at
                // last (#43). `None` when this pad was not asking,
                // which is the distinction the week needs: a tap can
                // land on something quiet, and `►` never can.
                //
                // Both numbers are last tick's, like the grid the
                // finger landed on -- the queue as it was when the
                // press happened, not as it is after it.
                let now = tally::now_ms();
                let waited = journal
                    .waiting()
                    .get(&at)
                    .map(|(_, since)| now.saturating_sub(*since));
                journal.jumped_pad(now, &label, waited, queued);
            }
        }
        Gesture::Chord { action, .. } => {
            return match ctx.chord(action, at, pages) {
                // The preview already said nothing would happen here, so
                // the log stays quiet too: a line per dimmed pad you
                // brushed is noise in the only file that is read after
                // something broke.
                Ok(Done::Nothing) => Hand::Unchanged,
                Ok(done @ Done::Picked(picked)) => {
                    eprintln!("tallyd: {action:?} {}:{row}:{col} {done}", at.page);
                    Hand::Took(picked)
                }
                Ok(done @ Done::Moved { .. }) => {
                    eprintln!("tallyd: {action:?} {}:{row}:{col} {done}", at.page);
                    if let Done::Moved { session, to, .. } = &done {
                        // The label is written; the pane follows it. Not
                        // part of the move: a failure is a line here and
                        // the next press of the pad repairs it.
                        let label = to.label(pages).unwrap_or_default();
                        if let Err(e) = tmux.relocate(session, &label, to.place()) {
                            eprintln!("tallyd: relocate {session}: {e}");
                        }
                    }
                    Hand::Emptied
                }
                Ok(done) => {
                    eprintln!("tallyd: {action:?} {}:{row}:{col} {done}", at.page);
                    Hand::Unchanged
                }
                Err(e) => {
                    eprintln!("tallyd: {action:?} {}:{row}:{col}: {e}", at.page);
                    // The write failed, so the pad is still in your hand
                    // and still white. Emptying it here would lose the
                    // pick-up to an error you can simply retry.
                    Hand::Unchanged
                }
            };
        }
        // Handled in the loop: they change where you are or what is
        // running, and all of them need state the loop computes.
        Gesture::PageUp
        | Gesture::PageDown
        | Gesture::Back
        | Gesture::Next
        | Gesture::Home
        | Gesture::Arts
        | Gesture::Launch { .. } => {}
    }
    Hand::Unchanged
}

/// The labels a `>` launch carries.
///
/// `tl.slot` and nothing else. **No `tl.spec`**, for the same reason
/// `tally spawn` writes none: a spec label is a name looked up in
/// `slots.toml`, and one nothing declares would be a label that lies
/// (`docs/07-cli.md`).
fn launch_labels(at: Slot, pages: &Pages) -> std::collections::BTreeMap<String, String> {
    let mut labels = std::collections::BTreeMap::new();
    if let Some(address) = at.label(pages) {
        let _slot = labels.insert(tally::anchor::LABEL_SLOT.to_owned(), address);
    }
    labels
}

/// Start the reel at its first piece, and say so.
///
/// Only ever reached from the working grid: once an art is up,
/// [`art_press`] intercepts every button before `feed` can emit
/// [`Gesture::Arts`] again, so the menu — not this — is what changes
/// pieces (`docs/05-interaction.md`).
fn next_art(reel: &mut Option<Reel>) {
    *reel = Piece::after(reel.as_ref().map(Reel::piece)).map(Reel::new);
    match reel.as_ref() {
        Some(r) => eprintln!("tallyd: arts {}", r.piece().name()),
        None => eprintln!("tallyd: arts off"),
    }
}

/// Route a press while an art is playing, and say whether it is
/// **swallowed** (`docs/05-interaction.md`).
///
/// In this mode the surface is a menu, not a grid. `Drums` and the eight
/// launchers each pick a piece, and nothing else on the board does
/// anything at all — a pad press lands on a sine wave, not a session, so
/// it is eaten rather than acted on.
///
/// `►` is the way out, and the only one. It is read here, before the
/// corner's double-tap clock and before `chords.feed`, which is what
/// keeps a press in this mode from reaching either of them
/// ([`Grip::feel`]).
fn art_press(reel: &mut Option<Reel>, repaint: &mut bool, button: Button, down: bool) -> bool {
    if !down || reel.is_none() {
        return false;
    }
    if button == tally::art::LEAVE_BUTTON {
        *reel = None;
        *repaint = true;
        eprintln!("tallyd: arts off");
        // Not swallowed: `►` still goes where it was always going, so
        // leaving the art and answering the thing that is waiting are one
        // press and not two.
        return false;
    }
    if let Some(piece) = Piece::at(button) {
        if reel.as_ref().map(Reel::piece) != Some(piece) {
            eprintln!("tallyd: arts {}", piece.name());
        }
        *reel = Some(Reel::new(piece));
        *repaint = true;
    }
    true
}

/// Advance the art one frame, on every other wake.
///
/// ~31 fps off a 16 ms tick: smooth to the eye at half the bytes
/// (`docs/05-interaction.md#the-arts`).
fn advance_art(reel: Option<&mut Reel>, wake: &mut bool) {
    *wake = !*wake;
    if let Some(r) = reel.filter(|_| *wake) {
        r.tick();
    }
}

/// The surface while an art is playing (`docs/05-interaction.md`).
///
/// The art owns the sixty-four pads and nothing else. Three lights are
/// not its to take: `►`, because the one gesture this product exists to
/// provide does not go away because the grid is busy — and behind a
/// plasma field it is carrying **all** the attention, since none of it is
/// on the grid; `Drums`, because a mode has to say it is on; and the
/// logo, because an art does not make the stack any more or less up.
fn art_surface(reel: &Reel, reachable: bool, queued: usize) -> Frame {
    let mut frame = reel.frame();
    // The menu: one button a piece, white on the one that is playing.
    for piece in Piece::REEL {
        frame.set(piece.selector(), tally::render::arts(piece == reel.piece()));
    }
    // The way out, and the thing that is waiting. `►` carries both, and
    // amber wins when there is any, because amber always does.
    frame.set(
        tally::art::LEAVE_BUTTON,
        if queued > 0 {
            tally::render::next(true)
        } else {
            tally::render::leave()
        },
    );
    // Health, the same as anywhere else. The art changed the grid, not
    // the stack behind it.
    frame.set(Button::Logo, tally::render::logo(reachable));
    frame
}

/// The Launchpad, if there is one, and what it has told us lately.
///
/// **Absence is a normal state, not a failure** — `tallyd` is started by
/// launchd and may well come up before the device is plugged in
/// ([08](08-lifecycle.md#who-starts-it)). What makes this a type rather
/// than four locals is that the rules for *leaving* are subtle and were
/// spread across sixty lines of `main`: a failed write drops it, silence
/// past [`DEVICE_SILENCE`] drops it, every arrival and departure forces a
/// total repaint, and both departures must clear the chord state or the
/// first tap after a replug is a kill (`docs/08-daemon.md`).
#[derive(Debug)]
struct Link {
    device: Option<Device>,
    /// When the port was last opened or pinged.
    last_try: Duration,
    /// When the device last answered anything.
    last_reply: Duration,
    /// The next frame must be total rather than a diff.
    ///
    /// Not a rendering preference: it is the answer to *has this device
    /// seen a frame from this process*, which is why it lives with the
    /// device and is set by whatever makes it untrue.
    repaint: bool,
    /// A device left during this tick. Drained once, by the loop.
    departed: bool,
}

impl Link {
    /// No device yet, and the first frame on any device is total.
    fn new() -> Self {
        Self {
            device: None,
            last_try: Duration::ZERO,
            last_reply: Duration::ZERO,
            repaint: true,
            departed: false,
        }
    }

    /// The device, for the things that still have to talk to it directly.
    fn device(&mut self) -> Option<&mut Device> {
        self.device.as_mut()
    }

    /// Look for a device, on the reconcile clock.
    ///
    /// `midir` enumerates ports; it does not tell you when one appears,
    /// so this is the same reconcile shape as everything else here.
    fn connect(&mut self, elapsed: Duration) {
        if self.device.is_some() || elapsed.saturating_sub(self.last_try) < RECONCILE {
            return;
        }
        self.last_try = elapsed;
        match Device::open() {
            Ok(d) => {
                eprintln!("tallyd: Launchpad connected");
                self.device = Some(d);
                self.repaint = true;
                self.last_reply = elapsed;
            }
            // Unplugged, or plugged in and still settling: a port can
            // enumerate a moment before the device answers an inquiry.
            // Both are ordinary and both resolve on the next try, so
            // neither gets a line every two seconds.
            Err(Error::NotFound | Error::NotALaunchpad) => {}
            Err(e) => eprintln!("tallyd: {e}"),
        }
    }

    /// It answered something.
    fn heard(&mut self, elapsed: Duration) {
        self.last_reply = elapsed;
    }

    /// Ping on the reconcile clock, and drop a device that has gone quiet.
    ///
    /// A failed write is not how you learn the device is gone — `CoreMIDI`
    /// accepts sends to a removed endpoint without complaint — so absence
    /// of a reply is the only signal there is.
    fn probe(&mut self, elapsed: Duration) {
        let Some(d) = self.device.as_mut() else {
            return;
        };
        if elapsed.saturating_sub(self.last_try) >= RECONCILE {
            self.last_try = elapsed;
            if let Err(e) = d.ping() {
                eprintln!("tallyd: {e}");
            }
        }
        if elapsed.saturating_sub(self.last_reply) >= DEVICE_SILENCE {
            eprintln!("tallyd: Launchpad stopped answering, dropping it");
            self.lost();
        }
    }

    /// Put a frame on it, and drop the device if the write fails.
    ///
    /// A full repaint on (re)connect: the device may have been unplugged,
    /// power-cycled, or left in another process's idea of a frame, so the
    /// first thing we put on it is total.
    fn paint(&mut self, frame: &Frame) {
        let Some(d) = self.device.as_mut() else {
            return;
        };
        let wrote = if self.repaint {
            d.repaint(frame)
        } else {
            d.render(frame)
        };
        match wrote {
            Ok(()) => self.repaint = false,
            Err(e) => {
                // The write failed, which on this stack means the port
                // went away. Drop it and go back to polling; the pads
                // are dark anyway, because the device is gone. `lost`
                // abandons it, so no restore is tried on a dead port.
                eprintln!("tallyd: Launchpad disconnected ({e})");
                self.lost();
            }
        }
    }

    /// The one departure path. Both callers above reach it, which is the
    /// point of the type: a device that leaves always leaves the same way.
    ///
    /// That includes the device being abandoned *here*, not by the caller:
    /// restoring Live mode on a port that has gone away either prints the
    /// Setup-menu alarm for what is only an unplug, or blocks this single
    /// thread for `REPLY_TIMEOUT` waiting on a readback that cannot come
    /// ([`Device::abandon`]).
    fn lost(&mut self) {
        if let Some(d) = self.device.as_mut() {
            d.abandon();
        }
        self.device = None;
        self.repaint = true;
        self.departed = true;
    }

    /// Did a device leave this tick? Answered once, because the thing the
    /// loop does about it — clear the held chords, so the release that
    /// went out with the cable does not strand a modifier — must happen
    /// exactly once per departure.
    fn take_departed(&mut self) -> bool {
        std::mem::take(&mut self.departed)
    }
}

/// The beat the surface breathes to.
///
/// `bpm` is recomputed each reconcile from the oldest thing waiting; the
/// rest is when the next `0xF8` is due. Held together because a tempo
/// without its schedule is a number nothing reads
/// (`docs/08-daemon.md`).
#[derive(Debug)]
struct Beat {
    bpm: u64,
    /// When the next pulse is due.
    next: Duration,
    /// Whether `clock_start` has been accepted by the device in front of
    /// us. Cleared when there is no device, so a replug starts over.
    started: bool,
}

impl Beat {
    /// Calm, and not yet running.
    fn new() -> Self {
        Self {
            bpm: tempo::FLOOR_BPM,
            next: Duration::ZERO,
            started: false,
        }
    }

    /// Set the tempo from the age of the oldest thing waiting, and say so
    /// when it moves (`docs/05-interaction.md`).
    fn retune(&mut self, oldest_wait_ms: Option<i64>) {
        let was = self.bpm;
        self.bpm = tempo::bpm(oldest_wait_ms);
        if self.bpm != was {
            eprintln!("tallyd: tempo {} bpm", self.bpm);
        }
    }

    /// Is a pulse due at `elapsed`? Claims it if so.
    ///
    /// The schedule is **monotonic**: a sent pulse advances `next` by one
    /// `gap` from where it was *due*, not from the tick that noticed it.
    /// Re-basing on the tick instead rounds every wait up to a whole
    /// `TICK`, which does not slow the beat a little — it quantises the
    /// whole 60..140 ramp onto the two rates 16 ms happens to divide
    /// into, and everything from 79 bpm up emits identically. The log
    /// said 140 and the device breathed 78.
    ///
    /// Split out from [`Beat::pulse`] so the schedule can be walked
    /// without a Launchpad in front of it; the emitted rate is the thing
    /// worth pinning and it is unobservable through a `Device`.
    fn due(&mut self, elapsed: Duration) -> bool {
        if elapsed < self.next {
            return false;
        }
        let gap = tempo::gap(self.bpm);
        self.next += gap;
        if self.next <= elapsed {
            // A whole gap went by unserved, so the loop stalled. Missed
            // pulses are **dropped, not caught up** — a burst would
            // visibly sprint the breathing — which means re-basing the
            // phase on now (`docs/05-interaction.md`). Unreachable while
            // `gap` exceeds the loop's tick, which `tempo.rs` pins.
            self.next = elapsed + gap;
        }
        true
    }

    /// Send the beat.
    ///
    /// One byte, up to fifty-six times a second, on the schedule
    /// [`Beat::due`] keeps.
    fn pulse(&mut self, device: Option<&mut Device>, elapsed: Duration) {
        let Some(d) = device else {
            // Unplugged. Start again when it comes back.
            self.started = false;
            return;
        };
        if !self.started {
            self.started = d.clock_start().is_ok();
            self.next = elapsed;
        }
        if !self.due(elapsed) {
            return;
        }
        if let Err(e) = d.clock() {
            eprintln!("tallyd: clock: {e}");
        }
    }
}

/// What the queue looked like at the last reconcile.
///
/// Deliberately **stale**: it is what the finger was looking at when it
/// landed, and recomputing on the press would act on a grid the eye never
/// saw. The two fields are one answer read two ways — `►` acts on the
/// head, the top row lights from the depth — and they are never computed
/// apart.
#[derive(Debug, Default)]
struct Waiting {
    head: Option<queue::Head>,
    queued: usize,
}

/// Spawn into a grid row from its `>` button. Reports whether the surface
/// needs repainting.
///
/// Lifted out of `main` to keep it inside its complexity budget. `grid`
/// is **last tick's** resolution — the same one that lit the button, so
/// the press and the preview cannot disagree about what the row holds.
fn launch(
    abs: u8,
    sessions: &[tally::anchor::Session],
    pages: &Pages,
    grid: &std::collections::BTreeMap<Slot, model::Resolved>,
    anchor: &Client,
) -> bool {
    let Some(l) = model::row_launch(sessions, pages, grid, abs) else {
        return false;
    };
    match anchor.create(&tally::anchor::Spawn {
        command: &l.command,
        args: &[],
        cwd: Some(std::path::Path::new(&l.cwd)),
        env: &std::collections::BTreeMap::new(),
        login_shell: l.login_shell,
        labels: &launch_labels(l.at, pages),
    }) {
        Ok(id) => eprintln!(
            "tallyd: launch {} -> {id} in {}",
            l.at.label(pages).unwrap_or_default(),
            l.cwd
        ),
        Err(e) => eprintln!("tallyd: launch: {e}"),
    }
    true
}

/// Move the window. **The offset and nothing else** — no session list, no
/// queue, no tmux and no journal, which is why `⌂`'s other call site
/// ([`Grip::home_due`]) reaches this and not [`navigate`]: handing those
/// five to an arm that provably never reads them reads as a trap.
fn scroll(gesture: Gesture, offset: &mut u8) {
    match gesture {
        // One row, not one page. A page boundary is not a thing you can
        // feel; a row is.
        Gesture::PageUp | Gesture::PageDown => {
            let n = if gesture == Gesture::PageUp {
                offset.saturating_sub(1)
            } else {
                offset.saturating_add(1).min(slot::MAX_OFFSET)
            };
            if n != *offset {
                *offset = n;
                eprintln!("tallyd: rows {}-{}", n + 1, n + slot::WINDOW);
            }
        }
        Gesture::Home => {
            if *offset != 0 {
                *offset = 0;
                eprintln!("tallyd: home");
            }
        }
        Gesture::Tap { .. }
        | Gesture::Chord { .. }
        | Gesture::Launch { .. }
        | Gesture::Arts
        | Gesture::Next
        | Gesture::Back => {}
    }
}

/// The top-row gestures: move the window ([`scroll`]), or move you.
///
/// Lifted out of `main` because none of them touches anchord. `head` and
/// `queued` are deliberately **stale** ([`Waiting`]); only `►` and `◄`
/// read them, which is the whole reason [`scroll`] is a separate name.
fn navigate(
    gesture: Gesture,
    offset: &mut u8,
    head: Option<&queue::Head>,
    queued: usize,
    pages: &Pages,
    tmux: &impl Jump,
    journal: &mut Journal,
) {
    match gesture {
        Gesture::PageUp | Gesture::PageDown | Gesture::Home => scroll(gesture, offset),
        Gesture::Next => {
            let Some(h) = head else {
                return;
            };
            let at = h.at.label(pages).unwrap_or_default();
            eprintln!("tallyd: next -> {at} ({queued} waiting)");
            if let Err(e) = tmux.jump(
                &h.session,
                h.claude.as_deref(),
                &at,
                h.at.place(),
                h.cwd.as_deref(),
            ) {
                eprintln!("tallyd: {e}");
            }
            // Follow the jump with the eyes, by the smallest scroll that
            // shows where it landed.
            *offset = View::offset_showing(h.at, *offset);
            let now = tally::now_ms();
            journal.jumped_next(now, &at, now.saturating_sub(h.since), queued);
        }
        Gesture::Back => match tmux.back() {
            Ok(moved) => {
                eprintln!(
                    "tallyd: back{}",
                    if moved { "" } else { " (nowhere to go)" }
                );
                journal.jumped_back(tally::now_ms(), moved);
            }
            Err(e) => eprintln!("tallyd: {e}"),
        },
        Gesture::Tap { .. } | Gesture::Chord { .. } | Gesture::Launch { .. } | Gesture::Arts => {}
    }
}

/// Count the views that are wrong — in the wrong window, or dead under a
/// live session — and say so. **Nothing is moved**
/// (`docs/06-views.md`).
///
/// Not at process start: with no session list there is nothing to audit,
/// and a sweep over nothing would report a reassuring zero. After that it
/// runs on every reconnect and on the [`SWEEP`] clock, because one sample
/// per daemon lifetime cannot see drift that heals lazily between
/// restarts ([#22](https://github.com/alehatsman/tally/issues/22)).
///
/// Here and not in `jump` because the comparison needs `Slot::place()`
/// and anchord's opinion of liveness, and `jump` knows about tmux and
/// nothing else (`src/slot.rs`, `tally::jump::Want`).
///
/// Returns whether a sample was taken. `false` means no session claimed
/// a pad, nothing was forked and nothing was printed — the caller's
/// [`SWEEP`] clock should not count it as a sweep.
fn sweep(tmux: &Tmux, sessions: &[tally::anchor::Session], pages: &Pages) -> bool {
    let want: Vec<tally::jump::Want> = sessions
        .iter()
        // Every session that claims a pad, exited ones included: a corpse
        // sitting at the wrong index wedges that pad exactly as a live
        // view does, which is the case `docs/35`'s first half fixes. It
        // is drift, and it counts. `alive` is what keeps a corpse at its
        // *own* address from being counted twice over.
        .filter_map(|s| {
            let at = s.slot_label()?;
            let slot = slot::parse(at, pages)?;
            Some(tally::jump::Want {
                session: s.id.clone(),
                at: at.to_owned(),
                place: slot.place(),
                // Not an allow-list. `lifecycle` maps any state this
                // build has not heard of to `Lost`, so naming the live
                // states would let a word anchord learns later put out
                // the `dead view` light for every session wearing it --
                // the blindness this fault was added to end. Only an
                // explicit `exited` makes a corpse unremarkable.
                alive: s.lifecycle() != SessionState::Exited,
            })
        })
        .collect();
    // Nothing claims a pad, so no `list-panes` is forked and no sample
    // is taken. The caller must not spend the [`SWEEP`] slot on it: an
    // unlabelled list at first contact would otherwise defer the first
    // real sample a full fifteen minutes.
    if want.is_empty() {
        return false;
    }
    let seen = tmux.audit(&want);
    for f in &seen.faults {
        eprintln!(
            "tallyd: sweep: {} claims {}, sits at {}, {}",
            f.session, f.claims, f.sits_at, f.fault
        );
    }
    let drifted = seen
        .faults
        .iter()
        .filter(|f| f.fault == tally::jump::Fault::Drifted)
        .count();
    // `ms=` in the journal's own unit, so a sweep line and a rising edge
    // can be read on one timeline. A summary line that cannot be placed
    // in time cannot be compared with anything (`docs/06-views.md`).
    eprintln!(
        "tallyd: sweep: ms={} {} views, {} drifted, {} dead",
        tally::now_ms(),
        seen.examined,
        drifted,
        seen.faults.len() - drifted,
    );
    true
}

/// Take the single-instance lock, or report who holds it.
///
/// `None` is "another instance is running", which `main` turns into a failure
/// exit -- not an error worth a `Result`, because there is exactly one caller
/// and exactly one thing it can do about it.
fn acquire_lock() -> Option<Lock> {
    let lock_path = Lock::default_path();
    match Lock::acquire(&lock_path) {
        Ok(lock) => Some(lock),
        Err(e) => {
            eprintln!(
                "tallyd: another instance holds {}: {e}",
                lock_path.display()
            );
            None
        }
    }
}

/// Load the page config, falling back to defaults.
///
/// A broken config is reported and then ignored, for the reason
/// `tally::config`'s module header gives in full.
fn load_pages() -> Pages {
    let config_path = config::default_path();
    let pages = config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("tallyd: {}: {e}", config_path.display());
        eprintln!("tallyd: running on defaults; pages are named 1-8");
        Pages::default()
    });
    eprintln!(
        "tallyd: {} pages from {}",
        pages.len(),
        config_path.display()
    );
    pages
}

/// A reply that says Live mode means the Launchpad was power-cycled under us.
/// Put it back into programmer mode and repaint from scratch.
fn handle_reply(reply: &[u8], link: &mut Link) {
    if tally::surface::sysex::programmer_mode_from(reply) == Some(false) {
        eprintln!("tallyd: Launchpad came back in Live mode, reprogramming");
        if let Some(e) = link.device().and_then(|d| d.reprogram().err()) {
            eprintln!("tallyd: {e}");
        }
        link.repaint = true;
    }
}

/// What the anchord branch of the reconcile remembers between calls.
///
/// Two facts, one lifetime: whether the last `list` succeeded, and when
/// the drift sweep last ran. They travel together because the reconnect
/// *edge* — the transition in the first — is one of the two occasions
/// the second runs on (`docs/08-daemon.md`).
#[derive(Debug, Default)]
struct Contact {
    /// Did the last `list` succeed?
    reachable: bool,
    /// When the drift sweep last ran, or `None` before the first one.
    last_sweep: Option<Duration>,
}

/// anchord stopped answering. Say so once per outage, not once per tick.
fn report_unreachable(e: &impl std::fmt::Display, reachable: &mut bool) {
    if *reachable {
        eprintln!("tallyd: {e}");
    }
    *reachable = false;
}

/// Ask anchord for the list and bring up to date everything that moves on
/// its clock: the watchers, the journal's first listing, the drift
/// sweep, and the two expensive answers that are pure functions of
/// the list. Returns the fresh `purgeable` and `viewed` for the caller to
/// move into its own.
///
/// Lifted out of `main` to keep it inside its length budget, and it is
/// one job with one name: this is the whole of what happens on the
/// `RECONCILE` clock rather than on the tick, and nothing else in the
/// loop runs on it.
fn reconcile(
    anchor: &Client,
    watchers: &Watchers,
    pages: &Pages,
    tmux: &Tmux,
    journal: &mut Journal,
    sessions: &mut Vec<tally::anchor::Session>,
    contact: &mut Contact,
    elapsed: Duration,
) -> (std::collections::BTreeMap<Slot, Vec<String>>, Vec<String>) {
    match anchor.list() {
        Ok(list) => {
            // Read before it is set: this is the reconnect edge, and it
            // is one of the two occasions the sweep runs on.
            let reconnected = !contact.reachable;
            if reconnected {
                eprintln!("tallyd: anchord reachable, {} sessions", list.len());
            }
            watchers.reconcile(&list, pages, tally::now_ms());
            watchers.retain_known(&list);
            *sessions = list;
            contact.reachable = true;
            // The journal has been holding its replayed map since
            // `open`, because until this line an empty `sessions`
            // meant "nobody asked anchord yet" rather than
            // "nothing is asking" (`tally::journal::Journal`).
            journal.saw_listing();
            // On the reconnect edge — anchord dropping its connections
            // is what killed seven view panes on 2026-10-20 and the old
            // once-ever sweep could not see it — and otherwise on the
            // [`SWEEP`] clock. The first successful `list` is a
            // reconnect, so first contact is covered by the same line
            // (`docs/06-views.md`).
            let since = contact.last_sweep.map(|t| elapsed.saturating_sub(t));
            let due = since.is_none_or(|d| d >= SWEEP);
            // The edge still sweeps at once, but not twice inside
            // [`SWEEP_FLOOR`]: a flapping anchord is itself a
            // reconcile-clock event, and the second edge has nothing
            // new to say about a grid the first one already read.
            let edge = reconnected && since.is_none_or(|d| d >= SWEEP_FLOOR);
            // Stamped only when a sample was actually taken, so a list
            // that claims no pads does not consume the slot.
            if (edge || due) && sweep(tmux, sessions, pages) {
                contact.last_sweep = Some(elapsed);
            }
        }
        Err(e) => {
            // Deliberately keep the previous `sessions`: the grid
            // freezes rather than lying about what is running.
            report_unreachable(&e, &mut contact.reachable);
        }
    }
    // Both of these are pure functions of state that only moves
    // in this branch, and both are expensive: `purgeable` walks
    // the session list allocating a `Vec<String>` per occupied
    // pad, and `viewing` forks two `tmux` processes. On the tick
    // they ran ~62 times a second each to produce a value that
    // changes at most twice (review w8 F2, review w4 F3).
    //
    // `purgeable` is recomputed on the error arm too, which is
    // free: `sessions` is deliberately kept on a failed list, so
    // the answer is the one it already had.
    let viewed = tmux.viewing();
    // Same reason, same clock: `can_go_back` is read on every frame and
    // must stay a map lookup, so the fork that tells a closed origin
    // from a live one happens here. `◄` is lit by the map this
    // leaves behind.
    tmux.prune_origins();
    (model::purgeable(sessions, pages), viewed)
}

/// Read the queue off the journal: which slot `►` would offer, and how
/// many are asking.
///
/// Lifted out of `main` to keep it inside its length budget, and the two
/// are one answer that is never computed apart ([`Waiting`]). `viewing`
/// lives here because it exists only to be their argument.
fn waiting_now(
    journal: &Journal,
    pads: &model::Pads<'_>,
    resolved: &std::collections::BTreeMap<Slot, model::Resolved>,
    viewed: &[String],
) -> Waiting {
    // The list of viewed sessions is the reconcile's; turning it into
    // pads is this tick's, so a session that moved pad since is drawn
    // where it is now rather than where it was.
    let viewing: std::collections::BTreeSet<Slot> = viewed
        .iter()
        .filter_map(|id| resolved.values().find(|r| &r.session == id))
        .map(|r| r.slot)
        .collect();
    Waiting {
        head: queue::head(journal.waiting(), pads, &viewing),
        queued: queue::depth(journal.waiting(), pads, &viewing),
    }
}

fn main() -> ExitCode {
    // Before the lock, so even a refused start says which binary was
    // refused: a log that cannot name its build cannot attribute
    // anything in it to a change (`tally::BUILD`, `docs/08-daemon.md`).
    eprintln!("tallyd: tally {} ({})", tally::VERSION, tally::BUILD);

    install_handlers();

    let Some(_lock) = acquire_lock() else {
        return ExitCode::FAILURE;
    };

    let anchor = Client::new(Client::default_path());
    eprintln!("tallyd: anchord at {}", anchor.path().display());

    // No device is a normal state, not a failure ([`Link`]).
    let mut link = Link::new();

    let pages = load_pages();

    let mut chords = Chords::default();
    let watchers = Watchers::new(anchor.clone());
    // Best effort ([`tally::journal::Sink`]), and read back from disk, so
    // a restart does not write a second rising edge for a slot that never
    // stopped asking (`tally::journal`).
    let journal_path = journal::default_path();
    let mut journal = Journal::open(&journal_path, &pages);
    let mut sessions = Vec::new();
    // Whether anchord answered last time, and when the sweep last ran.
    let mut contact = Contact::default();
    let mut offset = 0u8;
    // The timed gestures and the pad a held `Move` is carrying.
    let mut grip = Grip::default();
    // The queue, as of the last tick.
    let mut waiting = Waiting::default();
    let mut beat = Beat::new();
    let mut last_reconcile = Duration::ZERO;
    // The art, if one is playing (`docs/05-interaction.md`). `None` is the
    // working grid, and it is where the reel starts and ends.
    let mut reel: Option<Reel> = None;
    let mut art_wake = false;
    let tmux = Tmux::default();
    if tmux.server_running() {
        // A server that predates this daemon was started by someone else's
        // config; a tab closed before the first press would renumber it.
        tmux.hold_indexes();
    } else {
        // Said plainly, not fatal ([`Tmux::server_running`]).
        eprintln!("tallyd: tally's tmux server is not up yet; a jump will start one");
    }
    // What was on the grid last tick, so a press knows what it hit.
    let mut grid: std::collections::BTreeMap<Slot, model::Resolved> =
        std::collections::BTreeMap::new();
    // Rows anchord would forget, including ones nothing draws: a clean
    // exit keeps its `tl.slot` behind a dark pad, and the purge chord is
    // the only way to see or clear it.
    let mut purgeable: std::collections::BTreeMap<Slot, Vec<String>> =
        std::collections::BTreeMap::new();
    // The anchor sessions a tmux client is looking at, as of the last
    // reconcile. Asking costs two `tmux` fork+execs, so it is asked on
    // the same clock as every other outside-world fact here.
    let mut viewed: Vec<String> = Vec::new();
    let started = std::time::Instant::now();

    let signum = loop {
        let signum = SIGNALLED.load(Ordering::Relaxed);
        if signum != 0 {
            break signum;
        }

        let elapsed = started.elapsed();

        link.connect(elapsed);

        // Presses are event-driven, not on the reconcile clock. Drained
        // into a batch first so the handlers below can reach the rest of
        // `link` -- the art's repaint, a reprogram -- without holding a
        // borrow on the port they would all have to be threaded through.
        let events: Vec<Event> = link.device().map_or_else(Vec::new, |d| d.poll().collect());
        for event in events {
            match event {
                Event::Press { button, down } => {
                    // Captured before `feed`, which answers `None` for
                    // plenty of presses and would skip this with the
                    // `continue` below. The clock is here because
                    // [`tally::chord`] has none ([`Grip`]).
                    grip.aim(button, down, elapsed);
                    // Before the corner's clock and before `feed`:
                    // an art swallows the press whole, so neither
                    // gets to act on one the eye never aimed at a
                    // session (`docs/05-interaction.md`).
                    if art_press(&mut reel, &mut link.repaint, button, down) {
                        continue;
                    }
                    // Same reason, same place: a double-tap is two
                    // presses close in time, and the release that
                    // completes one answers `None` from `feed`.
                    link.repaint |= grip.feel(&mut chords, button, down, elapsed);
                    let Some(gesture) = chords.feed(button, down) else {
                        continue;
                    };
                    grip.corner.saw(gesture);
                    match gesture {
                        Gesture::Arts => {
                            next_art(&mut reel);
                            link.repaint = true;
                        }
                        Gesture::Launch { row } => {
                            link.repaint |= launch(
                                offset.saturating_add(row),
                                &sessions,
                                &pages,
                                &grid,
                                &anchor,
                            );
                        }
                        Gesture::PageUp
                        | Gesture::PageDown
                        | Gesture::Home
                        | Gesture::Next
                        | Gesture::Back => {
                            navigate(
                                gesture,
                                &mut offset,
                                waiting.head.as_ref(),
                                waiting.queued,
                                &pages,
                                &tmux,
                                &mut journal,
                            );
                            // `►` and `◄` move the tmux client, and which
                            // panes are being viewed is what `queue::head`
                            // skips on. Pull the reconcile forward, the
                            // same way a press on a pad does, rather than
                            // offering the same head again for two
                            // seconds. The three scrolls move no client
                            // and need nothing.
                            if matches!(gesture, Gesture::Next | Gesture::Back) {
                                last_reconcile = Duration::ZERO;
                            }
                        }
                        Gesture::Tap { row, col } | Gesture::Chord { row, col, .. } => {
                            // The window turns a pad into an address.
                            // Past the end of the address space there
                            // is no pad, and nothing happens.
                            let Some(at) = View::slot_at(offset, row, col) else {
                                continue;
                            };
                            let ctx = Context {
                                client: &anchor,
                                // Last tick's grid: what the pad the
                                // finger landed on was showing.
                                pads: model::Pads::new(&grid, &purgeable, grip.hand),
                            };
                            perform(
                                &ctx,
                                gesture,
                                at,
                                &pages,
                                &tmux,
                                &mut journal,
                                waiting.queued,
                            )
                            .apply(&mut grip.hand);
                            // Whatever it was, the grid is now stale.
                            // Pull the list forward rather than
                            // leaving the pad wrong for two seconds.
                            last_reconcile = Duration::ZERO;
                        }
                    }
                }
                Event::Reply(r) => {
                    link.heard(elapsed);
                    handle_reply(&r, &mut link);
                }
            }
        }

        link.probe(elapsed);

        if elapsed.saturating_sub(last_reconcile) >= RECONCILE || last_reconcile.is_zero() {
            last_reconcile = elapsed;
            (purgeable, viewed) = reconcile(
                &anchor,
                &watchers,
                &pages,
                &tmux,
                &mut journal,
                &mut sessions,
                &mut contact,
                elapsed,
            );
        }

        let now = tally::now_ms();
        // `resolve` and `journal.tick` stay on the fast clock while the
        // listing above does not, and the reason is `now`: amber, the
        // journal's rising edges and the tempo are all functions of it,
        // and the one number this project is measuring is a count of
        // those edges and how long each waited (`docs/04-signals.md`).
        //
        // `snapshot()` stays with them for the same reason, though it is
        // the one expensive call left here -- it locks and deep-clones
        // the signal map every wake (review w5). Moving it to the
        // reconcile would delay a hook-raised amber, and mistime the
        // edge the journal writes for it, by up to two seconds. The fix
        // belongs in `watch.rs` -- a generation counter, so a snapshot
        // that has not moved is not cloned -- not here.
        let mut resolved = model::resolve(&sessions, &watchers.snapshot(), &pages, now);
        journal.tick(&resolved, &pages, now);

        // The queue, after the journal has been brought up to date: the
        // map it keeps *is* the queue, ordered by when each slot started
        // asking (`docs/05-interaction.md`).
        let pads = model::Pads::new(&resolved, &purgeable, grip.hand);
        waiting = waiting_now(&journal, &pads, &resolved, &viewed);
        // How fast the surface breathes: the age of the oldest thing
        // waiting (`docs/05-interaction.md`). The ramp still runs on the
        // device -- only the beat is ours.
        beat.retune(queue::oldest(journal.waiting(), &pads).map(|since| now - since));

        // `▲` held past the threshold means *all the way up*
        // ([`Grip::home_due`]).
        if grip.home_due(elapsed) {
            scroll(Gesture::Home, &mut offset);
        }

        advance_art(reel.as_mut(), &mut art_wake);
        // The art is the **branch**, not a fallback over a grid that was
        // built and dropped. `frame_from` walks every resolved pad and
        // `launchable` is seven `row_launch` calls over the session list;
        // behind a piece neither is ever drawn, so on a 16 ms tick that
        // was ~62 wasted boards a second for the length of the piece
        // (review w2 F3).
        let frame = if let Some(r) = reel.as_ref() {
            art_surface(r, contact.reachable, waiting.queued)
        } else {
            // Which `>` would act, from the same resolution the
            // press uses.
            let pressed: std::collections::BTreeSet<Action> = Action::all()
                .into_iter()
                .filter(|&a| chords.is_held(a))
                .collect();
            let launchable: std::collections::BTreeSet<u8> = (1..=7u8)
                .filter(|row| {
                    model::row_launch(&sessions, &pages, &resolved, offset.saturating_add(*row))
                        .is_some()
                })
                .collect();
            let view = View {
                pages: &pages,
                offset,
                launchable: &launchable,
                anchord_reachable: contact.reachable,
                now,
                can_go_back: tmux.can_go_back(),
                queued: waiting.queued,
                held: chords.modifier(),
                hand: grip.hand,
                pressed: &pressed,
                mode: chords.mode(),
            };
            model::frame_from(&pads, &view)
        };
        link.paint(&frame);

        // Hand this tick's resolution to `grid`, which is what the next
        // tick's press will act on. A swap rather than a clone --
        // `resolved` is finished with here, and `Resolved` owns up to
        // three `String`s per pad, so copying it a second time cost a
        // second full allocation of the board every 16 ms.
        std::mem::swap(&mut grid, &mut resolved);

        // One departure, one response, wherever this tick it happened
        // ([`Link::take_departed`]).
        if link.take_departed() {
            grip.unplugged(&mut chords);
        }

        beat.pulse(link.device(), elapsed);

        std::thread::sleep(TICK);
    };

    eprintln!("tallyd: signal {signum}, restoring");
    // Hand the beat back before letting go of the device, so a grid left
    // behind breathes at its own 120 rather than holding our last tempo.
    // Only on a *clean* exit: a `tallyd` that is killed stops the clock
    // dead, and the frozen breathing is the point
    // (`docs/05-interaction.md`).
    if let Some(e) = link.device().and_then(|d| d.clock_stop().err()) {
        eprintln!("tallyd: clock stop: {e}");
    }
    drop(link); // Dropping the device restores Live mode, verified by readback.
    reraise(signum);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Two quick taps, and the corner flips.
    #[test]
    fn two_quick_taps_toggle_the_mode() {
        let mut c = Corner::default();
        assert!(!c.feed(true, ms(0)));
        assert!(!c.feed(false, ms(80)), "one tap is not a double");
        assert!(!c.feed(true, ms(200)));
        assert!(c.feed(false, ms(280)), "the second one inside the window");
    }

    /// **The assertion the whole toggle rests on.** Chord rule 4 says
    /// releasing a modifier without tapping is a no-op, and that escape
    /// hatch is what the hold-then-tap grammar is built on. Abort a kill
    /// twice and the mode must not have moved
    /// (`docs/05-interaction.md#changing-the-mode`).
    #[test]
    fn an_aborted_hold_is_not_a_tap() {
        let mut c = Corner::default();
        assert!(!c.feed(true, ms(0)));
        assert!(!c.feed(false, ms(900)), "a long look at the preview");
        assert!(!c.feed(true, ms(1000)));
        assert!(!c.feed(false, ms(1900)), "and another one");
    }

    /// A hold that actually fired a chord is not a tap either, however
    /// quick it was. You killed something; you did not ask for a mode.
    #[test]
    fn a_hold_that_did_something_is_not_a_tap() {
        let mut c = Corner::default();
        assert!(!c.feed(true, ms(0)));
        c.saw(Gesture::Chord {
            action: Action::Stop,
            row: 1,
            col: 1,
        });
        assert!(!c.feed(false, ms(60)));
        assert!(!c.feed(true, ms(100)));
        assert!(!c.feed(false, ms(160)), "the pair was never started");
    }

    /// Two taps too far apart are two taps, not a double. The second
    /// starts a new pair rather than failing, so a slow hand does not
    /// have to stop and start over.
    #[test]
    fn taps_too_far_apart_do_not_pair() {
        let mut c = Corner::default();
        assert!(!c.feed(true, ms(0)));
        assert!(!c.feed(false, ms(80)));
        assert!(!c.feed(true, ms(2000)));
        assert!(!c.feed(false, ms(2080)), "too late to pair");
        assert!(!c.feed(true, ms(2200)));
        assert!(c.feed(false, ms(2280)), "but it started a fresh pair");
    }

    /// A release whose press never arrived is not a tap.
    ///
    /// [`art_press`] swallows the press that exits an art but lets the
    /// release through, so this is the real shape of pressing `CC19`
    /// while a piece is playing. If it counted as a tap, the mode could
    /// toggle behind an animation with nothing on the surface able to
    /// report it — which is exactly the failure 6039146 fixed, arriving
    /// by a second road.
    #[test]
    fn a_release_with_no_press_is_not_a_tap() {
        let mut c = Corner::default();
        assert!(!c.feed(false, ms(40)), "the art ate the press");
        // And it must not have left half a pair behind for a later tap.
        assert!(!c.feed(true, ms(100)));
        assert!(!c.feed(false, ms(160)), "this is a first tap, not a second");
        assert!(!c.feed(true, ms(200)));
        assert!(c.feed(false, ms(260)), "the pair completes normally after");
    }

    /// The same, mid-pair: one real tap, then an art eats a press, then
    /// its release arrives. The orphan must break the pair rather than
    /// complete it.
    #[test]
    fn an_orphan_release_cannot_complete_a_pair() {
        let mut c = Corner::default();
        assert!(!c.feed(true, ms(0)));
        assert!(!c.feed(false, ms(60)), "a genuine first tap");
        assert!(!c.feed(false, ms(120)), "an orphan must not finish it");
    }

    /// A long press between two taps clears the half-pair. Otherwise an
    /// abort could silently complete a double you started a minute ago.
    #[test]
    fn a_long_press_clears_a_half_started_double() {
        let mut c = Corner::default();
        assert!(!c.feed(true, ms(0)));
        assert!(!c.feed(false, ms(80)), "first tap");
        assert!(!c.feed(true, ms(120)));
        assert!(!c.feed(false, ms(900)), "an abort, not a tap");
        assert!(!c.feed(true, ms(950)));
        assert!(!c.feed(false, ms(1000)), "so this is a first tap again");
    }

    /// What a departure means, independent of why it happened.
    ///
    /// **Both drop paths are one method**, which is the whole reason
    /// [`Link`] is a type: a device that leaves always leaves the same
    /// way — no device, a total frame owed to the next one, and exactly
    /// one departure for the loop to answer
    /// (`docs/08-daemon.md`).
    #[test]
    fn a_device_that_leaves_always_leaves_the_same_way() {
        let mut link = Link::new();
        assert!(link.repaint, "the first frame on any device is total");
        assert!(!link.take_departed(), "nothing has left yet");

        link.repaint = false;
        link.lost();
        assert!(link.device.is_none());
        assert!(link.repaint, "whatever comes back gets a total frame");
        assert!(link.take_departed(), "the loop is told once");
        assert!(
            !link.take_departed(),
            "and not twice -- clearing the chords twice is free, but a \
             second report is how a drained flag stops meaning anything"
        );
    }

    /// With no device every method is a no-op rather than a branch at the
    /// call site. No device is the normal state at login
    /// ([08](08-lifecycle.md#who-starts-it)), so it cannot be the
    /// exceptional path.
    #[test]
    fn a_link_with_no_device_does_nothing_quietly() {
        let mut link = Link::new();
        link.repaint = false;
        // `connect` is the one that would try to open a port; it is on
        // the reconcile clock and this is well inside the first tick.
        link.probe(Duration::from_secs(600));
        link.paint(&Frame::dark());
        assert!(link.device.is_none());
        assert!(
            !link.take_departed(),
            "nothing left, because nothing was here"
        );
        assert!(!link.repaint, "and nothing arrived to owe a frame to");
    }

    /// The beat stops when the device does, and starts over when one
    /// comes back: `clock_start` has to be accepted by the device in
    /// front of us, not by the one that was here before the cable moved.
    #[test]
    fn an_unplugged_beat_starts_over() {
        let mut beat = Beat::new();
        assert_eq!(beat.bpm, tempo::FLOOR_BPM, "calm, not stopped");
        beat.started = true;
        beat.pulse(None, Duration::from_millis(16));
        assert!(
            !beat.started,
            "a device that is gone has not started a clock"
        );
    }

    /// Walk a `Beat` through a loop that ticks every `period`, and report
    /// the tempo it actually emitted over `span`.
    ///
    /// This is the main loop's shape and nothing else: one `due` call per
    /// tick, one `0xF8` per `true`. The device is the part that cannot be
    /// in a test; the schedule is the part that was wrong.
    fn emitted_bpm(bpm: u64, period: Duration, span: Duration) -> f64 {
        let mut beat = Beat::new();
        beat.bpm = bpm;
        let mut pulses = 0u32;
        let mut elapsed = Duration::ZERO;
        while elapsed < span {
            if beat.due(elapsed) {
                pulses += 1;
            }
            elapsed += period;
        }
        f64::from(pulses) * 60.0 / (span.as_secs_f64() * tempo::PPQN as f64)
    }

    /// The tempo that reaches the device is the tempo that was commanded.
    ///
    /// `tallyd` logging `tempo 140 bpm` while the grid breathes at 78 is
    /// the failure this pins: the schedule used to re-base on the tick
    /// that sent the pulse, so the emitted interval was a whole number of
    /// `TICK`s and the ramp collapsed onto two rates. Walked at the real
    /// `TICK`, because the quantisation is a property of the pair.
    #[test]
    fn the_commanded_tempo_is_the_emitted_tempo() {
        for bpm in [tempo::FLOOR_BPM, 79, 100, tempo::CEILING_BPM] {
            let got = emitted_bpm(bpm, TICK, Duration::from_secs(10));
            assert!(
                (got - f64::from(u32::try_from(bpm).expect("bpm fits"))).abs() < 1.0,
                "commanded {bpm} bpm, emitted {got:.1}"
            );
        }
    }

    /// The one thing the tempo exists to say is *how long* this has been
    /// waiting, so the ends of the ramp have to arrive as different
    /// rates. They used to arrive as 52 and 78 — different, but a four
    /// minute wait and a twenty minute one were both the 78
    /// (`docs/05-interaction.md`).
    #[test]
    fn the_floor_and_the_ceiling_do_not_breathe_alike() {
        let span = Duration::from_secs(10);
        let calm = emitted_bpm(tempo::FLOOR_BPM, TICK, span);
        let urgent = emitted_bpm(tempo::CEILING_BPM, TICK, span);
        assert!(
            urgent > calm * 2.0,
            "the ramp is 60..140; emitted {calm:.1}..{urgent:.1}"
        );
    }

    /// A loop that was busy skips beats rather than firing them in a
    /// burst: the breathing must never sprint to pay off a stall, and a
    /// dropped pulse is the honest report (`docs/05-interaction.md`).
    #[test]
    fn a_stalled_loop_drops_pulses_rather_than_catching_up() {
        let mut beat = Beat::new();
        beat.bpm = tempo::CEILING_BPM;
        assert!(beat.due(Duration::ZERO));
        // A full second away from the grid is some sixty owed pulses.
        assert!(beat.due(Duration::from_secs(1)));
        assert!(
            !beat.due(Duration::from_secs(1)),
            "the stall was paid off with one pulse, not sixty"
        );
        assert_eq!(
            beat.next,
            Duration::from_secs(1) + tempo::gap(tempo::CEILING_BPM),
            "and the phase starts again from now"
        );
    }

    /// The tempo is a function of the oldest wait and nothing else, and
    /// `None` is the floor rather than a special case
    /// (`docs/05-interaction.md`).
    #[test]
    fn the_tempo_follows_the_oldest_wait() {
        let mut beat = Beat::new();
        beat.retune(Some(tempo::KNEE_MS));
        assert_eq!(beat.bpm, tempo::CEILING_BPM);
        beat.retune(None);
        assert_eq!(beat.bpm, tempo::FLOOR_BPM);
    }

    /// `▲` arms on the press and fires once, however long it is held.
    #[test]
    fn the_home_hold_fires_once_per_press() {
        let mut grip = Grip::default();
        grip.aim(Button::Top(1), true, Duration::ZERO);
        assert!(!grip.home_due(HOME_HOLD.saturating_sub(Duration::from_millis(1))));
        assert!(grip.home_due(HOME_HOLD));
        assert!(
            !grip.home_due(HOME_HOLD * 10),
            "no autorepeat: the press was taken when it fired"
        );

        // A release before the threshold disarms it -- that is a tap on
        // `▲`, which scrolls one row and is `chord.rs`'s business.
        grip.aim(Button::Top(1), true, Duration::ZERO);
        grip.aim(Button::Top(1), false, Duration::from_millis(10));
        assert!(!grip.home_due(HOME_HOLD * 10));
    }

    /// **A departure forgets the whole grip, not half of it.** The
    /// release for everything a finger was holding went out with the
    /// cable, so the modifier, the carried pad and the `▲` hold all have
    /// to go -- otherwise the first grid tap after a replug is a
    /// `Consequence::Dropped`, one `tl.slot` write moving a session to
    /// wherever the finger happened to land (review w3 F1).
    #[test]
    fn a_departure_empties_the_hand_as_well_as_the_modifier() {
        let mut chords = Chords::default();
        chords.toggle_mode();
        assert_eq!(chords.mode(), Action::Move);
        let _held = chords.feed(Action::Move.button(), true);
        assert!(chords.is_held(Action::Move), "the modifier is down");

        let mut grip = Grip {
            hand: Slot::at_absolute(1, 1),
            ..Grip::default()
        };
        grip.aim(Button::Top(1), true, Duration::ZERO);

        grip.unplugged(&mut chords);

        assert!(!chords.is_held(Action::Move), "the modifier went too");
        assert_eq!(grip.hand, None, "the pad was never put down");
        assert!(
            !grip.home_due(HOME_HOLD * 10),
            "no `⌂` from a `▲` that was released by the cable"
        );
        assert_eq!(
            chords.mode(),
            Action::Move,
            "the mode is not a button that can be stuck down, and survives"
        );
    }
}
