//! The colour vocabulary, verified against hardware in M0.
//!
//! Named rather than numeric at every call site: a bare `9` in render code
//! is a number nobody can check, and [`crate::render`] is the only place
//! that gets to decide what a colour *means*.
//!
//! Every entry here was confirmed by looking at a real Launchpad Mini MK3.
//! Two are no longer used by anything — they stay because what the device
//! does with them is a fact worth keeping, and re-deriving it costs a
//! device and an afternoon.

/// A palette entry, `0..=127`.
pub type Entry = u8;

/// Unlit.
///
/// Not black: the pads are white silicone, so an unlit pad still reads as
/// white in a lit room.
pub const OFF: Entry = 0;
/// A dim glow, cooler than "grey" suggests.
///
/// The only dim level in the language, and it says one thing in every
/// place it appears: *present, and not what you are being asked about*
/// — a navigation button with somewhere to go ([`crate::render::arrow`],
/// [`crate::render::overview_button`], [`crate::render::arts`]), a `>`
/// that would launch ([`crate::render::launch`]), a pad a held modifier
/// will skip ([`crate::render::preview`]), and the empty run that marks
/// where the window is standing ([`crate::render::overview`]).
///
/// Brightness answers "where am I" before hue answers anything, which is
/// why the top row spends exactly one bright colour and this carries
/// everything else on it.
pub const DARK_GREY: Entry = 1;
/// White. The brightest thing on the surface, spent on roles that all
/// mean *this button is live right now*: `◄` when there is somewhere to
/// go back to ([`crate::render::back`]), a modifier while it is held
/// ([`crate::render::action`]), and `Session` while the overview is up
/// ([`crate::render::overview_button`]).
///
/// It used to light *the page you are on* as well. That went with the
/// page buttons themselves when the top row became a queue
/// (`docs/16-the-queue.md`), and the roles left are the ones that answer
/// **what is my hand doing** rather than **where am I**.
pub const WHITE: Entry = 3;
/// Red. A session that exited non-zero, the corner resting in Stop, and a
/// pad that hold is about to terminate — the colour of what is about to
/// happen. Grid and corner never confuse, because one is the 8×8 and the
/// other is the right-column corner the silkscreen labels *Stop*.
pub const RED: Entry = 5;
/// A deeper red, clearly distinct from [`RED`].
///
/// Means *the daemon had a problem* wherever it appears — a `lost` session
/// on the grid, an unreachable anchord on the logo — and, under a held
/// Stop, a pad whose ended rows are about to be forgotten
/// (`docs/22-the-mode-button.md`).
pub const DARK_RED: Entry = 7;
/// Orange. **Needs an answer** — the one colour the product lives or dies
/// by, and since `9b2eee5` the only thing on the surface that moves.
pub const AMBER: Entry = 9;
/// Yellow with a green cast.
///
/// The colour language never uses it, which is exactly why the Uprising
/// ends on it: the smile is the one place on this surface yellow goes
/// (`docs/21-the-arts.md`).
pub const YELLOW: Entry = 13;
/// Green. **Alive**, and nothing finer than that.
///
/// It is static, always. Green used to pulse for *producing output* and
/// sit still for idle, and a day beside it reported the motion as tiring
/// — so motion is spent only on [`AMBER`] now, and the cost is that
/// working and quiet look identical
/// (`docs/05-interaction.md#nothing-blinks-and-only-attention-moves`).
pub const GREEN: Entry = 21;
/// A second green, close to [`GREEN`] but distinguishable by eye.
///
/// Unused, and the reason is worth keeping: it *was* "exited clean", and
/// three greens on one grid proved legible yet unmemorable.
pub const DARK_GREEN: Entry = 23;
/// Cyan. **Move at rest, and a pad you could pick up**
/// ([`crate::render::action`], [`crate::render::preview`]).
///
/// It failed once, as a page button lit beside a white current page: both
/// were legible and neither said which was which, the same way the three
/// greens failed, so the row went to one bright thing and [`DARK_GREY`]
/// took the role. One button is not a row and the failure mode does not
/// reach it, which is why the entry came back rather than staying retired
/// (`docs/22-the-mode-button.md#the-colours`).
pub const TURQUOISE: Entry = 37;
/// Blue. Unused: it *was* the action button at rest, until the kill
/// button took the colour of its own consequence.
pub const BLUE: Entry = 45;
