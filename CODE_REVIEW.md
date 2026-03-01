# Code Review

## High Impact

### 1. SVG regenerated on every render — fixed
**File:** `crates/frontend/src/components/map_view.rs`

~~The entire SVG string (~8KB+) is rebuilt on every zoom, pan, and mousemove. This is the biggest performance bottleneck. Memoizing it so it only rebuilds when markers/pairings/faction change (not zoom) would help significantly.~~ **Addressed:** Wrapped SVG generation in `use_memo` so it only recomputes when positions, zoom, selection, faction, weapons, pairings, or accuracy radii change. Pan/drag signals are read outside the memo, so panning only updates the CSS transform. Also converted `accuracy_radii_px` from a plain `Vec` prop to a `Memo`/`ReadSignal` so the memo can reactively track wind-driven accuracy changes.

### 2. No responsive layout — fixed
**File:** `crates/frontend/assets/main.css`

~~Zero `@media` queries. The `320px` sidebar is fixed — the app is completely unusable on mobile or small screens. Even a simple collapse-sidebar breakpoint at 768px would help.~~ **Addressed:** Added `@media (max-width: 768px)` breakpoint that collapses the sidebar into a fixed-position slide-in drawer toggled by a hamburger button. Includes semi-transparent backdrop overlay, CSS transition animation, and Escape key to close. Header and placement buttons scale down for mobile. E2E tests cover both desktop (1280×720) and mobile (375×667) viewports.

### 3. `.unwrap()` on GraphQL context data — fixed
**File:** `crates/backend/src/graphql/mod.rs`

~~About a dozen `ctx.data::<Arc<...>>().unwrap()` calls. If context data is ever misconfigured, the server panics and crashes. These should return GraphQL errors instead.~~ **Addressed:** Added `ctx_data<T>()` helper that returns `async_graphql::Error` instead of panicking. All 13 `.unwrap()` calls replaced. Return types for `maps` and `weapons` queries updated to `Result`. 10 tests verify every resolver returns a clean error when context is missing.

### 4. No input validation on GraphQL mutations — fixed
**File:** `crates/backend/src/graphql/mod.rs`

~~Plan names have no length limit, coordinates aren't bounds-checked (could be NaN/Infinity), `gun_target_indices` can reference out-of-bounds targets, wind direction isn't validated to 0-360. A crafted request could produce garbage data.~~ **Addressed:** Added field-level validators (`validate_name`, `validate_map_id`, `validate_weapon_ids`, `validate_position`, `validate_positions`, `validate_gun_target_indices`, `validate_wind_direction`, `validate_wind_strength`) called from `validate_create_plan`. Rejects: names >200 chars, unknown map/weapon IDs, NaN/Infinity/out-of-bounds coordinates, out-of-bounds target indices, wind direction outside [0,360), wind strength >5. Empty weapon IDs are also allowed (guns placed without a weapon selected). Unused `update_plan` and `delete_plan` mutations were removed. 12 tests cover validation and happy paths.

### 5. Permissive CORS — fixed
**File:** `crates/backend/src/main.rs`

~~`CorsLayer::permissive()` allows any origin. Fine for a personal tool, but if this ever faces the internet, any site can make requests to the API.~~ **Addressed:** Replaced `CorsLayer::permissive()` with explicit origin allowlist. Set `CORS_ORIGIN=https://arty.dp42.dev` in production; defaults to `localhost:8080`/`localhost:3000` for development. Only allows GET/POST methods and `Content-Type` header.

---

## Medium Impact

### 6. Cloning vectors on every render — fixed
**File:** `crates/frontend/src/components/map_view.rs`

~~Five `.read().clone()` calls on signal vectors every render cycle. With many markers this creates unnecessary allocations. Could use refs directly.~~ **Addressed:** Replaced 5 `.read().clone()` calls in the `svg_html` `use_memo` with direct read guards. References passed via `&*guard` syntax, eliminating heap allocations on every SVG recomputation.

### 7. Hardcoded CSS colors not covered by theme toggle — fixed
**File:** `crates/frontend/assets/main.css`

~~`.target-label { fill: #ffe0b3 }` and `.spotter-label { fill: #b3d4f0 }` are hardcoded — they won't change with the Colonial theme. Should use CSS variables.~~ **Addressed:** Added `--target-label` and `--spotter-label` CSS variables to both Warden and Colonial themes, and `.target-label`/`.spotter-label` now use `var()` references.

### 8. `u32` for indices instead of `usize` — fixed
**File:** `crates/shared/src/models.rs:92`

~~`gun_target_indices: Vec<Option<u32>>` forces casts to `usize` everywhere it's used. Creates boilerplate and a theoretical truncation risk.~~ **Addressed:** Changed `gun_target_indices` from `Vec<Option<u32>>` to `Vec<Option<usize>>` in the shared `Plan` model. Updated backend GQL input conversion from `as u32` to `as usize`. Backend validation still casts `i32→usize` since it operates on the GQL wire format. Frontend wire format (`i32` from GraphQL Int) unchanged. Updated shared model tests.

### 9. No loading/error UI for resource fetches — fixed
**File:** `crates/frontend/src/pages/planner.rs`

~~Maps and weapons fetch silently — if they fail, the user gets an empty dropdown with no indication anything went wrong. Should show a loading spinner and error message.~~ **Addressed:** Added explicit loading state (spinner + "Loading game data...") and error state (error message + Retry button) as early returns before the main planner UI. CSS styles added for `.loading-state`, `.error-state`, and `.spinner` with keyframe animation.

### 10. Duplicated gun-target pairing logic — fixed
**File:** `crates/frontend/src/components/map_view.rs`

~~Pairing logic appears in at least 3 places: gun-selected click near target, target placement mode click near existing target, and normal target placement. Could be extracted into a shared function.~~ **Addressed:** Extracted `find_first_unpaired_target()` and `pair_first_unpaired_gun()` helper functions. All 3 call sites in `handle_marker_placement` (gun placement, target-near-existing, new target) now use the shared helpers.

### 11. No GraphQL resolver tests — fixed
**File:** `crates/backend/src/graphql/mod.rs`

~~Zero unit tests for any query or mutation handler. All backend logic is only tested via E2E, which is slow and can't cover edge cases well.~~ **Addressed:** 21 unit tests added covering missing-context errors (8), valid-context smoke tests (2), and input validation (11 — bad maps, bad weapons, out-of-bounds positions, negative coords, wind limits, index bounds, plus happy paths).

### 12. `println!` instead of structured logging — fixed
**File:** `crates/backend/src/main.rs:24, 85-86`

~~No log levels, timestamps, or structured output. Makes production debugging difficult.~~ **Addressed:** Replaced all `println!` calls with `tracing` macros (`info!`, `warn!`, `error!`). Added `tracing-subscriber` with `EnvFilter` for runtime log-level control via `RUST_LOG` env var. Default level is `foxhole_backend=info`. Structured fields used throughout (plan_id, map, weapon, error, etc.). Bind/serve failures now log at `error` level before exiting.

---

## Low Impact (easy wins)

### 13. Unreachable match arm — fixed
**File:** `crates/shared/src/grid.rs:84`

~~The `_ => 5` default in keypad mapping is unreachable since all 9 combinations of `(0..=2, 0..=2)` are covered. Should be `unreachable!()`.~~ **Addressed:** Changed to `unreachable!("kx and ky are clamped to 0..=2")`.

### 14. Gun == target edge case — documented
**File:** `crates/shared/src/calc.rs`

~~`atan2(0, 0)` returns 0.0 for azimuth when gun and target overlap. Not validated anywhere — should at least document this.~~ **Addressed:** Unit test `test_gun_equals_target` now documents that `atan2(0, -0.0)` yields 180° due to IEEE 754 negative zero, distance=0, out-of-range, and accuracy clamps to `acc_radius[0]`.

### 15. Plan save uses `alert()` for errors
**File:** `crates/frontend/src/pages/planner.rs:554-559`

Browser alert boxes are jarring. A toast or inline error message would be better UX.

### 16. Hardcoded database/assets paths — fixed
**File:** `crates/backend/src/main.rs:73, 76`

~~`"data/plans.redb"` and `"assets"` are hardcoded. Should be configurable via env vars for deployment flexibility.~~ **Addressed:** Now reads `DB_PATH` and `ASSETS_DIR` env vars with fallback defaults. Also uses `parent()` for directory creation instead of hardcoded `"data"`.

### 17. Playwright only tests Chromium
**File:** `playwright.config.ts:10`

No Firefox or WebKit in the test matrix.

### 18. Missing accessibility attributes

SVG markers have `pointer-events: none` and no ARIA labels. Form inputs lack associated `<label>` elements. Wind buttons have no `aria-label`.

---

## Test Coverage Gaps

- ~~No E2E test for plan save-and-load round trip~~ **Added:** `e2e/planner.spec.ts` — "Plan save and load" describe block with full save/navigate/verify round trip
- ~~No E2E test for localStorage persistence of faction across reload~~ **Added:** `e2e/planner.spec.ts` — "localStorage faction persists across reload" in Theme toggle block
- ~~No test for API/network failure scenarios~~ **Added:** `e2e/planner.spec.ts` — "Error handling" describe block using `page.route()` to intercept GraphQL with 500
- ~~No unit tests for backend GraphQL resolvers~~ **Added:** `crates/backend/src/graphql/mod.rs` — 25 tests (missing-context errors, validation rejections, happy paths)
- ~~No unit test for gun==target, wind strength >5, or NaN coordinate edge cases~~ **Added:** `crates/shared/src/calc.rs` — 5 new edge case tests (gun==target, wind strength 10, NaN, Infinity, wind strength 0)
- **Added:** `crates/backend/src/storage/mod.rs` — 4 new Plan CRUD tests (save/get, not-found, count, overwrite)
