//! The authoritative game state machine: racks, turn order, and the four move
//! kinds (play / swap / pass / resign), plus end-game detection and final
//! scoring. The whole `GameEngine` serializes to JSON for persistence.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::engine::bag::TileBag;
use crate::engine::board::{Board, PlacedTile, Placement};
use crate::engine::constants::{BLANK, RACK_SIZE};
use crate::engine::scoring::{WordScore, final_adjustments, score_move};
use crate::engine::validation::{MoveError, validate_play};

use crate::engine::dictionary::Dictionary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: String,
    pub name: String,
    pub rack: Vec<char>,
    pub score: i32,
    #[serde(default)]
    pub resigned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEngine {
    pub board: Board,
    pub bag: TileBag,
    pub players: Vec<PlayerState>,
    pub current_turn: usize,
    pub pass_streak: u32,
    pub move_count: u32,
    pub finished: bool,
    pub winner_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GameError {
    #[error("it is not your turn")]
    NotYourTurn,
    #[error("the game is already finished")]
    GameFinished,
    #[error("unknown player")]
    UnknownPlayer,
    #[error("you don't hold the tiles for that move")]
    MissingTiles,
    #[error("not enough tiles in the bag to swap")]
    CannotSwap,
    #[error("need at least two players")]
    NotEnoughPlayers,
    #[error(transparent)]
    Move(#[from] MoveError),
}

/// Result of a successful scoring move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOutcome {
    pub score: i32,
    pub words: Vec<WordScore>,
    pub game_over: bool,
}

impl GameEngine {
    /// Start a new game, dealing a full rack to each player. `players` is an
    /// ordered list of `(id, name)`; turn order follows the list.
    pub fn new(players: &[(String, String)], rng: &mut impl Rng) -> Result<Self, GameError> {
        if players.len() < 2 {
            return Err(GameError::NotEnoughPlayers);
        }
        let mut bag = TileBag::shuffled(rng);
        let players = players
            .iter()
            .map(|(id, name)| PlayerState {
                id: id.clone(),
                name: name.clone(),
                rack: bag.draw(RACK_SIZE),
                score: 0,
                resigned: false,
            })
            .collect();
        Ok(GameEngine {
            board: Board::empty(),
            bag,
            players,
            current_turn: 0,
            pass_streak: 0,
            move_count: 0,
            finished: false,
            winner_id: None,
        })
    }

    pub fn current_player_id(&self) -> &str {
        &self.players[self.current_turn].id
    }

    fn player_index(&self, id: &str) -> Result<usize, GameError> {
        self.players
            .iter()
            .position(|p| p.id == id)
            .ok_or(GameError::UnknownPlayer)
    }

    fn ensure_turn(&self, id: &str) -> Result<usize, GameError> {
        if self.finished {
            return Err(GameError::GameFinished);
        }
        let idx = self.player_index(id)?;
        if idx != self.current_turn {
            return Err(GameError::NotYourTurn);
        }
        Ok(idx)
    }

    fn active_count(&self) -> usize {
        self.players.iter().filter(|p| !p.resigned).count()
    }

    fn advance_turn(&mut self) {
        let n = self.players.len();
        for _ in 0..n {
            self.current_turn = (self.current_turn + 1) % n;
            if !self.players[self.current_turn].resigned {
                return;
            }
        }
    }

    /// Remove `tiles` from a rack if the rack holds them all (multiset check).
    fn take_from_rack(rack: &mut Vec<char>, tiles: &[char]) -> bool {
        let mut working = rack.clone();
        for &t in tiles {
            match working.iter().position(|&c| c == t) {
                Some(pos) => {
                    working.remove(pos);
                }
                None => return false,
            }
        }
        *rack = working;
        true
    }

    /// Play tiles onto the board.
    pub fn apply_play(
        &mut self,
        player_id: &str,
        placements: &[Placement],
        dict: &Dictionary,
        _rng: &mut impl Rng,
    ) -> Result<MoveOutcome, GameError> {
        let idx = self.ensure_turn(player_id)?;

        // Normalize letters to uppercase.
        let placements: Vec<Placement> = placements
            .iter()
            .map(|p| Placement {
                letter: p.letter.to_ascii_uppercase(),
                ..*p
            })
            .collect();

        // The tiles this move consumes from the rack.
        let needed: Vec<char> = placements
            .iter()
            .map(|p| if p.is_blank { BLANK } else { p.letter })
            .collect();
        if !Self::take_from_rack(&mut self.players[idx].rack.clone(), &needed) {
            return Err(GameError::MissingTiles);
        }

        // Validate the play (geometry + words + dictionary) before mutating.
        let words = validate_play(&self.board, &placements, dict)?;
        let (score, breakdown) = score_move(&words, placements.len());

        // Commit: remove tiles, place on board, score, refill.
        Self::take_from_rack(&mut self.players[idx].rack, &needed);
        for p in &placements {
            self.board.place(
                p.row,
                p.col,
                PlacedTile {
                    letter: p.letter,
                    is_blank: p.is_blank,
                },
            );
        }
        self.players[idx].score += score;
        let refill = self.bag.draw(placements.len());
        self.players[idx].rack.extend(refill);
        self.pass_streak = 0;
        self.move_count += 1;

        // End-game: a player emptied their rack with the bag exhausted.
        let mut game_over = false;
        if self.players[idx].rack.is_empty() && self.bag.is_empty() {
            self.finalize(Some(player_id.to_string()));
            game_over = true;
        } else {
            self.advance_turn();
        }

        Ok(MoveOutcome {
            score,
            words: breakdown,
            game_over,
        })
    }

    /// Swap tiles back into the bag for fresh ones. Forfeits the turn.
    pub fn apply_swap(
        &mut self,
        player_id: &str,
        tiles: &[char],
        rng: &mut impl Rng,
    ) -> Result<(), GameError> {
        let idx = self.ensure_turn(player_id)?;
        if self.bag.remaining() < tiles.len() || self.bag.is_empty() {
            return Err(GameError::CannotSwap);
        }
        let tiles: Vec<char> = tiles.iter().map(|c| c.to_ascii_uppercase()).collect();
        if !Self::take_from_rack(&mut self.players[idx].rack, &tiles) {
            return Err(GameError::MissingTiles);
        }
        let fresh = self.bag.draw(tiles.len());
        self.players[idx].rack.extend(fresh);
        self.bag.return_tiles(&tiles, rng);
        self.scoreless_turn();
        Ok(())
    }

    /// Pass the turn.
    pub fn apply_pass(&mut self, player_id: &str) -> Result<(), GameError> {
        self.ensure_turn(player_id)?;
        self.scoreless_turn();
        Ok(())
    }

    /// Resign from the game. If only one active player remains, the game ends.
    pub fn apply_resign(&mut self, player_id: &str) -> Result<(), GameError> {
        if self.finished {
            return Err(GameError::GameFinished);
        }
        let idx = self.player_index(player_id)?;
        let was_current = idx == self.current_turn;
        self.players[idx].resigned = true;

        if self.active_count() <= 1 {
            self.finalize(None);
        } else if was_current {
            self.advance_turn();
        }
        Ok(())
    }

    /// Shared handling for pass/swap: bump the scoreless streak, end the game
    /// after two full scoreless rounds, otherwise advance the turn.
    fn scoreless_turn(&mut self) {
        self.pass_streak += 1;
        self.move_count += 1;
        if self.pass_streak >= 2 * self.active_count() as u32 {
            self.finalize(None);
        } else {
            self.advance_turn();
        }
    }

    /// Apply end-game rack adjustments and decide the winner.
    fn finalize(&mut self, finisher: Option<String>) {
        let racks: Vec<(String, Vec<char>)> = self
            .players
            .iter()
            .filter(|p| !p.resigned)
            .map(|p| (p.id.clone(), p.rack.clone()))
            .collect();
        for (id, delta) in final_adjustments(&racks, finisher.as_deref()) {
            if let Some(p) = self.players.iter_mut().find(|p| p.id == id) {
                p.score += delta;
            }
        }
        self.finished = true;
        self.winner_id = self
            .players
            .iter()
            .filter(|p| !p.resigned)
            .max_by_key(|p| p.score)
            .map(|p| p.id.clone());
    }

    /// Serialize the whole game to JSON for persistence.
    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("GameEngine serializes")
    }

    pub fn from_snapshot(value: serde_json::Value) -> serde_json::Result<Self> {
        serde_json::from_value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn engine() -> (GameEngine, StdRng) {
        let mut rng = StdRng::seed_from_u64(42);
        let players = vec![
            ("a".to_string(), "Alice".to_string()),
            ("b".to_string(), "Bob".to_string()),
        ];
        (GameEngine::new(&players, &mut rng).unwrap(), rng)
    }

    #[test]
    fn new_game_deals_racks() {
        let (g, _) = engine();
        assert_eq!(g.players.len(), 2);
        assert_eq!(g.players[0].rack.len(), 7);
        assert_eq!(g.players[1].rack.len(), 7);
        assert_eq!(g.bag.remaining(), 104 - 14);
        assert_eq!(g.current_turn, 0);
    }

    #[test]
    fn needs_two_players() {
        let mut rng = StdRng::seed_from_u64(1);
        let one = vec![("a".to_string(), "A".to_string())];
        assert_eq!(
            GameEngine::new(&one, &mut rng).unwrap_err(),
            GameError::NotEnoughPlayers
        );
    }

    #[test]
    fn cannot_play_out_of_turn() {
        let (mut g, mut rng) = engine();
        let dict = Dictionary::from_words(["CAT"]);
        let placements = vec![Placement {
            row: 7,
            col: 7,
            letter: 'C',
            is_blank: false,
        }];
        let err = g.apply_play("b", &placements, &dict, &mut rng).unwrap_err();
        assert_eq!(err, GameError::NotYourTurn);
    }

    #[test]
    fn missing_tiles_rejected() {
        let (mut g, mut rng) = engine();
        // Force a known rack for player a.
        g.players[0].rack = vec!['C', 'A', 'T', 'X', 'Y', 'Z', 'Q'];
        let dict = Dictionary::from_words(["DOG", "DO"]);
        let placements = vec![
            Placement { row: 7, col: 7, letter: 'D', is_blank: false },
            Placement { row: 7, col: 8, letter: 'O', is_blank: false },
        ];
        assert_eq!(
            g.apply_play("a", &placements, &dict, &mut rng).unwrap_err(),
            GameError::MissingTiles
        );
    }

    #[test]
    fn full_play_scores_and_advances() {
        let (mut g, mut rng) = engine();
        g.players[0].rack = vec!['C', 'A', 'T', 'E', 'R', 'S', 'O'];
        let dict = Dictionary::from_words(["CAT", "CATS"]);
        let placements = vec![
            Placement { row: 7, col: 7, letter: 'C', is_blank: false },
            Placement { row: 7, col: 8, letter: 'A', is_blank: false },
            Placement { row: 7, col: 9, letter: 'T', is_blank: false },
        ];
        let outcome = g.apply_play("a", &placements, &dict, &mut rng).unwrap();
        assert!(outcome.score > 0);
        assert_eq!(outcome.words[0].text, "CAT");
        assert_eq!(g.players[0].rack.len(), 7); // refilled
        assert_eq!(g.current_turn, 1); // advanced to Bob
        assert!(!g.board.is_empty());
    }

    #[test]
    fn pass_advances_and_streak_ends_game() {
        let (mut g, _) = engine();
        // Two players → game ends after 4 consecutive scoreless turns.
        g.apply_pass("a").unwrap();
        assert_eq!(g.current_turn, 1);
        g.apply_pass("b").unwrap();
        g.apply_pass("a").unwrap();
        assert!(!g.finished);
        g.apply_pass("b").unwrap();
        assert!(g.finished);
    }

    #[test]
    fn resign_ends_two_player_game() {
        let (mut g, _) = engine();
        g.apply_resign("a").unwrap();
        assert!(g.finished);
        assert_eq!(g.winner_id.as_deref(), Some("b"));
    }

    #[test]
    fn snapshot_roundtrips() {
        let (g, _) = engine();
        let snap = g.snapshot();
        let restored = GameEngine::from_snapshot(snap).unwrap();
        assert_eq!(restored.players.len(), g.players.len());
        assert_eq!(restored.bag.remaining(), g.bag.remaining());
        assert_eq!(restored.current_turn, g.current_turn);
    }
}
