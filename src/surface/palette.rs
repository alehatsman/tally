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
/// A dim glow, cooler than "grey" suggests.
///
/// The only dim level in the language, and it says one thing in both
/// places it appears: *present, and not what you are being asked about*
/// — a page button holding sessions you are not standing on
/// ([`crate::render::page`]), and a pad a held modifier will skip
/// ([`crate::render::preview`]). Brightness answers "where am I" before
/// hue answers anything, which is why this took the page-button role
/// from [`TURQUOISE`].
pub const DARK_GREY: Entry = 1;
/// White. The brightest thing on the surface, spent on three roles that
/// all mean *this is where you are, or what you are doing*: the page you
/// are on ([`crate::render::page`]), an action button while held, and the
/// pad a held mute would **un**mute ([`crate::render::preview`]). The
/// last shares [`BLUE`]'s job of saying you are about to change how tally
/// treats a pad rather than what is running on it.
pub const WHITE: Entry = 3;
/// Red. A session that exited non-zero, and the kill button at rest —
/// the colour of what that button does. The two never confuse because
/// one is on the grid and the other is the right-column corner the
/// silkscreen labels *Stop*.
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
/// Cyan.
///
/// Unused, and the reason is worth keeping: it *was* "a page button
/// holding sessions", lit beside a white current page. That failed the
/// way the three greens failed — both were legible and neither said
/// which was which — so the row went to one bright thing and
/// [`DARK_GREY`] took the role ([`crate::render::page`]). It survives
/// only in the encoder fixtures (`src/surface/frame.rs`), which need a
/// second colour and do not care which.
pub const TURQUOISE: Entry = 37;
/// Blue. Unused: it *was* the action button at rest, until the kill
/// button took the colour of its own consequence.
pub const BLUE: Entry = 45;
