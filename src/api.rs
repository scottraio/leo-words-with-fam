//! The game HTTP API. Leo proxies this at `/p/words-with-fam/api/*`.
//!
//! Identity comes from the `X-Leo-User-Id` header Leo's proxy injects (the
//! broker resolved the caller's session — the app never sees or validates
//! tokens for identity). The family roster (names/colors/avatars) is fetched
//! from `$LEO_API_URL/api/users` using the forwarded Bearer token. Game data
//! lives in the app's own SQLite. No leo-* crate dependencies.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::engine::game::MoveOutcome;
use crate::engine::{Dictionary, GameEngine, Placement};
use crate::store;
use crate::view::{self, ColorMap, GameSummary, GameView};

#[derive(Clone)]
pub struct ApiState {
    pub db: SqlitePool,
    /// Base URL of the Leo hub, for roster lookups (from `$LEO_API_URL`).
    pub leo_api_url: String,
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

// ── identity + roster (broker) ───────────────────────────────────────────────

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.trim().to_string())
}

/// The acting user, from the `X-Leo-User-Id` header Leo's proxy injects. The
/// proxy already authenticated the caller and strips any client-supplied value,
/// so this header is trusted.
fn auth_user(headers: &HeaderMap) -> ApiResult<String> {
    headers
        .get("x-leo-user-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or(ApiError::Unauthorized)
}

fn ustr(u: &Value, k: &str) -> String {
    u.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn uactive(u: &Value) -> bool {
    match u.get("is_active") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => true,
    }
}

fn color_of(u: &Value) -> String {
    let c = ustr(u, "color");
    if c.is_empty() { "#34D399".to_string() } else { c }
}

/// Fetch the family roster from Leo (`GET $LEO_API_URL/api/users`) with the
/// caller's forwarded token. Returns `[]` on any failure — the game degrades to
/// showing ids rather than erroring.
async fn fetch_roster(state: &ApiState, headers: &HeaderMap) -> Vec<Value> {
    let Some(token) = bearer(headers) else {
        return vec![];
    };
    let url = format!("{}/api/users", state.leo_api_url.trim_end_matches('/'));
    match reqwest::Client::new()
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.json::<Vec<Value>>().await.unwrap_or_default(),
        _ => vec![],
    }
}

fn colors_from(roster: &[Value]) -> ColorMap {
    roster
        .iter()
        .map(|u| (ustr(u, "id"), color_of(u)))
        .collect()
}

fn names_from(roster: &[Value]) -> HashMap<String, (String, String)> {
    roster
        .iter()
        .map(|u| (ustr(u, "id"), (ustr(u, "name"), color_of(u))))
        .collect()
}

async fn load(state: &ApiState, id: &str) -> ApiResult<(store::GameRecord, GameEngine)> {
    let rec = store::get_game(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("game not found".into()))?;
    let engine = GameEngine::from_snapshot(rec.state.clone())
        .map_err(|e| ApiError::Internal(format!("corrupt game state: {e}")))?;
    Ok((rec, engine))
}

// ── push (v1: degraded) ───────────────────────────────────────────────────────

/// Turn-change push is intentionally a no-op in the standalone build: notifying
/// *another* player requires a brokered push endpoint (Leo's `/api/push/send`
/// only self-notifies). Tracked as a follow-up; the game is fully playable
/// without it.
fn dispatch_push(_state: &ApiState, _game_name: String, _engine: &GameEngine) {}

// ── players ──────────────────────────────────────────────────────────────────

async fn list_players(State(state): State<ApiState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let _ = auth_user(&headers)?;
    let roster = fetch_roster(&state, &headers).await;
    let players: Vec<Value> = roster
        .iter()
        .filter(|u| uactive(u))
        .map(|u| {
            json!({
                "id": ustr(u, "id"),
                "name": ustr(u, "name"),
                "color": color_of(u),
                "avatar": ustr(u, "avatar"),
            })
        })
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
    let me = auth_user(&headers)?;

    let mut player_ids = vec![me.clone()];
    for o in body.opponent_ids {
        if !player_ids.contains(&o) {
            player_ids.push(o);
        }
    }
    if player_ids.len() < 2 {
        return Err(ApiError::BadRequest("pick at least one opponent".into()));
    }

    // Resolve display names from the Leo roster.
    let roster = fetch_roster(&state, &headers).await;
    let names = names_from(&roster);
    let mut pairs = Vec::with_capacity(player_ids.len());
    for pid in &player_ids {
        let name = names
            .get(pid)
            .map(|(n, _)| n.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| pid.clone());
        pairs.push((pid.clone(), name));
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
    let colors = colors_from(&roster);
    Ok(Json(view::build_view(&rec, &engine, &colors, Some(&me))))
}

async fn list_games(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<GameSummary>>> {
    let me = auth_user(&headers)?;
    let colors = colors_from(&fetch_roster(&state, &headers).await);
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
    let me = auth_user(&headers)?;
    let (rec, engine) = load(&state, &id).await?;
    let colors = colors_from(&fetch_roster(&state, &headers).await);
    Ok(Json(view::build_view(&rec, &engine, &colors, Some(&me))))
}

// ── moves ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MoveResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<MoveOutcome>,
    game: GameView,
}

#[allow(clippy::too_many_arguments)]
async fn finish_move(
    state: &ApiState,
    headers: &HeaderMap,
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
    let colors = colors_from(&fetch_roster(state, headers).await);
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
    let me = auth_user(&headers)?;
    let (rec, mut engine) = load(&state, &id).await?;
    let mut rng = rand::rngs::StdRng::from_entropy();
    let outcome = engine.apply_play(&me, &body.placements, &state.dict, &mut rng)?;
    let placements = serde_json::to_value(&body.placements).unwrap_or(json!([]));
    let words = serde_json::to_value(&outcome.words).unwrap_or(json!([]));
    let score = outcome.score;
    finish_move(
        &state, &headers, &rec, &engine, &me, "play", placements, words, score, Some(outcome),
    )
    .await
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
    let me = auth_user(&headers)?;
    let (rec, mut engine) = load(&state, &id).await?;
    let mut rng = rand::rngs::StdRng::from_entropy();
    engine.apply_swap(&me, &body.tiles, &mut rng)?;
    finish_move(
        &state, &headers, &rec, &engine, &me, "swap",
        json!({ "count": body.tiles.len() }), json!([]), 0, None,
    )
    .await
}

async fn pass(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<MoveResponse>> {
    let me = auth_user(&headers)?;
    let (rec, mut engine) = load(&state, &id).await?;
    engine.apply_pass(&me)?;
    finish_move(&state, &headers, &rec, &engine, &me, "pass", json!([]), json!([]), 0, None).await
}

async fn resign(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<MoveResponse>> {
    let me = auth_user(&headers)?;
    let (rec, mut engine) = load(&state, &id).await?;
    engine.apply_resign(&me)?;
    finish_move(&state, &headers, &rec, &engine, &me, "resign", json!([]), json!([]), 0, None).await
}

async fn history(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let _ = store::get_game(&state.db, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("game not found".into()))?;
    let names = names_from(&fetch_roster(&state, &headers).await);
    let moves = store::list_moves(&state.db, &id).await?;
    let out: Vec<Value> = moves
        .into_iter()
        .map(|m| {
            let player_name = names
                .get(&m.player_id)
                .map(|(n, _)| n.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| m.player_name.clone());
            json!({
                "id": m.id,
                "player_id": m.player_id,
                "player_name": player_name,
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

async fn leaderboard(State(state): State<ApiState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
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
            if let Ok(words) = serde_json::from_str::<Value>(&m.words_json)
                && let Some(arr) = words.as_array()
            {
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

    let names = names_from(&fetch_roster(&state, &headers).await);

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
