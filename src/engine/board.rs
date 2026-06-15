//! The 15×15 board and the tiles placed on it.

use serde::{Deserialize, Serialize};

use crate::engine::constants::BOARD_SIZE;

/// A tile locked onto the board. `letter` is always the effective face (for a
/// blank, the letter it was assigned); `is_blank` records that it scores 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacedTile {
    pub letter: char,
    pub is_blank: bool,
}

/// A tile a player is placing this turn (before it is committed to the board).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub row: usize,
    pub col: usize,
    /// The effective face. For a blank, the chosen letter.
    pub letter: char,
    #[serde(default)]
    pub is_blank: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    /// Row-major, `BOARD_SIZE * BOARD_SIZE` cells.
    cells: Vec<Option<PlacedTile>>,
}

impl Default for Board {
    fn default() -> Self {
        Self::empty()
    }
}

impl Board {
    pub fn empty() -> Self {
        Board {
            cells: vec![None; BOARD_SIZE * BOARD_SIZE],
        }
    }

    #[inline]
    fn idx(row: usize, col: usize) -> usize {
        row * BOARD_SIZE + col
    }

    pub fn in_bounds(row: usize, col: usize) -> bool {
        row < BOARD_SIZE && col < BOARD_SIZE
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&PlacedTile> {
        if Self::in_bounds(row, col) {
            self.cells[Self::idx(row, col)].as_ref()
        } else {
            None
        }
    }

    pub fn is_occupied(&self, row: usize, col: usize) -> bool {
        self.get(row, col).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c.is_none())
    }

    pub fn place(&mut self, row: usize, col: usize, tile: PlacedTile) {
        self.cells[Self::idx(row, col)] = Some(tile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_state() {
        let b = Board::empty();
        assert!(b.is_empty());
        assert!(b.get(7, 7).is_none());
        assert!(!b.is_occupied(0, 0));
    }

    #[test]
    fn place_and_read() {
        let mut b = Board::empty();
        b.place(
            7,
            7,
            PlacedTile {
                letter: 'A',
                is_blank: false,
            },
        );
        assert!(!b.is_empty());
        assert_eq!(b.get(7, 7).unwrap().letter, 'A');
        assert!(b.is_occupied(7, 7));
    }

    #[test]
    fn out_of_bounds_is_safe() {
        let b = Board::empty();
        assert!(!Board::in_bounds(15, 0));
        assert!(b.get(99, 99).is_none());
    }
}
