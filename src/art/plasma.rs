use super::colour::hue;
use super::grid::pads;
use crate::surface::{Button, Cell, Frame};

/// One period of a sine, scaled to ±255. Integers, not floats, so the
/// tests can assert exact frames on any machine.
pub(super) const SINE: [i32; 64] = [
    0, 25, 50, 74, 98, 120, 142, 162, 180, 197, 212, 225, 236, 244, 250, 254, 255, 254, 250, 244,
    236, 225, 212, 197, 180, 162, 142, 120, 98, 74, 50, 25, 0, -25, -50, -74, -98, -120, -142,
    -162, -180, -197, -212, -225, -236, -244, -250, -254, -255, -254, -250, -244, -236, -225, -212,
    -197, -180, -162, -142, -120, -98, -74, -50, -25,
];

/// `sin(i)`, where a full turn is 64 and the result is ±255.
pub(super) fn sin(i: i32) -> i32 {
    let k = usize::try_from(i.rem_euclid(64)).unwrap_or(0);
    SINE.get(k).copied().unwrap_or(0)
}

/// Octagonal distance from the centre of the grid, in half-pads.
///
/// `max + min/2` is within ~4% of the real hypotenuse and costs no square
/// root. On an 8×8 grid the error is smaller than one palette step, which
/// is the only resolution anything here has.
pub(super) fn radius(x: i32, y: i32) -> i32 {
    let (dx, dy) = ((2 * x - 9).abs(), (2 * y - 9).abs());
    dx.max(dy) + dx.min(dy) / 2
}

/// Four sines — across, down, diagonal, radial — summed and mapped onto
/// [`HUES`].
pub(super) fn paint(n: u32, frame: &mut Frame) {
    let time = i32::try_from(n % 10_000).unwrap_or(0);
    for (row, col) in pads() {
        let (x, y) = (i32::from(col), i32::from(row));
        let sum = sin(x * 5 + time)
            + sin(y * 7 - time / 2)
            + sin((x + y) * 4 + time / 3)
            + sin(radius(x, y) * 3 - time);
        // The sum is +/-1020. Shift it positive, then across the ramp.
        let step = usize::try_from((sum + 1020) * 15 / 2041).unwrap_or(0);
        frame.set(Button::Grid { row, col }, Cell::Static(hue(step)));
    }
}
