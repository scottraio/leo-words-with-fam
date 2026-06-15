# Words With Fam

A full **Words With Friends** game for the family, built as an **in-process
[leo](https://leo) package**. It renders inside leo's own UI (no standalone app),
plays as your leo user, persists to leo's database, and sends "your turn" push
notifications through leo's APNs service.

## What it does

- **The full game** — 15×15 board with the real WWF premium-square layout, the
  104-tile WWF letter set (incl. 2 blanks), dictionary-validated plays (main +
  cross words), blanks, swaps, passes, resigns, end-game detection, and final
  rack-adjustment scoring. The rules engine lives in `src/engine/` and is pure
  and fully unit-tested.
- **Push notifications** — on every turn change the next player gets a push
  ("Your turn in <game>"); when a game ends everyone gets the result. Delivered
  via `ApnsService::send_to_user` under the `words_with_fam` notification
  category.
- **Score keeping** — per-move scores and words, running game totals, full move
  history, and a cross-game family leaderboard (wins, points, best word).

## Architecture

This is a single leo-package crate. It is developed here and **symlinked** into
the leo source tree so leo's build picks it up:

```
~/Leo/leo/packages/leo-words-with-fam  ->  ~/Work/leo-words-with-fam   (symlink)
```

| Piece | Path | Notes |
|-------|------|-------|
| Package entry | `src/lib.rs` | `LeoPackage` impl — manifest, the `play` sidebar page, DB migrations, and `reload()` which builds the API router. |
| Game engine | `src/engine/` | Pure rules: `constants`, `bag`, `board`, `dictionary`, `validation`, `scoring`, `game`. No I/O. |
| HTTP API | `src/api.rs` | Mounted by leo at `/p/words-with-fam/api/*`. Identity comes from the caller's leo session; push is a direct `ApnsService` call. |
| Persistence | `src/store.rs` | sqlx on leo's shared `DbPool`; tables `wwf_games`, `wwf_moves`. |
| Views | `src/view.rs` | Builds the JSON the UI consumes. |
| UI | `ui/` | `index.tsx` exports the `play` page; `WordsFam.tsx` is the board UI. Uses leo's shared React/HeroUI globals (`window.__LEO_SHARED__`); tap-to-place tiles (phone-friendly). |
| Dictionary | `data/enable1.txt` | The public-domain ENABLE word list (~172k words), embedded at compile time. |

Players are leo `users`, so push targeting and identity are native — no separate
login.

## Develop

The crate only builds as a member of the leo workspace (path deps resolve via
the symlink). From `~/Leo/leo`:

```bash
# Rust: engine tests + type-check the package
cargo test -p leo-words-with-fam
cargo check -p leo-words-with-fam

# UI bundle (built as part of leo's frontend) -> dist/plugins/words-with-fam/ui.js
npm --prefix frontend run build
```

## Install into leo

1. Symlink this repo into `~/Leo/leo/packages/leo-words-with-fam`.
2. Register it: add the path dep in `crates/leo-api/Cargo.toml` and
   `packages.push(Box::new(leo_words_with_fam::WordsWithFamPlugin));` in
   `crates/leo-api/src/plugins.rs`.
3. Build + restart leo: `cargo build --release` then point the binary at the
   build (`leo devmode on`) and `leo restart`.

The game then appears in leo's sidebar under **Family → Words**.
