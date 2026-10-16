//! Addressing: where a button is, in the two coordinate systems that exist.
//!
//! The grid is written top-down everywhere a human reads it — in
//! `tl.slot` labels, in `slots.toml`, in this crate's types — and the
//! hardware numbers its rows from the bottom. **The flip happens here and
//! nowhere else** (`docs/02-surface.md`, `docs/03-model.md`).

/// A physical button on the surface.
///
/// Rows and columns are 1-based and **row 1 is the top row**, which is the
/// opposite of the wire encoding. `col` 1 is the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    /// One of the 64 slots.
    Grid {
        /// 1-8, **1 is the top row**.
        row: u8,
        /// 1-8, 1 is the left column.
        col: u8,
    },
    /// Top row, left to right: the page selectors.
    Page(u8),
    /// Right column, top to bottom: the action modifiers.
    Action(u8),
    /// The logo. Not a slot — it is the health light.
    Logo,
}

/// Every addressable position, including the logo. 81 of them.
pub const SURFACE_LEN: usize = 81;

impl Button {
    /// The byte the device uses for this button.
    ///
    /// `index = hw_row * 10 + col`, where `hw_row` counts from the bottom.
    /// A logical row 1 (top) is hardware row 8.
    pub fn index(self) -> u8 {
        match self {
            Self::Grid { row, col } => (9 - row) * 10 + col,
            Self::Page(col) => 90 + col,
            Self::Action(row) => (9 - row) * 10 + 9,
            Self::Logo => 99,
        }
    }

    /// The button a press on `index` came from, or `None` if the device
    /// sent something outside the surface.
    pub fn from_index(index: u8) -> Option<Self> {
        let (hw_row, col) = (index / 10, index % 10);
        match (hw_row, col) {
            (9, 9) => Some(Self::Logo),
            (9, 1..=8) => Some(Self::Page(col)),
            (1..=8, 9) => Some(Self::Action(9 - hw_row)),
            (1..=8, 1..=8) => Some(Self::Grid {
                row: 9 - hw_row,
                col,
            }),
            _ => None,
        }
    }

    /// Position in a frame's cell array. Dense, `0..SURFACE_LEN`.
    pub(crate) fn slot(self) -> usize {
        let i = self.index();
        (i / 10 - 1) as usize * 9 + (i % 10 - 1) as usize
    }

    /// Inverse of [`Button::slot`].
    pub(crate) fn from_slot(slot: usize) -> Self {
        let row = u8::try_from(slot / 9 + 1).unwrap_or(0);
        let col = u8::try_from(slot % 9 + 1).unwrap_or(0);
        Self::from_index(row * 10 + col).expect("slot is in range by construction")
    }

    /// Every button on the surface, in frame order.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..SURFACE_LEN).map(Self::from_slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flip is the one piece of arithmetic that is easy to get wrong in
    /// a way that still looks plausible, so all four corners are asserted
    /// rather than one (`docs/09-testing.md`).
    #[test]
    fn logical_row_one_is_the_bottom_on_the_wire() {
        // Confirmed on hardware in M0: 0x0B lit the bottom-left pad.
        assert_eq!(Button::Grid { row: 8, col: 1 }.index(), 0x0B); // bottom-left
        assert_eq!(Button::Grid { row: 8, col: 8 }.index(), 18); // bottom-right
        assert_eq!(Button::Grid { row: 1, col: 1 }.index(), 81); // top-left
        assert_eq!(Button::Grid { row: 1, col: 8 }.index(), 88); // top-right
    }

    /// Decimal row-column written in hex: pad (8,1) is 0x0B, not 0x11.
    #[test]
    fn index_is_decimal_not_hex() {
        assert_eq!(Button::Grid { row: 8, col: 1 }.index(), 11);
        assert_ne!(Button::Grid { row: 8, col: 1 }.index(), 0x11);
    }

    #[test]
    fn controls_and_logo() {
        assert_eq!(Button::Page(1).index(), 91);
        assert_eq!(Button::Page(8).index(), 98);
        assert_eq!(Button::Logo.index(), 99);
        // CC89 is the top of the right column; CC19 the bottom. docs/05
        // binds kill to CC19, so getting this backwards is a live hazard.
        assert_eq!(Button::Action(1).index(), 89);
        assert_eq!(Button::Action(8).index(), 19);
    }

    #[test]
    fn index_round_trips_for_every_button() {
        for b in Button::all() {
            assert_eq!(Button::from_index(b.index()), Some(b), "{b:?}");
            assert_eq!(Button::from_slot(b.slot()), b, "{b:?}");
        }
        assert_eq!(Button::all().count(), SURFACE_LEN);
    }

    #[test]
    fn off_surface_indices_are_rejected() {
        for i in [0, 10, 20, 90, 100, 255, 9, 19 + 100] {
            if let Some(b) = Button::from_index(i) {
                assert_eq!(b.index(), i, "index {i} decoded to a different button");
            }
        }
        assert_eq!(Button::from_index(0), None); // col 0
        assert_eq!(Button::from_index(10), None); // col 0
        assert_eq!(Button::from_index(100), None); // row 10
    }
}
