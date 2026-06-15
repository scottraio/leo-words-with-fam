//! The tile bag: a shuffled pool of tiles drawn onto racks.
//!
//! Blanks are stored as the [`BLANK`](crate::engine::constants::BLANK) character. The
//! bag is kept as a plain `Vec<char>` so it serializes trivially into the
//! persisted game snapshot.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::engine::constants::{BLANK, letter_count};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileBag {
    tiles: Vec<char>,
}

impl TileBag {
    /// A full, *unshuffled* bag (104 tiles incl. 2 blanks). Order is
    /// deterministic; call [`TileBag::shuffle`] before dealing.
    pub fn full() -> Self {
        let mut tiles = Vec::with_capacity(104);
        for b in b'A'..=b'Z' {
            let c = b as char;
            for _ in 0..letter_count(c) {
                tiles.push(c);
            }
        }
        tiles.push(BLANK);
        tiles.push(BLANK);
        TileBag { tiles }
    }

    /// A full bag, shuffled with the supplied RNG.
    pub fn shuffled(rng: &mut impl Rng) -> Self {
        let mut bag = Self::full();
        bag.shuffle(rng);
        bag
    }

    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        self.tiles.shuffle(rng);
    }

    /// Draw up to `n` tiles off the top of the bag (fewer if it runs out).
    pub fn draw(&mut self, n: usize) -> Vec<char> {
        let take = n.min(self.tiles.len());
        self.tiles.split_off(self.tiles.len() - take)
    }

    /// Return tiles to the bag (used by a swap) and reshuffle.
    pub fn return_tiles(&mut self, tiles: &[char], rng: &mut impl Rng) {
        self.tiles.extend_from_slice(tiles);
        self.shuffle(rng);
    }

    pub fn remaining(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::constants::BAG_SIZE;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn full_bag_size_and_blanks() {
        let bag = TileBag::full();
        assert_eq!(bag.remaining(), BAG_SIZE);
        let blanks = bag.tiles.iter().filter(|&&c| c == BLANK).count();
        assert_eq!(blanks, 2);
    }

    #[test]
    fn draw_reduces_remaining() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut bag = TileBag::shuffled(&mut rng);
        let hand = bag.draw(7);
        assert_eq!(hand.len(), 7);
        assert_eq!(bag.remaining(), BAG_SIZE - 7);
    }

    #[test]
    fn draw_past_empty_is_clamped() {
        let mut bag = TileBag::full();
        let all = bag.draw(1000);
        assert_eq!(all.len(), BAG_SIZE);
        assert!(bag.is_empty());
        assert!(bag.draw(5).is_empty());
    }

    #[test]
    fn return_tiles_roundtrips_count() {
        let mut rng = StdRng::seed_from_u64(2);
        let mut bag = TileBag::shuffled(&mut rng);
        let hand = bag.draw(3);
        let before = bag.remaining();
        bag.return_tiles(&hand, &mut rng);
        assert_eq!(bag.remaining(), before + 3);
    }
}
