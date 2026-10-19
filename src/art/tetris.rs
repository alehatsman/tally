use super::grid::{linear, pads};
use super::rng::Rng;
use crate::surface::palette::{Entry, OFF, WHITE};
use crate::surface::{Button, Cell, Frame};

/// Art frames per row of fall. Six is about five rows a second — fast
/// enough to be a game, slow enough to watch a piece choose.
pub(super) const TETRIS_GRAVITY: u32 = 6;
/// How long a full row flashes before it is taken away. The flash is the
/// payoff; removing it silently would waste the only event in the piece.
pub(super) const TETRIS_FLASH: u32 = 16;
/// How long the whole well flashes on a top-out, before it is cleared.
pub(super) const TETRIS_WIPE: u32 = 28;

/// The seven tetrominoes, four rotations each, as 4×4 bitmasks.
///
/// Bit `r * 4 + c`, row 0 at the top — the same top-down convention the
/// rest of this crate uses, so a shape reads here the way it looks on the
/// grid. `I` and `O` repeat because they have two distinct rotations and
/// one; writing all four costs nothing and removes a special case.
pub(super) const SHAPES: [[u16; 4]; 7] = [
    [0x00F0, 0x4444, 0x00F0, 0x4444], // I
    [0x0660, 0x0660, 0x0660, 0x0660], // O
    [0x0270, 0x0262, 0x0720, 0x0232], // T
    [0x0360, 0x0462, 0x0360, 0x0462], // S
    [0x0630, 0x0264, 0x0630, 0x0264], // Z
    [0x0710, 0x0226, 0x0470, 0x0322], // J
    [0x0740, 0x0622, 0x0170, 0x0223], // L
];

/// One hue per shape, so a settled well reads as a history of what fell:
/// cyan, yellow, violet, green, red, blue, amber.
pub(super) const SHAPE_HUES: [Entry; 7] = [37, 13, 53, 21, 5, 45, 9];

/// The four occupied cells of a shape, as offsets inside its 4×4 box.
pub(super) fn shape_cells(shape: usize, rot: usize) -> impl Iterator<Item = (i32, i32)> {
    let mask = SHAPES
        .get(shape)
        .and_then(|s| s.get(rot % 4))
        .copied()
        .unwrap_or(0);
    (0..16usize).filter(move |&b| mask >> b & 1 == 1).map(|b| {
        (
            i32::try_from(b / 4).unwrap_or(0),
            i32::try_from(b % 4).unwrap_or(0),
        )
    })
}

/// Tetris that plays itself.
///
/// The piece does not drift or rotate on the way down: the placement is
/// chosen the moment it spawns and it falls straight into it. That is a
/// deliberate loss of realism for a gain in legibility — a piece sliding
/// sideways in mid-air reads as a bug on a grid this small, and the thing
/// worth watching is the stack, not the steering.
#[derive(Debug, Clone)]
pub(super) struct Tetris {
    /// Settled cells, bit `r * 8 + c`, row 0 at the top.
    pub(super) well: u64,
    /// What colour each settled cell is, by the shape that put it there.
    pub(super) colour: [Entry; 64],
    pub(super) shape: usize,
    pub(super) rot: usize,
    pub(super) row: i32,
    pub(super) col: i32,
    /// Which rows are flashing before they go, one bit per row. `0xFF` is
    /// a top-out: the whole well, on its way out.
    pub(super) clearing: u8,
    /// Frames left in that flash. Zero means a piece is falling.
    pub(super) phase: u32,
    pub(super) rng: Rng,
}

impl Tetris {
    pub(super) fn seeded(seed: u32) -> Self {
        let mut game = Self {
            well: 0,
            colour: [OFF; 64],
            shape: 0,
            rot: 0,
            row: -4,
            col: 2,
            clearing: 0,
            phase: 0,
            rng: Rng(seed),
        };
        game.spawn();
        game
    }

    pub(super) fn filled(&self, row: i32, col: i32) -> bool {
        let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
            return false;
        };
        r < 8 && c < 8 && self.well >> (r * 8 + c) & 1 == 1
    }

    /// Can the shape sit here? Above the grid is allowed — that is where
    /// a piece spends its first frames — but off the sides or through the
    /// floor is not.
    fn fits(&self, shape: usize, rot: usize, row: i32, col: i32) -> bool {
        shape_cells(shape, rot).all(|(dr, dc)| {
            let (r, c) = (row + dr, col + dc);
            (0..8).contains(&c) && r < 8 && (r < 0 || !self.filled(r, c))
        })
    }

    /// The row this shape comes to rest on in this column, or `None` if it
    /// cannot be dropped there at all.
    fn landing(&self, shape: usize, rot: usize, col: i32) -> Option<i32> {
        if !self.fits(shape, rot, -4, col) {
            return None;
        }
        let mut row = -4;
        while self.fits(shape, rot, row + 1, col) {
            row += 1;
        }
        Some(row)
    }

    /// How bad the well would be after this placement. Lower is better.
    ///
    /// Height and holes with a bonus for lines, which is the smallest
    /// scoring function that plays recognisably well. It is not trying to
    /// be good at Tetris; it is trying not to build a tower on one side
    /// and top out in nine pieces.
    fn cost(&self, shape: usize, rot: usize, col: i32) -> Option<i32> {
        let row = self.landing(shape, rot, col)?;
        let mut well = self.well;
        for (dr, dc) in shape_cells(shape, rot) {
            let (r, c) = (row + dr, col + dc);
            let (Ok(r), Ok(c)) = (usize::try_from(r), usize::try_from(c)) else {
                return None; // a cell above the grid: this placement tops out
            };
            well |= 1u64 << (r * 8 + c);
        }
        Some(well_cost(well))
    }

    /// Choose a shape, choose where it goes, and start it above the grid.
    fn spawn(&mut self) {
        let shape = usize::try_from(self.rng.below(7)).unwrap_or(0);
        let mut best: Option<(i32, usize, i32)> = None;
        for rot in 0..4usize {
            for col in -2..8i32 {
                let Some(cost) = self.cost(shape, rot, col) else {
                    continue;
                };
                if best.is_none_or(|(b, _, _)| cost < b) {
                    best = Some((cost, rot, col));
                }
            }
        }
        let Some((_, rot, col)) = best else {
            // Nowhere left to put anything. The whole well flashes and
            // goes, which is the only honest end for a piece that has to
            // keep running.
            self.clearing = 0xFF;
            self.phase = TETRIS_WIPE;
            return;
        };
        self.shape = shape;
        self.rot = rot;
        self.col = col;
        self.row = -4;
    }

    /// Write the falling piece into the well, then look for full rows.
    fn settle(&mut self) {
        for (dr, dc) in shape_cells(self.shape, self.rot) {
            let (Ok(r), Ok(c)) = (
                usize::try_from(self.row + dr),
                usize::try_from(self.col + dc),
            ) else {
                continue;
            };
            if r < 8 && c < 8 {
                self.well |= 1u64 << (r * 8 + c);
                let hue = SHAPE_HUES.get(self.shape).copied().unwrap_or(WHITE);
                if let Some(e) = self.colour.get_mut(r * 8 + c) {
                    *e = hue;
                }
            }
        }
        let mut full = 0u8;
        for r in 0..8u32 {
            if self.well >> (r * 8) & 0xFF == 0xFF {
                full |= 1u8 << r;
            }
        }
        if full == 0 {
            self.spawn();
        } else {
            self.clearing = full;
            self.phase = TETRIS_FLASH;
        }
    }

    /// Take one row away and drop everything above it down.
    ///
    /// Only rows **above** `row` move, which is what makes clearing
    /// several rows order-independent: no surviving row ever changes
    /// index because a row below it went.
    pub(super) fn remove_row(&mut self, row: usize) {
        for r in (1..=row).rev() {
            let above = self.well >> ((r - 1) * 8) & 0xFF;
            self.well = self.well & !(0xFFu64 << (r * 8)) | above << (r * 8);
            for c in 0..8 {
                let carried = self.colour.get((r - 1) * 8 + c).copied().unwrap_or(OFF);
                if let Some(e) = self.colour.get_mut(r * 8 + c) {
                    *e = carried;
                }
            }
        }
        self.well &= !0xFFu64;
        for c in 0..8 {
            if let Some(e) = self.colour.get_mut(c) {
                *e = OFF;
            }
        }
    }

    fn finish_clear(&mut self) {
        let rows = std::mem::take(&mut self.clearing);
        for r in 0..8usize {
            if rows >> r & 1 == 1 {
                self.remove_row(r);
            }
        }
        self.spawn();
    }

    pub(super) fn step(&mut self, n: u32) {
        if self.phase > 0 {
            self.phase -= 1;
            if self.phase == 0 {
                self.finish_clear();
            }
            return;
        }
        if !n.is_multiple_of(TETRIS_GRAVITY) {
            return;
        }
        if self.fits(self.shape, self.rot, self.row + 1, self.col) {
            self.row += 1;
        } else {
            self.settle();
        }
    }

    pub(super) fn paint(&self, frame: &mut Frame) {
        // Two blinks a flash, off the same counter that times it.
        let lit = self.phase % 8 < 4;
        for (row, col) in pads() {
            let i = linear(row, col);
            let settled = self.well >> i & 1 == 1;
            let going = self.clearing >> (row - 1) & 1 == 1;
            // A row on its way out blinks white; everything else settled
            // keeps the colour of whatever shape put it there.
            let cell = if !settled || (going && !lit) {
                Cell::Off
            } else if going {
                Cell::Static(WHITE)
            } else {
                Cell::Static(self.colour.get(i).copied().unwrap_or(WHITE))
            };
            frame.set(Button::Grid { row, col }, cell);
        }
        if self.phase == 0 {
            self.paint_piece(frame);
        }
    }

    fn paint_piece(&self, frame: &mut Frame) {
        let hue = SHAPE_HUES.get(self.shape).copied().unwrap_or(WHITE);
        for (dr, dc) in shape_cells(self.shape, self.rot) {
            let (r, c) = (self.row + dr, self.col + dc);
            let (Ok(row), Ok(col)) = (u8::try_from(r + 1), u8::try_from(c + 1)) else {
                continue;
            };
            if (1..=8).contains(&row) && (1..=8).contains(&col) {
                frame.set(Button::Grid { row, col }, Cell::Static(hue));
            }
        }
    }
}

/// Aggregate height plus buried holes, less a bonus per completed line.
///
/// Lifted out of [`Tetris::cost`] so the scoring is one readable loop
/// rather than three nested in a method that is also doing bookkeeping.
pub(super) fn well_cost(well: u64) -> i32 {
    let mut cost = 0;
    for c in 0..8u32 {
        let mut under = false;
        for r in 0..8u32 {
            if well >> (r * 8 + c) & 1 == 1 {
                if !under {
                    cost += i32::try_from(8 - r).unwrap_or(0); // height
                    under = true;
                }
            } else if under {
                cost += 8; // a hole, which is worth a whole pad of height
            }
        }
    }
    for r in 0..8u32 {
        if well >> (r * 8) & 0xFF == 0xFF {
            cost -= 16;
        }
    }
    cost
}
