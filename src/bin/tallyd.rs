//! `tallyd` — the daemon that owns the Launchpad.
//!
//! Still a skeleton: there is no anchor client yet, so there are no
//! sessions and the grid is dark. That is not a placeholder, it is the
//! truthful rendering — an unreachable anchord lights the logo dark red
//! rather than showing sixty-four dark pads, because "nothing is running"
//! and "I cannot tell" are different facts (`docs/01-scope.md`).

use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use tally::device::{Device, Event};
use tally::lock::Lock;
use tally::render::{self, Health};
use tally::surface::{Button, Frame};

/// Set by the signal handler, read by the main loop. `0` means no signal.
static SIGNALLED: AtomicI32 = AtomicI32::new(0);

/// How often the loop reconciles. Teleport's cadence for the same job.
const TICK: Duration = Duration::from_millis(100);

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

    let mut device = match Device::open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("tallyd: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("tallyd: Launchpad ready; anchord is not wired yet, so the logo is red");

    let mut frame = Frame::dark();
    // No anchor client yet: we genuinely cannot tell what is running.
    frame.set(
        Button::Logo,
        render::logo(Health {
            anchord_reachable: false,
            offscreen_attention: false,
        }),
    );
    if let Err(e) = device.repaint(&frame) {
        eprintln!("tallyd: {e}");
        return ExitCode::FAILURE;
    }

    let signum = loop {
        let signum = SIGNALLED.load(Ordering::Relaxed);
        if signum != 0 {
            break signum;
        }
        for event in device.poll() {
            if let Event::Press { button, down } = event {
                eprintln!("tallyd: {button:?} {}", if down { "down" } else { "up" });
            }
        }
        std::thread::sleep(TICK);
    };

    eprintln!("tallyd: signal {signum}, restoring");
    drop(device); // Drop restores Live mode and verifies it by readback.
    reraise(signum);
}
