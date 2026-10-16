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

use tally::anchor::Client;
use tally::device::{Device, Error, Event};
use tally::journal::{self, Journal};
use tally::jump::{Jump, Tmux};
use tally::lock::Lock;
use tally::model::{self, View};
use tally::slot::Pages;
use tally::surface::Button;
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

    let pages = Pages::default();
    let watchers = Watchers::new(anchor.clone());
    let mut journal = Journal::default();
    // Best effort: a grid that lights up matters more than a log nothing
    // reads back. If it cannot be opened, say so once and carry on.
    let journal_path = journal::default_path();
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
    let mut current = 1u8;
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
    let mut grid: std::collections::BTreeMap<tally::slot::Slot, model::Resolved> =
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
                    Event::Press {
                        button: Button::Page(n),
                        down: true,
                    } => {
                        current = n;
                        eprintln!("tallyd: page {n}");
                    }
                    // A bound slot jumps. An unbound one does nothing yet:
                    // spawning is M3, and a press that silently starts a
                    // process would be worse than one that does nothing.
                    Event::Press {
                        button: Button::Grid { row, col },
                        down: true,
                    } => {
                        let at = tally::slot::Slot {
                            page: current,
                            row,
                            col,
                        };
                        if let Some(r) = grid.get(&at) {
                            eprintln!("tallyd: jump {}:{row}:{col} -> {}", current, r.session);
                            if let Err(e) = tmux.jump(&r.session) {
                                eprintln!("tallyd: {e}");
                            }
                        }
                    }
                    Event::Press { .. } => {}
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

        let view = View {
            pages: &pages,
            current,
            anchord_reachable: reachable,
            now: tally::now_ms(),
        };
        let resolved = model::resolve(&sessions, &watchers.snapshot(), &pages, view.now);
        grid.clone_from(&resolved);
        if let Some(out) = journal_out.as_mut() {
            // Unbuffered on purpose. This is a few lines a day, and a
            // buffer would hold them until a flush that SIGKILL never
            // reaches -- losing exactly the transitions it exists to count.
            if let Err(e) = journal
                .record(&resolved, &pages, view.now, out)
                .and_then(|()| out.flush())
            {
                eprintln!("tallyd: attention journal: {e}");
                journal_out = None;
            }
        }
        let frame = model::frame_from(&resolved, &view);
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
                }
            }
        }

        std::thread::sleep(TICK);
    };

    eprintln!("tallyd: signal {signum}, restoring");
    drop(device); // Drop restores Live mode and verifies it by readback.
    reraise(signum);
}
