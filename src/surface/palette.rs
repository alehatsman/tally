//! The colour vocabulary, verified against hardware in M0.
//!
//! Named rather than numeric at every call site: a bare `9` in render code
//! is a number nobody can check, and [`crate::render`] is the only place
//! that gets to decide what a colour *means*.
//!
//! Every entry here was confirmed by looking at a real Launchpad Mini MK3.
//! Some are no longer used by the colour language — they stay because what
//! the device does with them is a fact worth keeping, and re-deriving it
//! costs a device and an afternoon.

/// A palette entry, `0..=127`.
pub type Entry = u8;

/// Unlit.
///
/// Not black: the pads are white silicone, so an unlit pad still reads as
/// white in a lit room.
pub const OFF: Entry = 0;
/// A dim glow, cooler than "grey" suggests. Currently unused.
pub const DARK_GREY: Entry = 1;
/// White. The page you are on, and an action button while held.
pub const WHITE: Entry = 3;
/// Red. A session that exited non-zero.
pub const RED: Entry = 5;
/// A deeper red, clearly distinct from [`RED`].
///
/// Means *the daemon had a problem* wherever it appears — a `lost` session
/// on the grid, an unreachable anchord on the logo.
pub const DARK_RED: Entry = 7;
/// Orange. **Needs an answer** — the one colour the product lives or dies by.
pub const AMBER: Entry = 9;
/// Yellow with a green cast. Currently unused.
pub const YELLOW: Entry = 13;
/// Green. Alive — pulsing when producing output, static when idle.
pub const GREEN: Entry = 21;
/// A second green, close to [`GREEN`] but distinguishable by eye.
///
/// Unused, and the reason is worth keeping: it *was* "exited clean", and
/// three greens on one grid proved legible yet unmemorable.
pub const DARK_GREEN: Entry = 23;
/// Cyan. A page button holding sessions.
pub const TURQUOISE: Entry = 37;
/// Blue. An action button at rest.
pub const BLUE: Entry = 45;
