//! Persistence for games and moves in the app's OWN SQLite database. Tables are
//! created by [`MIGRATIONS`] at startup.

use crate::helpers::{id, now};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, SqlitePool as DbPool};

/// Schema for the app's own SQLite (run at startup). Previously the package's
/// `LeoPackage::migrations()`; now self-owned since the data lives in words.db.
pub const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS wwf_games (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'active',
        state TEXT NOT NULL,
        player_ids TEXT NOT NULL,
        current_player_id TEXT NOT NULL,
        winner_id TEXT,
        created_by TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        finished_at TEXT
    )",
    "CREATE TABLE IF NOT EXISTS wwf_moves (
        id TEXT PRIMARY KEY,
        game_id TEXT NOT NULL,
        player_id TEXT NOT NULL,
        move_no INTEGER NOT NULL,
        kind TEXT NOT NULL,
        placements TEXT NOT NULL DEFAULT '[]',
        words TEXT NOT NULL DEFAULT '[]',
        score INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_wwf_moves_game ON wwf_moves(game_id)",
];

/// A persisted game plus its serialized engine state. A few columns are carried
/// for completeness even where the views recompute them from the engine.
#[allow(dead_code)]
pub struct GameRecord {
    pub id: String,
    pub name: String,
    pub status: String,
    pub state: Value,
    pub player_ids: Vec<String>,
    pub current_player_id: String,
    pub winner_id: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(FromRow)]
struct GameRow {
    id: String,
    name: String,
    status: String,
    state: String,
    player_ids: String,
    current_player_id: String,
    winner_id: Option<String>,
    created_by: String,
    created_at: String,
    updated_at: String,
}

impl From<GameRow> for GameRecord {
    fn from(r: GameRow) -> Self {
        GameRecord {
            id: r.id,
            name: r.name,
            status: r.status,
            state: serde_json::from_str(&r.state).unwrap_or(Value::Null),
            player_ids: serde_json::from_str(&r.player_ids).unwrap_or_default(),
            current_player_id: r.current_player_id,
            winner_id: r.winner_id,
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// One row of move history, with the player's display name resolved from leo.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MoveRecord {
    pub id: String,
    pub player_id: String,
    pub player_name: String,
    pub move_no: i64,
    pub kind: String,
    #[sqlx(rename = "words")]
    pub words_json: String,
    pub score: i64,
    pub created_at: String,
}

pub async fn insert_game(
    pool: &DbPool,
    name: &str,
    state: &Value,
    player_ids: &[String],
    current_player_id: &str,
    created_by: &str,
) -> Result<String, sqlx::Error> {
    let gid = id();
    let ts = now();
    sqlx::query(
        "INSERT INTO wwf_games
         (id, name, status, state, player_ids, current_player_id, created_by, created_at, updated_at)
         VALUES (?, ?, 'active', ?, ?, ?, ?, ?, ?)",
    )
    .bind(&gid)
    .bind(name)
    .bind(state.to_string())
    .bind(serde_json::to_string(player_ids).unwrap_or_else(|_| "[]".into()))
    .bind(current_player_id)
    .bind(created_by)
    .bind(&ts)
    .bind(&ts)
    .execute(pool)
    .await?;
    Ok(gid)
}

pub async fn get_game(pool: &DbPool, gid: &str) -> Result<Option<GameRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, GameRow>("SELECT * FROM wwf_games WHERE id = ?")
        .bind(gid)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(GameRecord::from))
}

pub async fn update_game(
    pool: &DbPool,
    gid: &str,
    state: &Value,
    status: &str,
    current_player_id: &str,
    winner_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let ts = now();
    let finished_at = if status == "active" { None } else { Some(ts.clone()) };
    sqlx::query(
        "UPDATE wwf_games SET state=?, status=?, current_player_id=?, winner_id=?,
         updated_at=?, finished_at=COALESCE(finished_at, ?) WHERE id=?",
    )
    .bind(state.to_string())
    .bind(status)
    .bind(current_player_id)
    .bind(winner_id)
    .bind(&ts)
    .bind(finished_at)
    .bind(gid)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_games_for_player(
    pool: &DbPool,
    player_id: &str,
) -> Result<Vec<GameRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, GameRow>("SELECT * FROM wwf_games ORDER BY updated_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(GameRecord::from)
        .filter(|g| g.player_ids.iter().any(|p| p == player_id))
        .collect())
}

pub async fn list_finished_games(pool: &DbPool) -> Result<Vec<GameRecord>, sqlx::Error> {
    let rows =
        sqlx::query_as::<_, GameRow>("SELECT * FROM wwf_games WHERE status != 'active'")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(GameRecord::from).collect())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_move(
    pool: &DbPool,
    game_id: &str,
    player_id: &str,
    move_no: i64,
    kind: &str,
    placements: &Value,
    words: &Value,
    score: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO wwf_moves
         (id, game_id, player_id, move_no, kind, placements, words, score, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id())
    .bind(game_id)
    .bind(player_id)
    .bind(move_no)
    .bind(kind)
    .bind(placements.to_string())
    .bind(words.to_string())
    .bind(score)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count_moves(pool: &DbPool, game_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM wwf_moves WHERE game_id = ?")
        .bind(game_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn list_moves(pool: &DbPool, game_id: &str) -> Result<Vec<MoveRecord>, sqlx::Error> {
    // No users table in the app's own DB — player_name is filled from the Leo
    // roster at the API layer. Return the id here as a placeholder.
    sqlx::query_as::<_, MoveRecord>(
        "SELECT m.id, m.player_id, m.player_id AS player_name,
                m.move_no, m.kind, m.words, m.score, m.created_at
         FROM wwf_moves m
         WHERE m.game_id = ? ORDER BY m.move_no",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}
