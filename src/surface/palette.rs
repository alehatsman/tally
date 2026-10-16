//! The colour vocabulary, verified against hardware in M0.
//!
//! Named rather than numeric at every call site: a bare `9` in render code
//! is a number nobody can check, and `docs/05-interaction.md` is the only
//! place that gets to decide what a colour *means*.

/// A palette entry, 0-127.
pub type Entry = u8;

pub const OFF: Entry = 0;
/// Bound to a spec, nothing running. A dim glow — note that an *unlit* pad
/// is white silicone, not black, so this reads as "lit" rather than "grey".
pub const DARK_GREY: Entry = 1;
pub const WHITE: Entry = 3;
pub const RED: Entry = 5;
pub const DARK_RED: Entry = 7;
/// Needs an answer. The one colour the product lives or dies by.
pub const AMBER: Entry = 9;
pub const YELLOW: Entry = 13;
pub const GREEN: Entry = 21;
/// Exited clean. Close to [`GREEN`] — confirmed distinguishable by eye in
/// M0, but it is the first thing to re-check under different light.
pub const DARK_GREEN: Entry = 23;
pub const TURQUOISE: Entry = 37;
pub const BLUE: Entry = 45;
