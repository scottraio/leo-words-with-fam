//! Words With Friends board, tile, and scoring constants.
//!
//! The premium-square layout is the standard Words With Friends 15×15 board,
//! which is fully symmetric under the dihedral group of the square (both axes
//! and both diagonals). The center (7,7) is the start square and carries *no*
//! multiplier — only a placement constraint (the first word must cover it).
//!
//! The exact letter distribution and premium layout are locked by the tests in
//! this module (bag total = 104 incl. 2 blanks; premium counts; full symmetry).

use serde::{Deserialize, Serialize};

pub const BOARD_SIZE: usize = 15;
pub const RACK_SIZE: usize = 7;
pub const CENTER: (usize, usize) = (7, 7);
/// Total tiles in a full bag (102 letter tiles + 2 blanks).
pub const BAG_SIZE: usize = 104;
/// Bonus for playing all 7 tiles in one move. WWF awards 35 (Scrabble is 50).
pub const BINGO_BONUS: i32 = 35;
/// The blank tile is represented by this character on the bag/rack.
pub const BLANK: char = '_';

/// Premium square multiplier classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Premium {
    None,
    /// Double letter score.
    DL,
    /// Triple letter score.
    TL,
    /// Double word score.
    DW,
    /// Triple word score.
    TW,
}

/// Point value of a letter tile (A–Z). The blank is worth 0 and is handled by
/// the caller (a placed blank always scores 0 regardless of its assigned face).
pub fn letter_points(c: char) -> i32 {
    match c.to_ascii_uppercase() {
        'A' | 'E' | 'I' | 'O' | 'R' | 'S' | 'T' => 1,
        'D' | 'L' | 'N' | 'U' => 2,
        'G' | 'H' => 3,
        'B' | 'C' | 'F' | 'M' | 'P' | 'W' | 'Y' => 4,
        'K' | 'V' => 5,
        'X' => 8,
        'J' | 'Q' | 'Z' => 10,
        _ => 0,
    }
}

/// Number of copies of a letter in a full bag (A–Z). Blanks (2) are added
/// separately by [`crate::engine::bag::TileBag::full`].
pub fn letter_count(c: char) -> u8 {
    match c.to_ascii_uppercase() {
        'E' => 13,
        'A' => 9,
        'I' | 'O' => 8,
        'T' => 7,
        'R' => 6,
        'D' | 'N' | 'S' => 5,
        'H' | 'L' | 'U' => 4,
        'G' => 3,
        'B' | 'C' | 'F' | 'M' | 'P' | 'V' | 'W' | 'Y' => 2,
        'J' | 'K' | 'Q' | 'X' | 'Z' => 1,
        _ => 0,
    }
}

const TW: &[(usize, usize)] = &[
    (0, 3),
    (0, 11),
    (3, 0),
    (3, 14),
    (11, 0),
    (11, 14),
    (14, 3),
    (14, 11),
];

const TL: &[(usize, usize)] = &[
    (0, 6),
    (0, 8),
    (2, 7),
    (5, 5),
    (5, 9),
    (6, 0),
    (6, 14),
    (7, 2),
    (7, 12),
    (8, 0),
    (8, 14),
    (9, 5),
    (9, 9),
    (12, 7),
    (14, 6),
    (14, 8),
];

const DW: &[(usize, usize)] = &[
    (4, 4),
    (4, 10),
    (6, 6),
    (6, 8),
    (8, 6),
    (8, 8),
    (10, 4),
    (10, 10),
];

const DL: &[(usize, usize)] = &[
    (1, 5),
    (1, 9),
    (2, 2),
    (2, 12),
    (3, 3),
    (3, 11),
    (5, 1),
    (5, 13),
    (9, 1),
    (9, 13),
    (11, 3),
    (11, 11),
    (12, 2),
    (12, 12),
    (13, 5),
    (13, 9),
];

/// Build the full 15×15 premium grid.
pub fn premium_grid() -> [[Premium; BOARD_SIZE]; BOARD_SIZE] {
    let mut grid = [[Premium::None; BOARD_SIZE]; BOARD_SIZE];
    for &(r, c) in TW {
        grid[r][c] = Premium::TW;
    }
    for &(r, c) in TL {
        grid[r][c] = Premium::TL;
    }
    for &(r, c) in DW {
        grid[r][c] = Premium::DW;
    }
    for &(r, c) in DL {
        grid[r][c] = Premium::DL;
    }
    grid
}

/// The premium class at a board coordinate.
pub fn premium_at(r: usize, c: usize) -> Premium {
    premium_grid()[r][c]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bag_has_104_tiles_including_two_blanks() {
        let letters: u32 = (b'A'..=b'Z').map(|b| letter_count(b as char) as u32).sum();
        assert_eq!(letters, 102, "letter tiles");
        assert_eq!(letters as usize + 2, BAG_SIZE, "with 2 blanks");
    }

    #[test]
    fn premium_counts_match_wwf() {
        let grid = premium_grid();
        let mut tw = 0;
        let mut tl = 0;
        let mut dw = 0;
        let mut dl = 0;
        for row in &grid {
            for cell in row {
                match cell {
                    Premium::TW => tw += 1,
                    Premium::TL => tl += 1,
                    Premium::DW => dw += 1,
                    Premium::DL => dl += 1,
                    Premium::None => {}
                }
            }
        }
        assert_eq!((tw, tl, dw, dl), (8, 16, 8, 16));
    }

    #[test]
    fn premium_grid_is_fully_symmetric() {
        // The WWF board has full dihedral symmetry: reflecting across the
        // horizontal axis, vertical axis, and main diagonal all map premium
        // squares onto premium squares of the same class.
        let g = premium_grid();
        let n = BOARD_SIZE - 1;
        for r in 0..BOARD_SIZE {
            for c in 0..BOARD_SIZE {
                assert_eq!(g[r][c], g[n - r][c], "horizontal mirror at {r},{c}");
                assert_eq!(g[r][c], g[r][n - c], "vertical mirror at {r},{c}");
                assert_eq!(g[r][c], g[c][r], "diagonal mirror at {r},{c}");
            }
        }
    }

    #[test]
    fn center_has_no_multiplier() {
        assert_eq!(premium_at(CENTER.0, CENTER.1), Premium::None);
    }

    #[test]
    fn high_value_letters() {
        assert_eq!(letter_points('Q'), 10);
        assert_eq!(letter_points('z'), 10);
        assert_eq!(letter_points('X'), 8);
        assert_eq!(letter_points('A'), 1);
        assert_eq!(letter_points('_'), 0);
    }
}
