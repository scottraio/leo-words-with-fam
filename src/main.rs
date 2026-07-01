//! Words With Fam — standalone remote app package.
//!
//! Binds `$LEO_APP_PORT`, keeps its own SQLite at `$LEO_DATA_DIR/words.db`, and
//! serves the game API that Leo proxies at `/p/words-with-fam/*`. Identity comes
//! from the `X-Leo-User-Id` header Leo's proxy injects; the family roster comes
//! from `$LEO_API_URL/api/users`. Started by Leo's `spawn_app` with `start`.

mod api;
mod engine;
mod helpers;
mod store;
mod view;

use std::sync::Arc;

use crate::engine::Dictionary;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let port: u16 = std::env::var("LEO_APP_PORT")
        .ok()
        .or_else(|| std::env::var("PORT").ok())
        .and_then(|p| p.parse().ok())
        .unwrap_or(8422);
    let data_dir = std::env::var("LEO_DATA_DIR").unwrap_or_else(|_| ".".to_string());
    let leo_api_url =
        std::env::var("LEO_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8420".to_string());

    // Own SQLite — created if absent.
    let db_url = format!("sqlite://{data_dir}/words.db?mode=rwc");
    let db = sqlx::SqlitePool::connect(&db_url).await?;
    for m in store::MIGRATIONS {
        sqlx::query(m).execute(&db).await?;
    }

    let dict = Arc::new(Dictionary::from_words(
        include_str!("../data/enable1.txt").lines(),
    ));

    let state = api::ApiState {
        db,
        leo_api_url,
        dict,
    };
    // Leo proxies /p/words-with-fam/<rest> here verbatim, and the frontend calls
    // /p/words-with-fam/api/*, so mount the game router under /api.
    let app = axum::Router::new().nest("/api", api::router(state));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!("words-with-fam listening on 127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
