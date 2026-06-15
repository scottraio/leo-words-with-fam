//! Words With Fam — an in-process leo package.
//!
//! A full Words With Friends game for the family, rendered inside leo's own UI.
//! The package contributes a sidebar page (its React bundle lives in `ui/`), an
//! HTTP API mounted at `/p/words-with-fam/api/*`, tables in leo's database, and
//! turn-change push notifications via leo's APNs service.

mod api;
mod engine;
mod store;
mod view;

use std::sync::Arc;

use async_trait::async_trait;
use leo_apns::apns::ApnsService;
use leo_package::{
    FieldDef, LeoPackage, PackageCategory, PackageContext, PackageManifest, PackagePage,
    PackageState,
};

use crate::engine::Dictionary;

pub struct WordsWithFamPlugin;

/// The push category used for turn/game notifications. Created on reload.
const PUSH_CATEGORY: &str = "words_with_fam";

const MIGRATIONS: &[&str] = &[
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

#[async_trait]
impl LeoPackage for WordsWithFamPlugin {
    fn manifest(&self) -> PackageManifest {
        PackageManifest {
            name: "words-with-fam",
            label: "Words With Fam",
            description: "Play Words With Friends with your family.",
            details: "A full Words With Friends game — 15×15 board, the real WWF tile set and \
                      premium squares, dictionary-validated plays, blanks, swaps, scoring, and \
                      turn-change push notifications. Each family member plays as their leo user.",
            setup_hint: "",
            use_cases: &[
                "Start a game of Words With Fam with Steph",
                "Whose turn is it in our word game?",
            ],
            icon: "puzzle-piece",
            category: PackageCategory::Package,
            tools: &[],
            dependencies: &[],
        }
    }

    fn fields(&self) -> Vec<FieldDef> {
        vec![]
    }

    fn migrations(&self) -> &[&str] {
        MIGRATIONS
    }

    fn pages(&self) -> Vec<PackagePage> {
        vec![PackagePage {
            path: "play",
            label: "Words",
            icon: "puzzle-piece",
            sidebar_group: Some("Family"),
            settings_page: false,
        }]
    }

    async fn reload(&self, ctx: &PackageContext) -> anyhow::Result<PackageState> {
        // Ensure the push category exists and is on-by-default for the family
        // (without overriding anyone who later turns it off).
        let _ = leo_db::notification_categories::get_or_create_category(
            &ctx.db,
            PUSH_CATEGORY,
            "Words With Fam",
            Some("puzzle-piece"),
            Some("Turn alerts for Words With Fam"),
        )
        .await;
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO user_notification_preferences (user_id, category_id, enabled)
             SELECT id, ?, 1 FROM users WHERE is_active = 1",
        )
        .bind(PUSH_CATEGORY)
        .execute(&ctx.db)
        .await;

        // The bundled ENABLE word list is embedded at compile time.
        let dict = Arc::new(Dictionary::from_words(
            include_str!("../data/enable1.txt").lines(),
        ));

        let state = api::ApiState {
            db: ctx.db.clone(),
            apns: ctx.service::<ApnsService>(),
            dict,
        };

        Ok(PackageState {
            router: Some(api::router(state)),
            ..Default::default()
        })
    }

    async fn test(&self, _ctx: &PackageContext) -> (bool, String) {
        (true, "Words With Fam ready".to_string())
    }
}
