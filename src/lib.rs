//! tally — a Novation Launchpad Mini MK3 as the front panel for
//! anchor-hosted agent sessions.
//!
//! See `docs/01-scope.md` for what this is and, more usefully, what it is
//! not. The hardware protocol in `docs/02-surface.md` was verified against
//! a real device in M0; this crate implements it.

pub mod anchor;
pub mod device;
pub mod lock;
pub mod model;
pub mod render;
pub mod signal;
pub mod slot;
pub mod surface;
