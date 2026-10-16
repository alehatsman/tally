//! tally — a Novation Launchpad Mini MK3 as the front panel for
//! anchor-hosted agent sessions.
//!
//! See `docs/01-scope.md` for what this is and, more usefully, what it is
//! not. The hardware protocol in `docs/02-surface.md` was verified against
//! a real device in M0; this crate implements it.

pub mod act;
pub mod anchor;
pub mod attention;
pub mod chord;
pub mod cli;
pub mod config;
pub mod device;
pub mod journal;
pub mod jump;
pub mod lock;
pub mod model;
pub mod osc;
pub mod queue;
pub mod render;
pub mod signal;
pub mod slot;
pub mod surface;
pub mod tempo;
pub mod watch;

/// An environment variable, treating empty as unset.
///
/// `VAR=` is not `VAR=/some/path`, and `var_os` reports both as `Some`.
/// Taking an empty value literally turns an absolute path into a relative
/// one — which is how `tallyd` once looked for anchord's socket at
/// `anchor.sock` in its own working directory and reported the daemon
/// unreachable. anchor's own `paths.rs` filters empties; this is that.
pub fn env_path(key: &str) -> Option<std::path::PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
}

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
