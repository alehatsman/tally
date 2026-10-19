//! The bridge: the Launchpad on a Windows desk, `tallyd` in WSL.
//!
//! USB does not cross the WSL line, so the process is cut in two:
//! `tallybridge` runs on Windows and owns the MIDI port, `tallyd` runs in
//! WSL and owns everything else, and raw MIDI crosses a loopback TCP
//! socket between them (`docs/13-bridge.md`).
//!
//! **The bridge moves bytes.** It holds no model, no chord state, no frame
//! diffing and no idea what a pad means. The one deliberate exception is
//! putting the device back in Live mode when `tallyd` disappears, because
//! that is the moment `tallyd` can no longer do it ([`session::restore`]).
//!
//! This library is the part both ends compile: [`wire`] is the framing,
//! [`session`] is what the Windows end does with one connection. It is
//! std-only and has no idea what MIDI library sits under it.

pub mod session;
pub mod wire;
