//! The hardware surface: addressing, colour, and the bytes that go on the
//! wire. Everything here is pure — no I/O, no device, no `midir`.
//!
//! This is the internal boundary `docs/01-scope.md` keeps so a second grid
//! is possible later without the rest of the program knowing. It is not
//! generalised for one, because there is no second device in the room.

pub mod addr;
pub mod frame;
pub mod palette;
pub mod sysex;

pub use addr::{Button, SURFACE_LEN, TOP_BUTTONS};
pub use frame::{Cell, Frame};
