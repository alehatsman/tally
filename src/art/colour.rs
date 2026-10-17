use crate::surface::palette::{Entry, AMBER, DARK_GREY, DARK_RED, OFF, RED, WHITE, YELLOW};
use crate::surface::{Button, Cell, Frame};

/// The Mini MK3's bright-hue column, red round to pink.
///
/// Read off the manual's palette table and **not** eyeballed one by one,
/// which is the difference between this and [`crate::surface::palette`].
/// For a colour that means something that would be unacceptable; for a
/// plasma field it is fine, and nothing here asserts what any of them
/// looks like (`docs/21-the-arts.md`).
pub(super) const HUES: [Entry; 15] = [5, 9, 13, 17, 21, 25, 29, 33, 37, 41, 45, 49, 53, 57, 61];

/// A cooling ember, brightest first. [`Piece::Life`] draws its wake with
/// this; the last entry is the dimmest thing on the surface before `OFF`.
pub(super) const EMBER: [Entry; 6] = [WHITE, YELLOW, AMBER, RED, DARK_RED, DARK_GREY];

pub(super) fn hue(i: usize) -> Entry {
    HUES.get(i % HUES.len()).copied().unwrap_or(OFF)
}

/// Draw a heat board through a ramp: full heat is the ramp's first entry,
/// no heat is dark. Shared by [`Rain`] and [`Fire`], which differ in what
/// puts the heat there and in nothing else.
pub(super) fn paint_heat(frame: &mut Frame, heat: &[u8; 64], max: u8, ramp: &[Entry]) {
    let steps = u8::try_from(ramp.len()).unwrap_or(1).max(1);
    for row in 1..=8u8 {
        for col in 1..=8u8 {
            let i = usize::from(row - 1) * 8 + usize::from(col - 1);
            let h = heat.get(i).copied().unwrap_or(0).min(max);
            let cell = if h == 0 {
                Cell::Off
            } else {
                let step = usize::from((max - h) * steps / max.max(1));
                Cell::Static(ramp.get(step).or(ramp.last()).copied().unwrap_or(OFF))
            };
            frame.set(Button::Grid { row, col }, cell);
        }
    }
}
