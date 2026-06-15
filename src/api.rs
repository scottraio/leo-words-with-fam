//! The package's HTTP API, mounted by leo at `/p/words-with-fam/api/*`.
//!
//! Identity comes from the caller's leo session token (so a player can only act
//! as themselves). Persistence uses leo's shared DB; push notifications call
//! `ApnsService` directly — no cross-process bridge.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use leo_apns::apns::ApnsService;
use leo_db::DbPool;
use rand::SeedableRng;

use crate::engine::game::MoveOutcome;
use crate::engine::{Dictionary, GameEngine, Placement};
use crate::store;
use crate::view::{self, ColorMap, GameSummary, GameView};

const PUSH_CATEGORY: &str = "words_with_fam";

#[derive(Clone)]
pub struct ApiState {
    pub db: DbPool,
    pub apns: Option<Arc<ApnsService>>,
    pub dict: Arc<Dictionary>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/players", get(list_players))
        .route("/games", get(list_games).post(create_game))
        .route("/games/{id}", get(get_game))
        .route("/games/{id}/moves", get(history).post(play))
        .route("/games/{id}/swap", post(swap))
        .route("/games/{id}/pass", post(pass))
        .route("/games/{id}/resign", post(resign))
        .route("/leaderboard", get(leaderboard))
        .with_state(state)
}

// ── errors ─────────────────────────────────────────────────────────────────

enum ApiError {
    Unauthorized,
    NotFound(String),
    BadRequest(String),
    Conflict(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(format!("database error: {e}"))
    }
}
impl From<crate::engine::game::GameError> for ApiError {
    fn from(e: crate::engine::game::GameError) -> Self {
        ApiError::Conflict(e.to_string())
    }
}

type ApiResult<T> = Result<T, ApiError>;

// ── auth + helpers ───────────────────────────────────────────────────────────

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.trim().to_string())
}

/// Resolve the acting leo user from the session token.
async fn auth_user(state: &ApiState, headers: &HeaderMap) -> ApiResult<String> {
    let token = bearer(headers).ok_or(ApiError::Unauthorized)?;
    let session = leo_db::sessions::get_session_by_token(&state.db, &token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    Ok(session.user_id)
}

/// Map of leo user id → color, for rendering player chips.
async fn color_map(db: &DbPool) -> ColorMap {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, color FROM users")
        .fetch_all(db)
        .await
        .unwrap_or_default();
    rows.into_iter().collect()
}

async fn load(state: &ApiState, id: &str) -> ApiResult<(store::GameRecord, GameEngine)> {
    let rec = store::get_game(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("game not found".into()))?;
    let engine = GameEngine::from_snapshot(rec.state.clone())
        .map_err(|e| ApiError::Internal(format!("corrupt game state: {e}")))?;
    Ok((rec, engine))
}

// ── push ─────────────────────────────────────────────────────────────────────

/// After any move, nudge whoever's turn it now is; on game end, tell everyone.
fn dispatch_push(state: &ApiState, game_name: String, engine: &GameEngine) {
    let Some(apns) = state.apns.clone() else { return };
    let db = state.db.clone();
    let finished = engine.finished;
    let current = engine.current_player_id().to_string();
    let winner = engine.winner_id.clone();
    let participants: Vec<(String, String)> =
        engine.players.iter().map(|p| (p.id.clone(), p.name.clone())).collect();

    tokio::spawn(async move {
        if finished {
            let winner_name = winner
                .as_ref()
                .and_then(|w| participants.iter().find(|(id, _)| id == w))
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| "Nobody".into());
            for (uid, _) in &participants {
                let _ = apns
                    .send_to_user(
                        &db,
                        uid,
                        PUSH_CATEGORY,
                        &format!("Game over — {game_name}"),
                        &format!("{winner_name} wins!"),
                        None,
                        None,
                    )
                    .await;
            }
        } else {
            let _ = apns
                .send_to_user(
                    &db,
                    &current,
                    PUSH_CATEGORY,
                    &format!("Your turn in {game_name}"),
                    "Tap to play your move.",
                    None,
                    None,
                )
                .await;
        }
    });
}

// ── players ──────────────────────────────────────────────────────────────────

async fn list_players(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let _ = auth_user(&state, &headers).await?;
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT id, name, color, avatar FROM users WHERE is_active = 1 ORDER BY name")
            .fetch_all(&state.db)
            .await?;
    let players: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, color, avatar)| json!({ "id": id, "name": name, "color": color, "avatar": avatar }))
        .collect();
    Ok(Json(json!(players)))
}

// ── games ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateGame {
    #[serde(default)]
    name: String,
    /// The other players (the creator is added automatically and goes first).
    opponent_ids: Vec<String>,
}

async fn create_game(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<CreateGame>,
) -> ApiResult<Json<GameView>> {
    let me = auth_user(&state, &headers).await?;

    let mut player_ids = vec![me.clone()];
    for o in body.opponent_ids {
        if !player_ids.contains(&o) {
            player_ids.push(o);
        }
    }
    if player_ids.len() < 2 {
        return Err(ApiError::BadRequest("pick at least one opponent".into()));
    }

    // Resolve display names from leo's users.
    let mut pairs = Vec::with_capacity(player_ids.len());
    for pid in &player_ids {
        let user = leo_db::users::get_user(&state.db, pid)
            .await?
            .ok_or_else(|| ApiError::BadRequest(format!("unknown player: {pid}")))?;
        pairs.push((user.id, user.name));
    }

    let mut rng = rand::rngs::StdRng::from_entropy();
    let engine = GameEngine::new(&pairs, &mut rng)?;

    let name = if body.name.trim().is_empty() {
        "Family Game".to_string()
    } else {
        body.name.trim().to_string()
    };
    let gid = store::insert_game(
        &state.db,
        &name,
        &engine.snapshot(),
        &player_ids,
        engine.current_player_id(),
        &me,
    )
    .await?;

    dispatch_push(&state, name.clone(), &engine);

    let rec = store::get_game(&state.db, &gid)
        .await?
        .ok_or_else(|| ApiError::Internal("game vanished after insert".into()))?;
    let colors = color_map(&state.db).await;
    Ok(Json(view::build_view(&rec, &engine, &colors, Some(&me))))
}

async fn list_games(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<GameSummary>>> {
    let me = auth_user(&state, &headers).await?;
    let colors = color_map(&state.db).await;
    let mut out = Vec::new();
    for rec in store::list_games_for_player(&state.db, &me).await? {
        if let Ok(engine) = GameEngine::from_snapshot(rec.state.clone()) {
            out.push(view::build_summary(&rec, &engine, &colors));
        }
    }
    Ok(Json(out))
}

async fn get_game(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<GameView>> {
    let me = auth_user(&state, &headers).await?;
    let (rec, engine) = load(&state, &id).await?;
    let colors = color_map(&state.db).await;
    Ok(Json(view::build_view(&rec, &engine, &colors, Some(&me))))
}

// ── moves ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MoveResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<MoveOutcome>,
    game: GameView,
}

async fn finish_move(
    state: &ApiState,
    rec: &store::GameRecord,
    engine: &GameEngine,
    actor: &str,
    kind: &str,
    placements: Value,
    words: Value,
    score: i32,
    outcome: Option<MoveOutcome>,
) -> ApiResult<Json<MoveResponse>> {
    let status = if engine.finished { "finished" } else { "active" };
    store::update_game(
        &state.db,
        &rec.id,
        &engine.snapshot(),
        status,
        engine.current_player_id(),
        engine.winner_id.as_deref(),
    )
    .await?;
    let move_no = store::count_moves(&state.db, &rec.id).await? + 1;
    store::insert_move(&state.db, &rec.id, actor, move_no, kind, &placements, &words, score).await?;
    dispatch_push(state, rec.name.clone(), engine);
    let colors = color_map(&state.db).await;
    let game = view::build_view(rec, engine, &colors, Some(actor));
    Ok(Json(MoveResponse { outcome, game }))
}

#[derive(Deserialize)]
struct PlayReq {
    placements: Vec<Placement>,
}

async fn play(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<PlayReq>,
) -> ApiResult<Json<MoveResponse>> {
    let me = auth_user(&state, &headers).await?;
    let (rec, mut engine) = load(&state, &id).await?;
    let mut rng = rand::rngs::StdRng::from_entropy();
    let outcome = engine.apply_play(&me, &body.placements, &state.dict, &mut rng)?;
    let placements = serde_json::to_value(&body.placements).unwrap_or(json!([]));
    let words = serde_json::to_value(&outcome.words).unwrap_or(json!([]));
    let score = outcome.score;
    finish_move(&state, &rec, &engine, &me, "play", placements, words, score, Some(outcome)).await
}

#[derive(Deserialize)]
struct SwapReq {
    tiles: Vec<char>,
}

async fn swap(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SwapReq>,
) -> ApiResult<Json<MoveResponse>> {
    let me = auth_user(&state, &headers).await?;
    let (rec, mut engine) = load(&state, &id).await?;
    let mut rng = rand::rngs::StdRng::from_entropy();
    engine.apply_swap(&me, &body.tiles, &mut rng)?;
    finish_move(&state, &rec, &engine, &me, "swap", json!({ "count": body.tiles.len() }), json!([]), 0, None).await
}

async fn pass(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<MoveResponse>> {
    let me = auth_user(&state, &headers).await?;
    let (rec, mut engine) = load(&state, &id).await?;
    engine.apply_pass(&me)?;
    finish_move(&state, &rec, &engine, &me, "pass", json!([]), json!([]), 0, None).await
}

async fn resign(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<MoveResponse>> {
    let me = auth_user(&state, &headers).await?;
    let (rec, mut engine) = load(&state, &id).await?;
    engine.apply_resign(&me)?;
    finish_move(&state, &rec, &engine, &me, "resign", json!([]), json!([]), 0, None).await
}

async fn history(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let _ = store::get_game(&state.db, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("game not found".into()))?;
    let moves = store::list_moves(&state.db, &id).await?;
    let out: Vec<Value> = moves
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id,
                "player_id": m.player_id,
                "player_name": m.player_name,
                "move_no": m.move_no,
                "kind": m.kind,
                "words": serde_json::from_str::<Value>(&m.words_json).unwrap_or(json!([])),
                "score": m.score,
                "created_at": m.created_at,
            })
        })
        .collect();
    Ok(Json(json!(out)))
}

// ── leaderboard ──────────────────────────────────────────────────────────────

#[derive(Default)]
struct Agg {
    games_played: u32,
    games_won: u32,
    total_points: i64,
    best_word: String,
    best_word_score: i32,
}

async fn leaderboard(State(state): State<ApiState>) -> ApiResult<Json<Value>> {
    let mut agg: HashMap<String, Agg> = HashMap::new();

    for rec in store::list_finished_games(&state.db).await? {
        if let Ok(engine) = GameEngine::from_snapshot(rec.state.clone()) {
            for p in &engine.players {
                let e = agg.entry(p.id.clone()).or_default();
                e.games_played += 1;
                e.total_points += p.score as i64;
                if engine.winner_id.as_deref() == Some(p.id.as_str()) {
                    e.games_won += 1;
                }
            }
        }
        for m in store::list_moves(&state.db, &rec.id).await? {
            if let Ok(words) = serde_json::from_str::<Value>(&m.words_json) {
                if let Some(arr) = words.as_array() {
                    for w in arr {
                        let pts = w.get("points").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let text = w.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let e = agg.entry(m.player_id.clone()).or_default();
                        if pts > e.best_word_score {
                            e.best_word_score = pts;
                            e.best_word = text.to_string();
                        }
                    }
                }
            }
        }
    }

    let names: HashMap<String, (String, String)> = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, name, color FROM users",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, name, color)| (id, (name, color)))
    .collect();

    let mut out: Vec<Value> = agg
        .into_iter()
        .map(|(id, a)| {
            let (name, color) = names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| (id.clone(), "#34D399".to_string()));
            json!({
                "player_id": id,
                "name": name,
                "color": color,
                "games_played": a.games_played,
                "games_won": a.games_won,
                "total_points": a.total_points,
                "best_word": a.best_word,
                "best_word_score": a.best_word_score,
            })
        })
        .collect();

    out.sort_by(|a, b| {
        let aw = a["games_won"].as_u64().unwrap_or(0);
        let bw = b["games_won"].as_u64().unwrap_or(0);
        let ap = a["total_points"].as_i64().unwrap_or(0);
        let bp = b["total_points"].as_i64().unwrap_or(0);
        bw.cmp(&aw).then(bp.cmp(&ap))
    });

    Ok(Json(json!(out)))
}
