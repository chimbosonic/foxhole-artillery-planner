# Foxhole Artillery Planner

A web-based artillery calculator and planning tool for [Foxhole](https://www.foxholegame.com/). Place guns, targets, and spotters on real in-game maps to get accurate firing solutions including azimuth, distance, and wind-adjusted corrections.

**Live at [arty.dp42.dev](https://arty.dp42.dev)**

## About

This project calculates artillery firing solutions using the same formulas the game uses — azimuth, distance, accuracy radius, and optional wind compensation. You can:

- Place multiple guns and targets on any active war map
- Get real-time firing solutions (azimuth, distance, accuracy)
- Adjust for wind direction and strength
- Select from all Colonial and Warden artillery weapons
- Save and share plans via URL
- Place spotters for coordination

Map assets by [Rustard's Improved Map Mod](https://rustard.itch.io/improved-map-mod).

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, [Axum](https://github.com/tokio-rs/axum), [async-graphql](https://github.com/async-graphql/async-graphql) |
| Frontend | Rust/WASM, [Dioxus](https://dioxuslabs.com/) 0.7 |
| Database | [ReDB](https://github.com/cberner/redb) (embedded key-value store) |
| Shared | Shared Rust crate for models and calculation logic |
| Build | [dioxus-cli](https://github.com/DioxusLabs/dioxus) (`dx`), Cargo |

## Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.80+)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [dioxus-cli](https://dioxuslabs.com/learn/0.6/getting_started): `cargo install dioxus-cli@0.7.3 --locked`
- (Optional) [Docker](https://www.docker.com/) for containerized builds

## Development

Run both the backend API server and the frontend dev server concurrently:

```bash
make dev
```

Or run them separately in two terminals:

```bash
# Terminal 1: Backend on http://localhost:3000
make backend

# Terminal 2: Frontend dev server on http://localhost:8080 (proxies API to backend)
make frontend
```

The frontend dev server (`dx serve`) proxies `/graphql` and `/static` requests to the backend on port 3000.

### GraphiQL Playground

Visit [http://localhost:3000/graphql](http://localhost:3000/graphql) in your browser for the interactive GraphQL playground.

## Testing

```bash
# All tests (unit + e2e)
make test

# Rust unit tests only
make test-unit

# Playwright end-to-end tests
make test-e2e
```

## Building for Production

### Native Build

```bash
# Build backend
cargo build -p foxhole-backend --release

# Build frontend WASM
cd crates/frontend && dx build --release --platform web

# Run (from project root)
./target/release/foxhole-backend
```

The backend serves the frontend from `dist/`, game assets from `assets/`, and stores plans in `data/plans.redb`.

### Docker Build

```bash
# Build the image
docker build -t foxhole-artillery-planner .

# Run the container
docker run -p 3000:3000 -v foxhole-data:/app/data foxhole-artillery-planner
```

The `-v foxhole-data:/app/data` flag persists saved plans across container restarts.

### Docker Compose

A `docker-compose.yml` is provided for convenience:

```bash
docker compose up -d
```

This starts the app on port 3000 with a named volume for database persistence. See [docker-compose.yml](docker-compose.yml) for details.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | HTTP server listen port |

## GraphQL API

The API is available at `/graphql`. Key queries and mutations:

### Queries

- `maps(activeOnly: Boolean)` — list available maps
- `weapons(faction: Faction)` — list weapons, optionally filtered by faction
- `calculate(input: CalculateInput!)` — compute a firing solution
- `plan(id: ID!)` — fetch a saved plan
- `stats` — server statistics

### Mutations

- `createPlan(input: CreatePlanInput!)` — save a new plan
- `updatePlan(input: UpdatePlanInput!)` — update an existing plan
- `deletePlan(id: ID!)` — delete a plan

### Stats API

Query server statistics including total saved plans and database size:

```bash
curl -s http://localhost:3000/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ stats { totalPlans dbSizeBytes } }"}' | python3 -m json.tool
```

Example response:

```json
{
    "data": {
        "stats": {
            "totalPlans": 42,
            "dbSizeBytes": 131072
        }
    }
}
```

## Project Structure

```
foxhole-artillery-planner/
├── Cargo.toml                  # Workspace root
├── Dockerfile                  # Multi-stage production build
├── docker-compose.yml          # Compose config
├── Makefile                    # Dev commands
├── assets/                     # Game data (maps, weapons, images)
│   ├── maps.json
│   ├── weapons.json
│   └── images/maps/            # Map image files
├── crates/
│   ├── backend/                # Axum + GraphQL API server
│   ├── frontend/               # Dioxus WASM web UI
│   └── shared/                 # Shared models & calculation logic
├── data/                       # Runtime database (created at startup)
│   └── plans.redb
└── dist/                       # Frontend build output (generated)
```

## Built with Claude

This project was built almost entirely with [Claude Code](https://claude.ai/claude-code) (Anthropic's AI coding agent) as a test of its capabilities. The architecture, implementation, bug fixes, and even this README were authored by Claude with human direction and review. It demonstrates Claude's ability to:

- Design and implement a full-stack Rust application from scratch
- Work across backend (Axum, GraphQL, embedded database), frontend (Dioxus, WASM), and shared library crates
- Write correct artillery math (azimuth, wind compensation, accuracy interpolation)
- Produce a multi-stage Dockerfile and CI-ready project structure
- Iterate on features like multi-gun support, selectable markers, and plan sharing
- Maintain clean code with passing clippy and tests throughout development

## License

MIT
