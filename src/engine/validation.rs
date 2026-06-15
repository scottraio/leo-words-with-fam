//! Move validation: geometry (in-line, no gaps, connected, first move covers
//! the center) and word formation (main word + cross words), plus dictionary
//! checks. All functions are pure and operate on a board + a set of pending
//! placements.

use std::collections::{HashMap, HashSet};

use crate::engine::board::{Board, Placement};
use crate::engine::constants::{BOARD_SIZE, CENTER};
use crate::engine::dictionary::Dictionary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn step(self) -> (isize, isize) {
        match self {
            Axis::Horizontal => (0, 1),
            Axis::Vertical => (1, 0),
        }
    }
    fn cross(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoveError {
    #[error("no tiles placed")]
    EmptyPlacement,
    #[error("placement is off the board")]
    OffBoard,
    #[error("two tiles placed on the same square")]
    DuplicateCell,
    #[error("a tile was placed on an occupied square")]
    OverlapsExisting,
    #[error("tiles must be in a single row or column")]
    NotInLine,
    #[error("the placed tiles leave a gap")]
    HasGaps,
    #[error("the first word must cover the center star")]
    FirstMoveMissesCenter,
    #[error("the word must connect to an existing tile")]
    Disconnected,
    #[error("you must form a word of two or more letters")]
    NoWordFormed,
    #[error("'{0}' is not a valid word")]
    InvalidWord(String),
}

/// One cell of a formed word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCell {
    pub row: usize,
    pub col: usize,
    pub letter: char,
    pub is_blank: bool,
    /// True if this tile was placed on the current turn.
    pub is_new: bool,
}

/// A word formed by a move (the main word or a perpendicular cross word).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedWord {
    pub text: String,
    pub cells: Vec<WordCell>,
}

fn placement_map(placements: &[Placement]) -> HashMap<(usize, usize), (char, bool)> {
    placements
        .iter()
        .map(|p| ((p.row, p.col), (p.letter.to_ascii_uppercase(), p.is_blank)))
        .collect()
}

/// The effective tile at a coordinate, considering pending placements overlaid
/// on the board. Returns `(letter, is_blank, is_new)`.
fn tile_at(
    board: &Board,
    map: &HashMap<(usize, usize), (char, bool)>,
    r: isize,
    c: isize,
) -> Option<(char, bool, bool)> {
    if r < 0 || c < 0 || r as usize >= BOARD_SIZE || c as usize >= BOARD_SIZE {
        return None;
    }
    let (ru, cu) = (r as usize, c as usize);
    if let Some(&(letter, is_blank)) = map.get(&(ru, cu)) {
        Some((letter, is_blank, true))
    } else {
        board
            .get(ru, cu)
            .map(|t| (t.letter.to_ascii_uppercase(), t.is_blank, false))
    }
}

/// Validate the placement geometry and return the move's main axis.
pub fn validate_geometry(board: &Board, placements: &[Placement]) -> Result<Axis, MoveError> {
    if placements.is_empty() {
        return Err(MoveError::EmptyPlacement);
    }

    let map = placement_map(placements);
    let mut seen = HashSet::new();
    for p in placements {
        if !Board::in_bounds(p.row, p.col) {
            return Err(MoveError::OffBoard);
        }
        if !seen.insert((p.row, p.col)) {
            return Err(MoveError::DuplicateCell);
        }
        if board.is_occupied(p.row, p.col) {
            return Err(MoveError::OverlapsExisting);
        }
    }

    let rows: HashSet<usize> = placements.iter().map(|p| p.row).collect();
    let cols: HashSet<usize> = placements.iter().map(|p| p.col).collect();

    let axis = if placements.len() == 1 {
        // A single tile: pick the axis along which it extends an existing word.
        let p = &placements[0];
        let (r, c) = (p.row as isize, p.col as isize);
        let horiz = tile_at(board, &map, r, c - 1).is_some()
            || tile_at(board, &map, r, c + 1).is_some();
        if horiz { Axis::Horizontal } else { Axis::Vertical }
    } else if rows.len() == 1 {
        Axis::Horizontal
    } else if cols.len() == 1 {
        Axis::Vertical
    } else {
        return Err(MoveError::NotInLine);
    };

    // No gaps along the main axis between the first and last placed tile.
    let (lo, hi, fixed, horizontal) = match axis {
        Axis::Horizontal => {
            let r = placements[0].row;
            let lo = placements.iter().map(|p| p.col).min().unwrap();
            let hi = placements.iter().map(|p| p.col).max().unwrap();
            (lo, hi, r, true)
        }
        Axis::Vertical => {
            let c = placements[0].col;
            let lo = placements.iter().map(|p| p.row).min().unwrap();
            let hi = placements.iter().map(|p| p.row).max().unwrap();
            (lo, hi, c, false)
        }
    };
    for p in placements {
        let on_line = if horizontal { p.row == fixed } else { p.col == fixed };
        if !on_line {
            return Err(MoveError::NotInLine);
        }
    }
    for i in lo..=hi {
        let (r, c) = if horizontal {
            (fixed as isize, i as isize)
        } else {
            (i as isize, fixed as isize)
        };
        if tile_at(board, &map, r, c).is_none() {
            return Err(MoveError::HasGaps);
        }
    }

    // First move covers the center; later moves connect to an existing tile.
    if board.is_empty() {
        if !placements.iter().any(|p| (p.row, p.col) == CENTER) {
            return Err(MoveError::FirstMoveMissesCenter);
        }
    } else {
        let touches = placements.iter().any(|p| {
            let (r, c) = (p.row as isize, p.col as isize);
            [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().any(|(dr, dc)| {
                let (nr, nc) = (r + dr, c + dc);
                nr >= 0
                    && nc >= 0
                    && (nr as usize) < BOARD_SIZE
                    && (nc as usize) < BOARD_SIZE
                    && board.is_occupied(nr as usize, nc as usize)
            })
        });
        if !touches {
            return Err(MoveError::Disconnected);
        }
    }

    Ok(axis)
}

fn build_word(
    board: &Board,
    map: &HashMap<(usize, usize), (char, bool)>,
    from: (usize, usize),
    axis: Axis,
) -> FormedWord {
    let (dr, dc) = axis.step();
    // Walk back to the start of the contiguous run.
    let (mut sr, mut sc) = (from.0 as isize, from.1 as isize);
    while tile_at(board, map, sr - dr, sc - dc).is_some() {
        sr -= dr;
        sc -= dc;
    }
    // Walk forward, collecting cells.
    let mut cells = Vec::new();
    let (mut cr, mut cc) = (sr, sc);
    while let Some((letter, is_blank, is_new)) = tile_at(board, map, cr, cc) {
        cells.push(WordCell {
            row: cr as usize,
            col: cc as usize,
            letter,
            is_blank,
            is_new,
        });
        cr += dr;
        cc += dc;
    }
    let text = cells.iter().map(|c| c.letter).collect();
    FormedWord { text, cells }
}

/// Collect the main word plus every cross word formed by the placements.
/// Only words of length ≥ 2 are returned.
pub fn collect_formed_words(
    board: &Board,
    placements: &[Placement],
    axis: Axis,
) -> Vec<FormedWord> {
    let map = placement_map(placements);
    let mut words = Vec::new();

    let main = build_word(board, &map, (placements[0].row, placements[0].col), axis);
    if main.cells.len() >= 2 {
        words.push(main);
    }
    for p in placements {
        let cross = build_word(board, &map, (p.row, p.col), axis.cross());
        if cross.cells.len() >= 2 {
            words.push(cross);
        }
    }
    words
}

/// Full validation: geometry, word formation, and dictionary lookup.
/// Returns the formed words (with cells) for scoring.
pub fn validate_play(
    board: &Board,
    placements: &[Placement],
    dict: &Dictionary,
) -> Result<Vec<FormedWord>, MoveError> {
    let axis = validate_geometry(board, placements)?;
    let words = collect_formed_words(board, placements, axis);
    if words.is_empty() {
        return Err(MoveError::NoWordFormed);
    }
    for w in &words {
        if !dict.contains(&w.text) {
            return Err(MoveError::InvalidWord(w.text.clone()));
        }
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::board::PlacedTile;

    fn p(row: usize, col: usize, letter: char) -> Placement {
        Placement {
            row,
            col,
            letter,
            is_blank: false,
        }
    }

    fn dict() -> Dictionary {
        Dictionary::from_words(["CAT", "CATS", "AT", "AS", "TO", "CATSUP"])
    }

    #[test]
    fn first_move_must_cover_center() {
        let board = Board::empty();
        let placements = vec![p(0, 0, 'C'), p(0, 1, 'A'), p(0, 2, 'T')];
        assert_eq!(
            validate_geometry(&board, &placements),
            Err(MoveError::FirstMoveMissesCenter)
        );
    }

    #[test]
    fn first_move_on_center_is_valid() {
        let board = Board::empty();
        let placements = vec![p(7, 6, 'C'), p(7, 7, 'A'), p(7, 8, 'T')];
        let words = validate_play(&board, &placements, &dict()).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "CAT");
    }

    #[test]
    fn gaps_rejected() {
        let board = Board::empty();
        let placements = vec![p(7, 6, 'C'), p(7, 8, 'T')];
        assert_eq!(
            validate_geometry(&board, &placements),
            Err(MoveError::HasGaps)
        );
    }

    #[test]
    fn not_in_line_rejected() {
        let board = Board::empty();
        let placements = vec![p(7, 7, 'C'), p(8, 8, 'A')];
        assert_eq!(
            validate_geometry(&board, &placements),
            Err(MoveError::NotInLine)
        );
    }

    #[test]
    fn second_move_must_connect() {
        let mut board = Board::empty();
        for (c, ch) in [(6, 'C'), (7, 'A'), (8, 'T')] {
            board.place(
                7,
                c,
                PlacedTile {
                    letter: ch,
                    is_blank: false,
                },
            );
        }
        // Disconnected play elsewhere.
        let placements = vec![p(0, 0, 'A'), p(0, 1, 'T')];
        assert_eq!(
            validate_geometry(&board, &placements),
            Err(MoveError::Disconnected)
        );
    }

    #[test]
    fn cross_word_collected_and_validated() {
        // Board has CAT horizontally at row 7, cols 6..8.
        let mut board = Board::empty();
        for (c, ch) in [(6, 'C'), (7, 'A'), (8, 'T')] {
            board.place(
                7,
                c,
                PlacedTile {
                    letter: ch,
                    is_blank: false,
                },
            );
        }
        // Play S below T (8,8) making TO? No — make "AS" downward from A(7,7): place S at (8,7).
        let placements = vec![p(8, 7, 'S')];
        let words = validate_play(&board, &placements, &dict()).unwrap();
        // Forms "AS" vertically.
        assert!(words.iter().any(|w| w.text == "AS"));
    }

    #[test]
    fn invalid_word_rejected() {
        let board = Board::empty();
        let placements = vec![p(7, 7, 'Z'), p(7, 8, 'Z')];
        assert_eq!(
            validate_play(&board, &placements, &dict()),
            Err(MoveError::InvalidWord("ZZ".to_string()))
        );
    }

    #[test]
    fn extending_existing_word() {
        // CAT at row 7 cols 6-8; play S at (7,9) to make CATS.
        let mut board = Board::empty();
        for (c, ch) in [(6, 'C'), (7, 'A'), (8, 'T')] {
            board.place(
                7,
                c,
                PlacedTile {
                    letter: ch,
                    is_blank: false,
                },
            );
        }
        let placements = vec![p(7, 9, 'S')];
        let words = validate_play(&board, &placements, &dict()).unwrap();
        assert!(words.iter().any(|w| w.text == "CATS"));
    }
}
