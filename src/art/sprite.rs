use crate::surface::palette::Entry;
use crate::surface::{Button, Cell, Frame};

/// An 8×8 picture, one byte per row, row 0 at the top.
///
/// **Bit 7 is column 1**, so a literal written in binary *is* the sprite —
/// you read the cat off the source. That is the whole reason this is a
/// `[u8; 8]` and not a `u64`.
pub(super) type Sprite = [u8; 8];

/// Paint every set bit of `sprite` in `entry`, leaving the rest alone.
///
/// Leaving the rest alone is what lets a second sprite go on top: a body
/// in one colour, then eyes in white over it.
pub(super) fn draw(frame: &mut Frame, sprite: Sprite, entry: Entry) {
    for row in 1..=8u8 {
        let bits = sprite.get(usize::from(row - 1)).copied().unwrap_or(0);
        for col in 1..=8u8 {
            if bits >> (8 - col) & 1 == 1 {
                frame.set(Button::Grid { row, col }, Cell::Static(entry));
            }
        }
    }
}
