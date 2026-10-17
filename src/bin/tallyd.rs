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

use std::fs::OpenOptions;
use std::io::Write;

use tally::act::{Context, Done};
use tally::anchor::Client;
use tally::art::{Piece, Reel};
use tally::chord::{Action, Chords, Gesture};
use tally::config::Config;
use tally::device::{Device, Error, Event};
use tally::journal::{self, Journal};
use tally::jump::{Jump, Tmux};
use tally::lock::Lock;
use tally::model::{self, View};
use tally::queue;
use tally::slot::{self, Pages, Slot};
use tally::surface::Button;
use tally::tempo;
use tally::watch::Watchers;

/// Set by the signal handler, read by the main loop. `0` means no signal.
static SIGNALLED: AtomicI32 = AtomicI32::new(0);

/// How often the loop asks anchord for the session list. teleport's
/// cadence for the same job; presses are not on this clock.
const RECONCILE: Duration = Duration::from_secs(2);
/// How often the loop wakes. Input is drained at this rate so a press
/// feels immediate rather than waiting out a reconcile.
const TICK: Duration = Duration::from_millis(16);
/// How long without a reply before the device counts as gone.
///
/// Generous against the ~2ms warm round trip, because the cost of being
/// wrong is a needless reconnect, and three missed probes is not a blip.
const DEVICE_SILENCE: Duration = Duration::from_secs(7);

/// How long `▲` must be held before it means *all the way up*.
///
/// `⌂` lost its own button when `Session` became the overview modifier,
/// and it landed here rather than being folded into the overview because
/// the one jump the journal describes — the deep row back to row 1 — is
/// worth a single press (`docs/20-the-overview.md`).
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
/// deliberate tap is about 100 ms (`docs/22-the-mode-button.md`).
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
/// (`docs/08-lifecycle.md`).
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

/// Carry out one gesture and say what happened.
///
/// A jump is tmux's and everything else is anchord's, which is the same
/// split as `docs/06-jump.md`: `tally::act` never opens a terminal and
/// `tally::jump` never opens a socket.
///
/// Errors are printed and dropped. The next reconcile redraws from a
/// fresh `list`, so a kill that did not land shows as a pad that is still
/// green — the truth, and better than an error state invented for the
/// surface.
/// The `CC19` double-tap, tracked across presses.
///
/// A tap is a press and a release close together with **no chord fired
/// in between** — which is exactly what distinguishes it from an aborted
/// hold, the gesture chord rule 4 promises will do nothing. Two of them
/// inside [`DOUBLE_TAP`] toggle the mode.
///
/// The clock is here and not in `chord.rs` because that module is pure —
/// presses in, gestures out, no `now` — and that purity is what lets the
/// four chord rules be tested without a device or a timeline. Same shape
/// as `home_press`, and the same precedent (`docs/20-the-overview.md`).
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
        // An aborted hold, or a hold that did something. Neither is a
        // tap, and a long press must not leave a half-double behind for
        // the next one to pair with.
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

/// `CC19` went down or came up. Returns whether the surface changed.
///
/// Extracted rather than inlined because `main` is already at the
/// cognitive-complexity cap, and because the two things that happen on
/// this one button — a double-tap toggling the verb, and a release
/// emptying the hand — are unrelated to each other and to everything
/// else in the press loop.
fn corner_event(
    corner: &mut Corner,
    chords: &mut Chords,
    hand: &mut Option<Slot>,
    button: Button,
    down: bool,
    elapsed: Duration,
) -> bool {
    if button != Action::Stop.button() {
        return false;
    }
    let toggled = corner.feed(down, elapsed);
    if toggled {
        chords.toggle_mode();
        eprintln!("tallyd: corner {:?}", chords.mode());
    }
    // Letting go empties the hand. A pick-up you did not put down must
    // leave the board exactly as it was, which is chord rule 4 doing the
    // job it was put there for.
    if !down {
        *hand = None;
    }
    toggled
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
    /// Fold this into the loop's hand.
    ///
    /// A method rather than a `match` at the call site for the same
    /// reason [`corner_event`] exists: `main` is at the
    /// cognitive-complexity cap and this is the kind of three-arm
    /// bookkeeping that belongs next to the enum it is about.
    fn apply(self, hand: &mut Option<Slot>) {
        match self {
            Self::Unchanged => {}
            Self::Took(at) => *hand = Some(at),
            Self::Emptied => *hand = None,
        }
    }
}

fn perform(ctx: &Context<'_>, gesture: Gesture, at: Slot, pages: &Pages, tmux: &impl Jump) -> Hand {
    let (row, col) = (at.row, at.col);
    match gesture {
        Gesture::Tap { .. } => match ctx.tap(at, pages) {
            // Occupied: this is a jump, and jumps are not anchord's.
            Ok(None) => {
                if let Some(r) = ctx.pads.grid().get(&at) {
                    eprintln!("tallyd: jump {}:{row}:{col} -> {}", at.page, r.session);
                    // The address names the window. Unnamed pages cannot
                    // happen for a slot that parsed, but an empty string
                    // is a legal name and falls back to the old form.
                    let label = at.label(pages).unwrap_or_default();
                    if let Err(e) = tmux.jump(&r.session, &label) {
                        eprintln!("tallyd: {e}");
                    }
                }
            }
            Ok(Some(Done::Nothing)) => {}
            Ok(Some(done)) => eprintln!("tallyd: {}:{row}:{col} {done}", at.page),
            Err(e) => eprintln!("tallyd: spawn {}:{row}:{col}: {e}", at.page),
        },
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
        | Gesture::Goto { .. }
        | Gesture::Arts
        | Gesture::Launch { .. } => {}
    }
    Hand::Unchanged
}

/// Write one navigation line, dropping the sink if it fails.
///
/// Same rule as the state lines: a journal that cannot be written is
/// reported once and then stops being tried, because the alternative is a
/// line per press in the only file read after something broke.
fn log_nav(
    out: &mut Option<std::fs::File>,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) {
    let Some(sink) = out.as_mut() else {
        return;
    };
    if let Err(e) = write(sink).and_then(|()| sink.flush()) {
        eprintln!("tallyd: attention journal: {e}");
        *out = None;
    }
}

/// The labels a `>` launch carries.
///
/// `tl.slot` and nothing else. **No `tl.spec`**, for the same reason
/// `tally spawn` writes none: a spec label is a name looked up in
/// `slots.toml`, and one nothing declares would be a label that lies
/// (`docs/11-cli.md`).
fn launch_labels(at: Slot, pages: &Pages) -> std::collections::BTreeMap<String, String> {
    let mut labels = std::collections::BTreeMap::new();
    if let Some(address) = at.label(pages) {
        let _slot = labels.insert(tally::anchor::LABEL_SLOT.to_owned(), address);
    }
    labels
}

/// Move along the reel, and say where it went.
///
/// `None` is both ends of the cycle: the working grid is what you get
/// before the first press and after the last (`docs/21-the-arts.md`).
fn next_art(reel: &mut Option<Reel>) {
    *reel = Piece::after(reel.as_ref().map(Reel::piece)).map(Reel::new);
    match reel.as_ref() {
        Some(r) => eprintln!("tallyd: arts {}", r.piece().name()),
        None => eprintln!("tallyd: arts off"),
    }
}

/// The two top-row buttons the art treats specially: the one that moves
/// along the reel, and the one that is allowed through it.
const ARTS_BUTTON: Button = Button::Top(6);
/// `►`. See [`left_art`].
const NEXT_BUTTON: Button = Button::Top(4);

/// End the art if this press should, and say whether the press itself is
/// then **swallowed**.
///
/// Takes `repaint` the way [`beat`] takes its cursor, so `main` is left
/// with one branch rather than three — the complexity cap here is real
/// and this loop is already against it.
///
/// **Every press except `Drums`, not every gesture.** This is read from
/// the raw button and before `chords.feed`, because `feed` answers `None`
/// for a held modifier — so a `CC19` press used to reach the corner's
/// double-tap clock and silently toggle Stop/Move behind a plasma field,
/// where nothing on the surface could show that it had changed. You
/// reached for the grid; the art is over, and nothing else happened.
///
/// `►` is the one that still acts: making you press it twice would be the
/// toy charging rent on the one gesture this product exists to provide
/// (`docs/21-the-arts.md`).
fn left_art(reel: &mut Option<Reel>, repaint: &mut bool, button: Button, down: bool) -> bool {
    if !down || reel.is_none() || button == ARTS_BUTTON {
        return false;
    }
    *reel = None;
    *repaint = true;
    eprintln!("tallyd: arts off");
    button != NEXT_BUTTON
}

/// Advance the art one frame, on every other wake.
///
/// ~31 fps off a 16 ms tick: smooth to the eye at half the bytes
/// (`docs/21-the-arts.md#the-frame-rate`).
fn advance_art(reel: Option<&mut Reel>, wake: &mut bool) {
    *wake = !*wake;
    if let Some(r) = reel.filter(|_| *wake) {
        r.tick();
    }
}

/// The surface while an art is playing (`docs/21-the-arts.md`).
///
/// The art owns the sixty-four pads and nothing else. Three lights are
/// not its to take: `►`, because the one gesture this product exists to
/// provide does not go away because the grid is busy; `Drums`, because a
/// mode has to say it is on; and the logo, which carries **all** the
/// attention now rather than only the scrolled-off kind — a grid showing
/// a plasma field is showing none of it.
fn art_surface(reel: &Reel, reachable: bool, queued: usize) -> tally::surface::Frame {
    let mut frame = reel.frame();
    frame.set(Button::Top(4), tally::render::next(queued > 0));
    frame.set(Button::Top(6), tally::render::arts(true));
    frame.set(
        Button::Logo,
        tally::render::logo(tally::render::Health {
            anchord_reachable: reachable,
            offscreen_attention: queued > 0,
        }),
    );
    frame
}

/// Send the beat the surface breathes to.
///
/// One byte, up to fifty-six times a second. Due pulses are sent and
/// missed ones are **dropped, not caught up**: a burst after a stall would
/// visibly sprint the breathing, and a skipped beat is the honest report
/// of a loop that was busy (`docs/17-the-tempo.md`).
fn beat(
    device: Option<&mut Device>,
    elapsed: Duration,
    bpm: u64,
    started: &mut bool,
    next: &mut Duration,
) {
    let Some(d) = device else {
        // Unplugged. Start again when it comes back.
        *started = false;
        return;
    };
    if !*started {
        *started = d.clock_start().is_ok();
        *next = elapsed;
    }
    if elapsed < *next {
        return;
    }
    if let Err(e) = d.clock() {
        eprintln!("tallyd: clock: {e}");
    }
    *next = elapsed + tempo::gap(bpm);
}

/// Spawn into a grid row from its `>` button. Reports whether the surface
/// needs repainting.
///
/// Lifted out of `main` to keep it inside its complexity budget. `grid`
/// is **last tick's** resolution — the same one that lit the button, so
/// the press and the preview cannot disagree about what the row holds.
/// The preview said so first: `None` is a dark button, and a dark button
/// does nothing.
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

/// Put the window where an overview pad points.
///
/// Lifted out of `main` for the same reason `navigate` was: it touches
/// nothing but the offset and the journal, and `main` has a complexity
/// budget that this pushed over.
///
/// `grid` is **last tick's** resolution — the map the finger was looking
/// at when it landed, which is the only honest thing to count `lit` from.
fn goto(
    row: u8,
    col: u8,
    offset: &mut u8,
    grid: &std::collections::BTreeMap<Slot, model::Resolved>,
    journal_out: &mut Option<std::fs::File>,
) {
    let to = View::overview_row(row, col);
    let from = *offset;
    // Clamped, so the deep rows land somewhere real rather than hanging
    // the window off the end. Row 60 arrives fifth, which is honest
    // (`docs/20-the-overview.md`).
    *offset = to.saturating_sub(1).min(slot::MAX_OFFSET);
    let lit = (1..=slot::ROWS)
        .filter(|abs| model::row_summary(grid, *abs).is_some())
        .count();
    eprintln!("tallyd: goto row {to} ({lit} rows lit)");
    let now = tally::now_ms();
    log_nav(journal_out, |out| {
        journal::nav_goto(out, now, to, from, lit)
    });
}

/// The four top-row gestures: move the window, or move you.
///
/// Lifted out of `main` because none of them touches anchord and all of
/// them act on the queue the last tick computed -- `head` and `queued` are
/// deliberately **stale**, because they are what the finger was looking at
/// when it landed. Recomputing here would act on a grid the eye never saw.
fn navigate(
    gesture: Gesture,
    offset: &mut u8,
    head: Option<&queue::Head>,
    queued: usize,
    pages: &Pages,
    tmux: &impl Jump,
    journal_out: &mut Option<std::fs::File>,
) {
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
                eprintln!("tallyd: rows {}-{}", n + 1, n + 8);
            }
        }
        Gesture::Home => {
            if *offset != 0 {
                *offset = 0;
                eprintln!("tallyd: home");
            }
        }
        Gesture::Next => {
            let Some(h) = head else {
                return;
            };
            let at = h.at.label(pages).unwrap_or_default();
            eprintln!("tallyd: next -> {at} ({queued} waiting)");
            if let Err(e) = tmux.jump(&h.session, &at) {
                eprintln!("tallyd: {e}");
            }
            // Follow the jump with the eyes, by the smallest scroll that
            // shows where it landed.
            *offset = View::offset_showing(h.at, *offset);
            let now = tally::now_ms();
            log_nav(journal_out, |out| {
                journal::nav_next(out, now, &at, now.saturating_sub(h.since), queued)
            });
        }
        Gesture::Back => match tmux.back() {
            Ok(moved) => {
                eprintln!(
                    "tallyd: back{}",
                    if moved { "" } else { " (nowhere to go)" }
                );
                let now = tally::now_ms();
                log_nav(journal_out, |out| journal::nav_back(out, now, moved));
            }
            Err(e) => eprintln!("tallyd: {e}"),
        },
        Gesture::Tap { .. }
        | Gesture::Chord { .. }
        | Gesture::Launch { .. }
        | Gesture::Arts
        | Gesture::Goto { .. } => {}
    }
}

fn main() -> ExitCode {
    install_handlers();

    let lock_path = Lock::default_path();
    let _lock = match Lock::acquire(&lock_path) {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!(
                "tallyd: another instance holds {}: {e}",
                lock_path.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let anchor = Client::new(Client::default_path());
    eprintln!("tallyd: anchord at {}", anchor.path().display());

    // No device is a normal state, not a failure. `tallyd` is started by
    // launchd and may well come up before the Launchpad is plugged in --
    // and the thing it is watching is anchord, which it can do blind.
    let mut device: Option<Device> = None;
    let mut last_device_try = Duration::ZERO;

    // A broken config is reported and then ignored. Refusing to start
    // would take away the watching to punish a mistake in the spawning,
    // and the watching is the half that is load-bearing.
    let config_path = Config::default_path();
    let config = Config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("tallyd: {}: {e}", config_path.display());
        eprintln!("tallyd: running on defaults; no pad will spawn anything");
        Config::default()
    });
    eprintln!(
        "tallyd: {} pages, {} slots from {}",
        config.pages().len(),
        config.len(),
        config_path.display()
    );
    let pages = config.pages().clone();

    let mut chords = Chords::default();
    let watchers = Watchers::new(anchor.clone());
    // Best effort: a grid that lights up matters more than a log. If it
    // cannot be opened, say so once and carry on.
    let journal_path = journal::default_path();
    // Read back who was already asking when we last stopped. Without this
    // a restart writes a second rising edge for a slot that never stopped
    // asking, and times it from the restart rather than from the question
    // (`docs/14-attention.md`). The one number M1 exists to produce is a
    // count of these, so it has to count them once.
    let mut journal = Journal::resumed(&journal_path, &pages);
    let mut journal_out = journal_path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&journal_path)
        })
        .map_err(|e| {
            eprintln!(
                "tallyd: no attention journal at {}: {e}",
                journal_path.display()
            );
        })
        .ok();
    let mut sessions = Vec::new();
    let mut reachable = false;
    let mut offset = 0u8;
    // Set while `▲` is down, taken when the hold fires.
    let mut home_press: Option<Duration> = None;
    // The `CC19` double-tap, and the pad a held `Move` is carrying. Both
    // live here for the same reason `home_press` does: they are timed or
    // multi-press gestures, and `chord.rs` is pure on purpose.
    let mut corner = Corner::default();
    let mut hand: Option<Slot> = None;
    // The queue, as of the last tick: what the finger was looking at when
    // it landed. Recomputing on the press would act on a grid the eye
    // never saw.
    let mut head: Option<queue::Head> = None;
    let mut queued = 0usize;
    // The beat. `bpm` is recomputed each reconcile from the oldest thing
    // waiting; `next_clock` is when the next `0xF8` is due.
    let mut bpm = tempo::FLOOR_BPM;
    let mut next_clock = Duration::ZERO;
    let mut clock_started = false;
    let mut last_reconcile = Duration::ZERO;
    // The art, if one is playing (`docs/21-the-arts.md`). `None` is the
    // working grid, and it is where the reel starts and ends.
    let mut reel: Option<Reel> = None;
    // Art advances every other wake: ~31 fps off a 16 ms tick.
    let mut art_wake = false;
    let mut repaint = true;
    let mut last_reply = Duration::ZERO;
    let tmux = Tmux::default();
    if !tmux.server_running() {
        // Said plainly, not fatal. A missing server is the normal state at
        // login; the grid is still truthful without one, and the jump
        // starts a server when it has to.
        eprintln!("tallyd: no tmux server yet; a jump will start one");
    }
    // What was on the grid last tick, so a press knows what it hit.
    let mut grid: std::collections::BTreeMap<Slot, model::Resolved> =
        std::collections::BTreeMap::new();
    // Rows anchord would forget, including ones nothing draws: a clean
    // exit keeps its `tl.slot` behind a dark pad, and the purge chord is
    // the only way to see or clear it.
    let mut purgeable: std::collections::BTreeMap<Slot, Vec<String>> =
        std::collections::BTreeMap::new();
    let started = std::time::Instant::now();

    let signum = loop {
        let signum = SIGNALLED.load(Ordering::Relaxed);
        if signum != 0 {
            break signum;
        }

        let elapsed = started.elapsed();

        // Poll for the device. `midir` enumerates ports; it does not tell
        // you when one appears, so this is the same reconcile shape as
        // everything else here.
        if device.is_none() && elapsed.saturating_sub(last_device_try) >= RECONCILE {
            last_device_try = elapsed;
            match Device::open() {
                Ok(d) => {
                    eprintln!("tallyd: Launchpad connected");
                    device = Some(d);
                    repaint = true;
                    last_reply = elapsed;
                }
                // Unplugged, or plugged in and still settling: a port can
                // enumerate a moment before the device answers an inquiry.
                // Both are ordinary and both resolve on the next try, so
                // neither gets a line every two seconds.
                Err(Error::NotFound | Error::NotALaunchpad) => {}
                Err(e) => eprintln!("tallyd: {e}"),
            }
        }

        // Presses are event-driven, not on the reconcile clock.
        if let Some(d) = device.as_mut() {
            for event in d.poll().collect::<Vec<_>>() {
                match event {
                    Event::Press { button, down } => {
                        // Captured before `feed`, which answers `None` for
                        // plenty of presses and would skip this with the
                        // `continue` below. The clock deliberately lives
                        // here and not in `chord.rs`: that module is pure
                        // and has no `now` (`docs/20-the-overview.md`).
                        if button == Button::Top(1) {
                            home_press = down.then_some(elapsed);
                        }
                        // Before the corner's clock and before `feed`:
                        // an art swallows the press whole, so neither
                        // gets to act on one the eye never aimed at a
                        // session (`docs/21-the-arts.md`).
                        if left_art(&mut reel, &mut repaint, button, down) {
                            continue;
                        }
                        // Same reason, same place: a double-tap is two
                        // presses close in time, and the release that
                        // completes one answers `None` from `feed`.
                        repaint |= corner_event(
                            &mut corner,
                            &mut chords,
                            &mut hand,
                            button,
                            down,
                            elapsed,
                        );
                        let Some(gesture) = chords.feed(button, down) else {
                            continue;
                        };
                        corner.saw(gesture);
                        match gesture {
                            Gesture::Arts => {
                                next_art(&mut reel);
                                repaint = true;
                            }
                            Gesture::Launch { row } => {
                                repaint |= launch(
                                    offset.saturating_add(row),
                                    &sessions,
                                    &pages,
                                    &grid,
                                    &anchor,
                                );
                            }
                            Gesture::Goto { row, col } => {
                                goto(row, col, &mut offset, &grid, &mut journal_out);
                            }
                            Gesture::PageUp
                            | Gesture::PageDown
                            | Gesture::Home
                            | Gesture::Next
                            | Gesture::Back => navigate(
                                gesture,
                                &mut offset,
                                head.as_ref(),
                                queued,
                                &pages,
                                &tmux,
                                &mut journal_out,
                            ),
                            Gesture::Tap { row, col } | Gesture::Chord { row, col, .. } => {
                                // The window turns a pad into an address.
                                // Past the end of the address space there
                                // is no pad, and nothing happens.
                                let Some(at) = Slot::at_absolute(offset + row, col) else {
                                    continue;
                                };
                                let ctx = Context {
                                    client: &anchor,
                                    // Last tick's grid: what the pad the
                                    // finger landed on was showing, which
                                    // is the only honest thing to act on.
                                    pads: model::Pads::new(&grid, &config, &purgeable, hand),
                                };
                                perform(&ctx, gesture, at, &pages, &tmux).apply(&mut hand);
                                // Whatever it was, the grid is now stale.
                                // Pull the list forward rather than
                                // leaving the pad wrong for two seconds.
                                last_reconcile = Duration::ZERO;
                            }
                        }
                    }
                    Event::Reply(r) => {
                        last_reply = elapsed;
                        // It answered Live mode: it was power-cycled under
                        // us. Put it back and repaint from scratch.
                        if tally::surface::sysex::programmer_mode_from(&r) == Some(false) {
                            eprintln!("tallyd: Launchpad came back in Live mode, reprogramming");
                            if let Err(e) = d.reprogram() {
                                eprintln!("tallyd: {e}");
                            }
                            repaint = true;
                        }
                    }
                }
            }

            // Liveness probe on the reconcile clock. A failed write is not
            // how you learn the device is gone -- CoreMIDI accepts sends to
            // a removed endpoint without complaint -- so absence of a reply
            // is the only signal there is.
            if elapsed.saturating_sub(last_device_try) >= RECONCILE {
                last_device_try = elapsed;
                if let Err(e) = d.ping() {
                    eprintln!("tallyd: {e}");
                }
            }
            if elapsed.saturating_sub(last_reply) >= DEVICE_SILENCE {
                eprintln!("tallyd: Launchpad stopped answering, dropping it");
                // It is gone, so do not try to restore it on the way out
                // and do not warn that the restore failed.
                d.abandon();
                device = None;
                repaint = true;
                // The release for anything held went out with the cable.
                // Without this the first tap after a replug is a kill.
                chords.clear();
            }
        }

        if elapsed.saturating_sub(last_reconcile) >= RECONCILE || last_reconcile.is_zero() {
            last_reconcile = elapsed;
            match anchor.list() {
                Ok(list) => {
                    if !reachable {
                        eprintln!("tallyd: anchord reachable, {} sessions", list.len());
                    }
                    watchers.reconcile(&list, &pages, tally::now_ms());
                    watchers.retain_known(&list);
                    sessions = list;
                    reachable = true;
                }
                Err(e) => {
                    if reachable {
                        eprintln!("tallyd: {e}");
                    }
                    // Deliberately keep the previous `sessions`: the grid
                    // freezes rather than lying about what is running.
                    reachable = false;
                }
            }
        }

        purgeable = model::purgeable(&sessions, &pages);
        let pressed: std::collections::BTreeSet<Action> = Action::all()
            .into_iter()
            .filter(|&a| chords.is_held(a))
            .collect();
        let now = tally::now_ms();
        let resolved = model::resolve(&sessions, &watchers.snapshot(), &pages, now);
        grid.clone_from(&resolved);
        if let Some(out) = journal_out.as_mut() {
            // Unbuffered on purpose. This is a few lines a day, and a
            // buffer would hold them until a flush that SIGKILL never
            // reaches -- losing exactly the transitions it exists to count.
            if let Err(e) = journal
                .record(&resolved, &pages, now, out)
                .and_then(|()| out.flush())
            {
                eprintln!("tallyd: attention journal: {e}");
                journal_out = None;
            }
        }

        // The queue, after the journal has been brought up to date: the
        // map it keeps *is* the queue, ordered by when each slot started
        // asking (`docs/16-the-queue.md`).
        let pads = model::Pads::new(&resolved, &config, &purgeable, hand);
        let viewing: std::collections::BTreeSet<Slot> = tmux
            .viewing()
            .iter()
            .filter_map(|id| resolved.values().find(|r| &r.session == id))
            .map(|r| r.slot)
            .collect();
        head = queue::head(journal.waiting(), &pads, &viewing);
        queued = queue::depth(journal.waiting(), &pads, &viewing);
        // How fast the surface breathes: the age of the oldest thing
        // waiting (`docs/17-the-tempo.md`). The ramp still runs on the
        // device -- only the beat is ours.
        let was = bpm;
        bpm = tempo::bpm(queue::oldest(journal.waiting(), &pads).map(|since| now - since));
        if bpm != was {
            eprintln!("tallyd: tempo {bpm} bpm");
        }

        // Which `>` would act, from the same resolution the press uses.
        // `▲` held past the threshold means *all the way up*. Taking the
        // press out of `home_press` latches it: one `⌂` per press, no
        // autorepeat, and nothing more until the finger lifts and lands
        // again.
        if home_press.is_some_and(|since| elapsed.saturating_sub(since) >= HOME_HOLD) {
            home_press = None;
            navigate(
                Gesture::Home,
                &mut offset,
                head.as_ref(),
                queued,
                &pages,
                &tmux,
                &mut journal_out,
            );
        }

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
            anchord_reachable: reachable,
            now,
            can_go_back: tmux.can_go_back(),
            queued,
            held: chords.modifier(),
            hand,
            overview: chords.overview(),
            art: reel.is_some(),
            pressed: &pressed,
        };
        let frame = model::frame_from(&pads, &view);
        advance_art(reel.as_mut(), &mut art_wake);
        let frame = reel
            .as_ref()
            .map_or(frame, |r| art_surface(r, reachable, queued));
        if let Some(d) = device.as_mut() {
            // A full repaint on (re)connect: the device may have been
            // unplugged, power-cycled, or left in another process's idea
            // of a frame, so the first thing we put on it is total.
            let wrote = if repaint {
                d.repaint(&frame)
            } else {
                d.render(&frame)
            };
            match wrote {
                Ok(()) => repaint = false,
                Err(e) => {
                    // The write failed, which on this stack means the port
                    // went away. Drop it and go back to polling; the pads
                    // are dark anyway, because the device is gone.
                    eprintln!("tallyd: Launchpad disconnected ({e})");
                    device = None;
                    repaint = true;
                    chords.clear();
                }
            }
        }

        beat(
            device.as_mut(),
            elapsed,
            bpm,
            &mut clock_started,
            &mut next_clock,
        );

        std::thread::sleep(TICK);
    };

    eprintln!("tallyd: signal {signum}, restoring");
    // Hand the beat back before letting go of the device, so a grid left
    // behind breathes at its own 120 rather than holding our last tempo.
    // Only on a *clean* exit: a `tallyd` that is killed stops the clock
    // dead, and the frozen breathing is the point
    // (`docs/17-the-tempo.md`).
    if let Some(d) = device.as_mut() {
        if let Err(e) = d.clock_stop() {
            eprintln!("tallyd: clock stop: {e}");
        }
    }
    drop(device); // Drop restores Live mode and verifies it by readback.
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
    /// (`docs/22-the-mode-button.md#changing-the-mode`).
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
}
