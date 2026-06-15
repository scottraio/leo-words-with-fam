//! API data-transfer objects and builders that turn an engine + player colors
//! into the JSON the UI consumes.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Value, json};

use crate::engine::GameEngine;
use crate::engine::constants::BOARD_SIZE;
use crate::store::GameRecord;

#[derive(Serialize)]
pub struct PlayerView {
    pub id: String,
    pub name: String,
    pub color: String,
    pub score: i32,
    pub rack_count: usize,
    pub resigned: bool,
}

#[derive(Serialize)]
pub struct GameView {
    pub id: String,
    pub name: String,
    pub status: String,
    pub board: Value,
    pub players: Vec<PlayerView>,
    pub current_player_id: String,
    pub bag_remaining: usize,
    pub move_count: u32,
    pub finished: bool,
    pub winner_id: Option<String>,
    pub your_rack: Vec<char>,
    pub your_player_id: Option<String>,
}

#[derive(Serialize)]
pub struct GameSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub current_player_id: String,
    pub current_player_name: String,
    pub players: Vec<PlayerView>,
    pub winner_id: Option<String>,
    pub updated_at: String,
}

pub type ColorMap = HashMap<String, String>;

fn color_of(colors: &ColorMap, id: &str) -> String {
    colors.get(id).cloned().unwrap_or_else(|| "#34D399".to_string())
}

fn player_views(engine: &GameEngine, colors: &ColorMap) -> Vec<PlayerView> {
    engine
        .players
        .iter()
        .map(|p| PlayerView {
            id: p.id.clone(),
            name: p.name.clone(),
            color: color_of(colors, &p.id),
            score: p.score,
            rack_count: p.rack.len(),
            resigned: p.resigned,
        })
        .collect()
}

/// A 15×15 row-major board; each cell is `null` or `{ letter, blank }`.
fn board_json(engine: &GameEngine) -> Value {
    let mut rows = Vec::with_capacity(BOARD_SIZE);
    for r in 0..BOARD_SIZE {
        let mut row = Vec::with_capacity(BOARD_SIZE);
        for c in 0..BOARD_SIZE {
            match engine.board.get(r, c) {
                Some(t) => row.push(json!({ "letter": t.letter, "blank": t.is_blank })),
                None => row.push(Value::Null),
            }
        }
        rows.push(Value::Array(row));
    }
    Value::Array(rows)
}

fn status_str(engine: &GameEngine) -> &'static str {
    if engine.finished { "finished" } else { "active" }
}

pub fn build_view(
    rec: &GameRecord,
    engine: &GameEngine,
    colors: &ColorMap,
    viewer: Option<&str>,
) -> GameView {
    let your_rack = viewer
        .and_then(|v| engine.players.iter().find(|p| p.id == v))
        .map(|p| p.rack.clone())
        .unwrap_or_default();
    GameView {
        id: rec.id.clone(),
        name: rec.name.clone(),
        status: status_str(engine).to_string(),
        board: board_json(engine),
        players: player_views(engine, colors),
        current_player_id: engine.current_player_id().to_string(),
        bag_remaining: engine.bag.remaining(),
        move_count: engine.move_count,
        finished: engine.finished,
        winner_id: engine.winner_id.clone(),
        your_rack,
        your_player_id: viewer.map(str::to_string),
    }
}

pub fn build_summary(rec: &GameRecord, engine: &GameEngine, colors: &ColorMap) -> GameSummary {
    let current_player_name = engine
        .players
        .get(engine.current_turn)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    GameSummary {
        id: rec.id.clone(),
        name: rec.name.clone(),
        status: status_str(engine).to_string(),
        current_player_id: engine.current_player_id().to_string(),
        current_player_name,
        players: player_views(engine, colors),
        winner_id: engine.winner_id.clone(),
        updated_at: rec.updated_at.clone(),
    }
}
