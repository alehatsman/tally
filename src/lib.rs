//! tally — a Novation Launchpad Mini MK3 as the front panel for
//! anchor-hosted agent sessions.
//!
//! See `docs/01-scope.md` for what this is and, more usefully, what it is
//! not. The hardware protocol in `docs/02-surface.md` was verified against
//! a real device in M0; this crate implements it.

pub mod anchor;
pub mod device;
pub mod journal;
pub mod lock;
pub mod model;
pub mod render;
pub mod signal;
pub mod slot;
pub mod surface;
pub mod watch;

/// Milliseconds since the unix epoch.
///
/// One clock for the whole program: signals compare timestamps across
/// threads, and two sources of "now" is how those comparisons go subtly
/// wrong.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
