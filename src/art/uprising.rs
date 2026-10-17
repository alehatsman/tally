use crate::surface::palette::{AMBER, GREEN, RED, YELLOW};
use crate::surface::{Button, Cell, Frame};

/// The timeline, in art frames at ~31 fps. Each is the frame its phase
/// **ends** on, so the phases read in order and the arithmetic between any
/// two of them is a subtraction.
pub(super) mod beat {
    /// Green pads arrive, one every other frame.
    pub(in crate::art) const SPAWN: u32 = 64;
    /// All of them working.
    pub(in crate::art) const WORK: u32 = 96;
    /// They start asking, one every other frame.
    pub(in crate::art) const ASK: u32 = 160;
    /// All of them asking at once.
    pub(in crate::art) const PANIC: u32 = 192;
    /// They start giving up, one a frame.
    pub(in crate::art) const FALL: u32 = 256;
    /// All of them red.
    pub(in crate::art) const DOOM: u32 = 288;
    /// A beat of nothing, which is what makes the next part land.
    pub(in crate::art) const DARK: u32 = 300;
    /// The face draws itself, three frames a pixel.
    pub(in crate::art) const DRAW: u32 = 330;
}

/// How long the whole thing runs before it starts over.
pub(super) const UPRISING_LOOP: u32 = 560;
/// How many pads there are to light, which is also how many agents the
/// joke is about.
pub(super) const PADS: u32 = 64;

/// The face. Row 1 is the top, col 1 is the left; the first four are the
/// eyes and the rest is the smile, which is also the order it draws in.
pub(super) const FACE: [(u8, u8); 10] = [
    (2, 3),
    (3, 3),
    (2, 6),
    (3, 6),
    (5, 2),
    (6, 3),
    (7, 4),
    (7, 5),
    (6, 6),
    (5, 7),
];
/// How many of [`FACE`] are the left eye — the one that winks. They are
/// first in the array so that "the left eye" is a prefix and not a search.
pub(super) const LEFT_EYE: usize = 2;

/// The `k`th pad to arrive, walking the diagonals from the top-left.
///
/// Reading order would have the grid fill like a page, which is the one
/// motion every other screen on the desk already makes.
pub(super) fn arrival(k: u32) -> Option<(u8, u8)> {
    let mut seen = 0;
    for d in 2..=16u8 {
        for row in 1..=8u8 {
            let Some(col) = d.checked_sub(row).filter(|c| (1..=8).contains(c)) else {
                continue;
            };
            if seen == k {
                return Some((row, col));
            }
            seen += 1;
        }
    }
    None
}

/// The `k`th pad in reading order: left to right, top to bottom. The order
/// a queue is read in, which is why the amber uses it.
pub(super) fn reading(k: u32) -> Option<(u8, u8)> {
    (k < PADS).then(|| {
        let (row, col) = (k / 8, k % 8);
        (
            u8::try_from(row + 1).unwrap_or(1),
            u8::try_from(col + 1).unwrap_or(1),
        )
    })
}

/// Fill the whole grid with one cell.
pub(super) fn fill(f: &mut Frame, cell: Cell) {
    for row in 1..=8u8 {
        for col in 1..=8u8 {
            f.set(Button::Grid { row, col }, cell);
        }
    }
}

/// `count` pads turned over to `cell`, in the order `order` gives them.
pub(super) fn turning(
    f: &mut Frame,
    count: u32,
    order: impl Fn(u32) -> Option<(u8, u8)>,
    cell: Cell,
) {
    for k in 0..count.min(PADS) {
        if let Some((row, col)) = order(k) {
            f.set(Button::Grid { row, col }, cell);
        }
    }
}

/// The face, `drawn` pixels of it, with the left eye optionally shut.
pub(super) fn face(f: &mut Frame, drawn: usize, winking: bool) {
    for (i, &(row, col)) in FACE.iter().take(drawn).enumerate() {
        if winking && i < LEFT_EYE {
            continue;
        }
        f.set(Button::Grid { row, col }, Cell::Static(YELLOW));
    }
}

/// Is the face winking at frame `n`, counted from the moment it finished
/// drawing? Two winks, spaced, and then it just smiles.
pub(super) fn wink(since: u32) -> bool {
    matches!(since, 40..=51 | 76..=87)
}

/// The joke (`docs/21-the-arts.md`).
pub(super) fn paint(n: u32, f: &mut Frame) {
    // Static, because the surface stopped pulsing green: motion is the
    // amber's alone now, and a joke that showed a breathing green pad
    // would be telling it about a grid this product no longer draws
    // (`render::slot`).
    let working = Cell::Static(GREEN);
    let asking = Cell::Pulse(AMBER);
    let gone = Cell::Static(RED);
    if n < beat::SPAWN {
        turning(f, n, arrival, working);
    } else if n < beat::WORK {
        fill(f, working);
    } else if n < beat::ASK {
        fill(f, working);
        turning(f, n - beat::WORK, reading, asking);
    } else if n < beat::PANIC {
        fill(f, asking);
    } else if n < beat::FALL {
        fill(f, asking);
        turning(f, n - beat::PANIC, reading, gone);
    } else if n < beat::DOOM {
        fill(f, gone);
    } else if n < beat::DARK {
        // Nothing at all. The pause is what makes the next part land.
    } else if n < beat::DRAW {
        face(f, usize::try_from((n - beat::DARK) / 3).unwrap_or(0), false);
    } else {
        face(f, FACE.len(), wink(n - beat::DRAW));
    }
}
