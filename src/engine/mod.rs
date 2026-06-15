//! Pure Words With Friends game engine — no I/O, fully unit-tested.
//!
//! The board, tile bag, dictionary, move validation, scoring, and the
//! turn-by-turn [`GameEngine`] state machine. Everything is deterministic given
//! an RNG, and the whole engine serializes to JSON via
//! [`GameEngine::snapshot`] so the package can persist it in leo's database.

pub mod bag;
pub mod board;
pub mod constants;
pub mod dictionary;
pub mod game;
pub mod scoring;
pub mod validation;

pub use board::Placement;
pub use dictionary::Dictionary;
pub use game::GameEngine;
