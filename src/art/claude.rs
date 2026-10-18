use super::sprite::draw;
use super::sprite::Sprite;
use crate::surface::palette::AMBER;
use crate::surface::Frame;

/// Frames per spoke position. Four positions make a turn, so the mark
/// rotates about once and a quarter a second.
pub(super) const SPARK_FRAME: u32 = 6;
/// Frames per bloom step. Faster than the turn, because the bloom is an
/// arrival and an arrival should not be gradual.
pub(super) const BLOOM_FRAME: u32 = 5;
/// How long before it blooms again.
pub(super) const CLAUDE_LOOP: u32 = 240;

/// The mark arriving: a single lit centre, then a core, then open.
pub(super) const BLOOM: [Sprite; 2] = [
    [0, 0, 0, 0b0001_1000, 0b0001_1000, 0, 0, 0],
    [
        0,
        0,
        0b0011_1100,
        0b0011_1100,
        0b0011_1100,
        0b0011_1100,
        0,
        0,
    ],
];

/// The turn, as four spoke positions.
///
/// Orthogonal, then all eight, then diagonal, then all eight again —
/// which reads as rotation and is not, because nothing on an 8×8 grid can
/// rotate by less than 45° and a mark that jumped would read as a glitch.
/// The full asterisk between them is the frame that sells it.
pub(super) const SPARK: [Sprite; 4] = [
    // the arms
    [
        0b0001_1000,
        0b0001_1000,
        0b0001_1000,
        0b1111_1111,
        0b1111_1111,
        0b0001_1000,
        0b0001_1000,
        0b0001_1000,
    ],
    // The mark itself: a rounded sunburst. The diagonals stop short of
    // the corners on purpose — that taper is what gives it a circular
    // silhouette instead of the square one a plain asterisk has, and it
    // is the whole difference between Claude's mark and a `*`.
    [
        0b0001_1000,
        0b0101_1010,
        0b0011_1100,
        0b1111_1111,
        0b1111_1111,
        0b0011_1100,
        0b0101_1010,
        0b0001_1000,
    ],
    // turned 45°
    [
        0b1000_0001,
        0b0100_0010,
        0b0010_0100,
        0b0001_1000,
        0b0001_1000,
        0b0010_0100,
        0b0100_0010,
        0b1000_0001,
    ],
    [
        0b0001_1000,
        0b0101_1010,
        0b0011_1100,
        0b1111_1111,
        0b1111_1111,
        0b0011_1100,
        0b0101_1010,
        0b0001_1000,
    ],
];

/// Claude's mark, in Claude's colour.
///
/// Amber, which is the surface's reserved *answer me*. Borrowing it is
/// safe for the reason every piece here borrows something: while an art
/// is playing the grid means nothing at all, and the logo carries the
/// attention instead (`docs/05-interaction.md`). Fire and the ember trail
/// spend the same amber, and the Uprising spends it telling a joke about
/// what it normally means.
pub(super) fn paint(n: u32, frame: &mut Frame) {
    let t = n % CLAUDE_LOOP;
    let opening = BLOOM_FRAME * 2;
    let sprite = if t < opening {
        let step = usize::try_from(t / BLOOM_FRAME).unwrap_or(0);
        BLOOM.get(step).copied().unwrap_or(BLOOM[1])
    } else {
        let step = usize::try_from((t - opening) / SPARK_FRAME % 4).unwrap_or(0);
        SPARK.get(step).copied().unwrap_or(SPARK[1])
    };
    draw(frame, sprite, AMBER);
}
