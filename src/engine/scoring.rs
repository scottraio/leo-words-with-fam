//! Move scoring. Premium squares apply only to newly placed tiles; word
//! multipliers (DW/TW) stack across a word; blank tiles always score 0; using
//! all seven tiles in one move earns [`BINGO_BONUS`].

use serde::{Deserialize, Serialize};

use crate::engine::constants::{BINGO_BONUS, Premium, RACK_SIZE, letter_points, premium_at};
use crate::engine::validation::FormedWord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordScore {
    pub text: String,
    pub points: i32,
}

/// Score a single formed word, honoring premium squares under newly placed
/// tiles only.
pub fn score_word(word: &FormedWord) -> i32 {
    let mut letter_sum = 0;
    let mut word_mult = 1;
    for cell in &word.cells {
        let base = if cell.is_blank {
            0
        } else {
            letter_points(cell.letter)
        };
        let (letter_mult, this_word_mult) = if cell.is_new {
            match premium_at(cell.row, cell.col) {
                Premium::DL => (2, 1),
                Premium::TL => (3, 1),
                Premium::DW => (1, 2),
                Premium::TW => (1, 3),
                Premium::None => (1, 1),
            }
        } else {
            (1, 1)
        };
        letter_sum += base * letter_mult;
        word_mult *= this_word_mult;
    }
    letter_sum * word_mult
}

/// Score a whole move: the sum of all formed words, plus the bingo bonus if all
/// seven tiles were used. Returns the total and the per-word breakdown.
pub fn score_move(words: &[FormedWord], tiles_placed: usize) -> (i32, Vec<WordScore>) {
    let mut total = 0;
    let mut breakdown = Vec::with_capacity(words.len());
    for w in words {
        let pts = score_word(w);
        total += pts;
        breakdown.push(WordScore {
            text: w.text.clone(),
            points: pts,
        });
    }
    if tiles_placed == RACK_SIZE {
        total += BINGO_BONUS;
    }
    (total, breakdown)
}

/// Point value of the tiles left on a rack (blanks count 0).
pub fn rack_value(tiles: &[char]) -> i32 {
    tiles.iter().map(|&c| letter_points(c)).sum()
}

/// End-of-game scoring adjustments. Every player loses the value of the tiles
/// left on their rack. If exactly one player emptied their rack (`finisher`),
/// they additionally gain the sum of everyone else's remaining tiles.
pub fn final_adjustments(racks: &[(String, Vec<char>)], finisher: Option<&str>) -> Vec<(String, i32)> {
    let total_remaining: i32 = racks.iter().map(|(_, t)| rack_value(t)).sum();
    racks
        .iter()
        .map(|(id, tiles)| {
            let own = rack_value(tiles);
            if Some(id.as_str()) == finisher {
                (id.clone(), total_remaining - own)
            } else {
                (id.clone(), -own)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::validation::WordCell;

    fn word(cells: Vec<WordCell>) -> FormedWord {
        let text = cells.iter().map(|c| c.letter).collect();
        FormedWord { text, cells }
    }

    fn cell(row: usize, col: usize, letter: char, is_new: bool) -> WordCell {
        WordCell {
            row,
            col,
            letter,
            is_blank: false,
            is_new,
        }
    }

    #[test]
    fn plain_word_no_premium() {
        // CAT on plain squares: 4 + 1 + 1 = 6.
        let w = word(vec![
            cell(7, 6, 'C', true),
            cell(7, 7, 'A', true),
            cell(7, 8, 'T', true),
        ]);
        assert_eq!(score_word(&w), 6);
    }

    #[test]
    fn triple_word_on_new_tile() {
        // C on (0,3)=TW: (4+1+1) * 3 = 18.
        let w = word(vec![
            cell(0, 3, 'C', true),
            cell(0, 4, 'A', true),
            cell(0, 5, 'T', true),
        ]);
        assert_eq!(score_word(&w), 18);
    }

    #[test]
    fn double_letter_under_new_tile() {
        // (1,5) is DL. AT with A on DL: A(1*2) + T(1) = 3.
        let w = word(vec![cell(1, 5, 'A', true), cell(1, 6, 'T', true)]);
        assert_eq!(score_word(&w), 3);
    }

    #[test]
    fn premium_ignored_for_existing_tile() {
        // Same DL square but the A is already on the board (not new): 1 + 1 = 2.
        let w = word(vec![cell(1, 5, 'A', false), cell(1, 6, 'T', true)]);
        assert_eq!(score_word(&w), 2);
    }

    #[test]
    fn blank_scores_zero() {
        let mut blank_c = cell(7, 6, 'C', true);
        blank_c.is_blank = true;
        let w = word(vec![blank_c, cell(7, 7, 'A', true), cell(7, 8, 'T', true)]);
        assert_eq!(score_word(&w), 2); // C(blank)=0 + A1 + T1
    }

    #[test]
    fn bingo_bonus_added_for_seven() {
        let w = word(vec![cell(7, 6, 'C', true), cell(7, 7, 'A', true)]);
        let (total, _) = score_move(std::slice::from_ref(&w), 7);
        assert_eq!(total, score_word(&w) + BINGO_BONUS);
        let (total6, _) = score_move(std::slice::from_ref(&w), 6);
        assert_eq!(total6, score_word(&w));
    }

    #[test]
    fn endgame_finisher_gains_others_tiles() {
        let racks = vec![
            ("a".to_string(), vec![]),            // went out
            ("b".to_string(), vec!['Q', 'Z']),    // 10 + 10 = 20 left
        ];
        let deltas = final_adjustments(&racks, Some("a"));
        assert_eq!(deltas[0], ("a".to_string(), 20));
        assert_eq!(deltas[1], ("b".to_string(), -20));
    }
}
