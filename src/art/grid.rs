//! The walk across the sixty-four pads, written once.
//!
//! The dimensions come from [`crate::slot::WINDOW`] and
//! [`crate::slot::COLS`], which are the *hardware* grid — the pads the
//! Launchpad has — and not the address space. An art paints what is on the
//! desk; it has never heard of a page.

use crate::slot::{COLS, WINDOW};

/// Every pad, top-left to bottom-right, as 1-based `(row, col)`.
pub(super) fn pads() -> impl Iterator<Item = (u8, u8)> {
    (1..=WINDOW).flat_map(|row| (1..=COLS).map(move |col| (row, col)))
}

/// A pad's index into a 64-cell board: 1-based in, 0-based out.
///
/// The boards are flat arrays rather than `[[_; 8]; 8]` because every
/// simulation here reads its neighbours by offset, and `i - 8` is the
/// whole of "the cell above".
pub(super) fn linear(row: u8, col: u8) -> usize {
    usize::from(row - 1) * usize::from(COLS) + usize::from(col - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The walk covers the grid exactly once, in reading order, and
    /// [`linear`] numbers it 0..64 in that same order. Both halves matter:
    /// a board is indexed by one and painted by the other.
    #[test]
    fn the_walk_and_the_index_agree() {
        let walked: Vec<(u8, u8)> = pads().collect();
        assert_eq!(walked.len(), 64);
        assert_eq!(walked.first(), Some(&(1, 1)));
        assert_eq!(walked.last(), Some(&(8, 8)));
        for (i, &(row, col)) in walked.iter().enumerate() {
            assert_eq!(linear(row, col), i, "({row},{col})");
        }
    }
}
