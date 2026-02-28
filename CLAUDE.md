# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
make dev              # Run backend + frontend concurrently
make backend          # Backend only on :3000
make frontend         # Frontend dev server on :8080 (proxies to backend)
```

## Testing & Linting

```bash
cargo test --workspace          # All unit tests (shared + backend + frontend)
cargo test -p foxhole-shared    # Shared crate tests only
cargo test -p foxhole-frontend  # Frontend tests only
cargo clippy -- -D warnings     # Lint (treat warnings as errors)
npx playwright test             # E2E tests (auto-starts both servers)
npx playwright test --grep "test name"  # Run specific e2e test
```

## Production Build

```bash
cargo build -p foxhole-backend --release
cd crates/frontend && dx build --release --platform web
```

## Architecture

Rust workspace with three crates:

- **`crates/shared`** — Models (`Weapon`, `Plan`, `FiringSolution`, `Position`), grid math, artillery calculations. Used by both backend and frontend. Feature `uuid-support` is enabled for backend but disabled for WASM frontend.
- **`crates/backend`** — Axum server on port 3000. GraphQL API (`async-graphql`), embedded ReDB database (`data/plans.redb`), static file serving with cache headers.
- **`crates/frontend`** — Dioxus 0.7 WASM app. Dev server on port 8080 proxies `/graphql` and `/static` to backend (configured in `Dioxus.toml`).

## Coordinate Systems

Three coordinate spaces are used throughout the codebase. Tracking which space you're in is critical:

1. **Map-image pixels** (0–2048 × 0–1776) — Internal logical coordinate space. All position state (`gun_positions`, `target_positions`, `spotter_positions`) stored in this space. Constants in `crates/shared/src/grid.rs`.
2. **Meters** (0–2184 × 0–1890) — World coordinates used for artillery math and plan storage. Conversion via `grid::px_to_meters()` / `grid::meters_to_px()`.
3. **Grid coordinates** ("G9k5") — Display format. 17 columns (A–Q) × 15 rows (1–15), each cell has a 3×3 numpad sub-grid (k1–k9).

Browser click coordinates go through: client → container-relative → undo zoom/pan transform → map-image pixels. See `crates/frontend/src/coords.rs`.

## Gun-Target Pairing

Guns and targets use explicit pairing via `gun_target_indices: Vec<Option<usize>>`. Entry `[i] = Some(j)` means gun `i` fires at target `j`. `None` = unpaired. Each gun also has its own weapon via `gun_weapon_ids: Vec<String>`. Old plans with single positions auto-migrate on load.

## Frontend State

All UI state lives as Dioxus `Signal`s in `crates/frontend/src/pages/planner.rs`. Firing solutions auto-recalculate reactively when gun positions, target positions, weapon selections, or wind inputs change.

## Game Assets

- `assets/maps.json` / `assets/weapons.json` — Game data loaded at backend startup
- `assets/images/maps/*.webp` — Map images (2048×1776 pixels)

## Rules
Don't commit code
