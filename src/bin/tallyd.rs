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
use tally::chord::{Action, Chords, Gesture};
use tally::config::Config;
use tally::device::{Device, Error, Event};
use tally::journal::{self, Journal};
use tally::jump::{Jump, Tmux};
use tally::lock::Lock;
use tally::model::{self, View};
use tally::queue;
use tally::slot::{self, Pages, Slot};
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
fn perform(ctx: &Context<'_>, gesture: Gesture, at: Slot, pages: &Pages, tmux: &impl Jump) {
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
            // Taken before the act, because afterwards anchord has
            // forgotten them and there is nothing left to look up. Purge
            // clears the pad, so it clears the view (`docs/15-views.md`).
            let purging: Vec<String> = if action == Action::Purge {
                ctx.pads.rows(at).to_vec()
            } else {
                Vec::new()
            };
            match ctx.chord(action, at, pages) {
                // The preview already said nothing would happen here, so
                // the log stays quiet too: a line per dimmed pad you
                // brushed is noise in the only file that is read after
                // something broke.
                Ok(Done::Nothing) => {}
                Ok(done) => {
                    eprintln!("tallyd: {action:?} {}:{row}:{col} {done}", at.page);
                    for id in &purging {
                        if let Err(e) = tmux.close(id) {
                            eprintln!("tallyd: could not close the view for {id}: {e}");
                        }
                    }
                }
                Err(e) => eprintln!("tallyd: {action:?} {}:{row}:{col}: {e}", at.page),
            }
        }
        // Handled in the loop: they change where you are, not what the
        // grid holds, and two of them need the queue the loop computes.
        Gesture::PageUp | Gesture::PageDown | Gesture::Back | Gesture::Next | Gesture::Home => {}
    }
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
        Gesture::Tap { .. } | Gesture::Chord { .. } => {}
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
                        let Some(gesture) = chords.feed(button, down) else {
                            continue;
                        };
                        match gesture {
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
                                    pads: model::Pads::new(&grid, &config, &purgeable),
                                };
                                perform(&ctx, gesture, at, &pages, &tmux);
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
        let pads = model::Pads::new(&resolved, &config, &purgeable);
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

        let view = View {
            pages: &pages,
            offset,
            anchord_reachable: reachable,
            now,
            can_go_back: tmux.can_go_back(),
            queued,
            held: chords.modifier(),
            pressed: &pressed,
        };
        let frame = model::frame_from(&pads, &view);
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
