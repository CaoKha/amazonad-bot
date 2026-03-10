# mts-dashboard (Tauri v2 Backend)

Native desktop app backend. Connects to Postgres, serves data to the Leptos frontend via Tauri IPC commands.

## Quick Start

```bash
# Terminal 1 — frontend (must start first)
cd crates/dashboard-ui && trunk serve --port 1420

# Terminal 2 — backend + native window
cd crates/dashboard && cargo tauri dev
```

Requires:
- Postgres running on `localhost:5435` (see `docker-compose.yml` at repo root)
- `DATABASE_URL` set in `.env` or defaults to `postgresql://mts:mts@localhost:5435/mts`
- Trunk installed (`cargo install trunk`)
- Tauri CLI installed (`cargo install tauri-cli@^2`)

## Architecture

```
┌─────────────────────────────────────────┐
│  Tauri v2 native window                 │
│  ┌───────────────────────────────────┐  │
│  │  WebView (loads localhost:1420)   │  │
│  │  Leptos WASM calls:              │  │
│  │    window.__TAURI__.core.invoke() │  │
│  └──────────────┬────────────────────┘  │
│                 │ IPC                    │
│  ┌──────────────▼────────────────────┐  │
│  │  Rust backend (this crate)        │  │
│  │  commands.rs → SQL → PgPool       │  │
│  └──────────────┬────────────────────┘  │
│                 │                        │
│  ┌──────────────▼────────────────────┐  │
│  │  PostgreSQL (mts-postgres:5435)   │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Loads `.env`, connects PgPool, registers 5 Tauri commands |
| `src/commands.rs` | 5 `#[tauri::command]` functions, each runs a SQL query |
| `src/models.rs` | Serde structs for query results (mirrors `dashboard-ui/src/models.rs`) |
| `tauri.conf.json` | Tauri v2 config: `withGlobalTauri: true`, devUrl `localhost:1420` |
| `build.rs` | `tauri_build::build()` — required by Tauri |
| `icons/icon.png` | 32x32 RGBA app icon |
| `capabilities/default.json` | Tauri permission: `core:default` |

## IPC Commands

Each command takes no arguments, queries Postgres, returns JSON as `Result<String, String>`.

| Command | SQL Summary | Returns |
|---------|-------------|---------|
| `get_snapshots` | Latest row per (marketplace, keyword) from `scrape_runs` | `Vec<MarketSnapshot>` — current SOV per keyword |
| `get_sov_trend` | Daily avg SOV per marketplace, last 30 days | `Vec<SovPoint>` — time series for line chart |
| `get_placement_mix` | Count of sponsored results by placement type, last 30 days | `Vec<PlacementPoint>` — for stacked bar chart |
| `get_top_competitors` | Top 50 sponsored brands by frequency, last 30 days | `Vec<CompetitorRow>` — for table |
| `get_fr_gap` | Daily SOV pivot: FR vs DE vs ES, last 30 days | `Vec<FrGapPoint>` — for area chart |

## Adding a New Command

1. Add a model struct in `src/models.rs` (derive `Serialize, Deserialize`)
2. Add the `#[tauri::command]` function in `src/commands.rs`
3. Register it in `src/main.rs` inside `tauri::generate_handler![]`
4. Add the matching model + API wrapper in `dashboard-ui` (see that README)

## Key Gotchas

- **`withGlobalTauri: true`** in `tauri.conf.json` is required. Without it, the WASM frontend can't access `window.__TAURI__` for IPC.
- **`beforeDevCommand: ""`** — set to empty because Trunk must be started manually in a separate terminal. Tauri's CWD is unpredictable and breaks relative paths.
- **`frontendDist: "../dashboard-ui/dist"`** — points to Trunk's build output for production builds (`cargo tauri build`).
- **PgPool** is passed to commands via Tauri's `manage()` state. Access it as `pool: State<'_, PgPool>`.
- All commands return `Result<String, String>` (not typed JSON) because Tauri IPC serializes via `serde_json` and the WASM side deserializes the string.

## Database

See `migrations/001_initial_schema.sql` at repo root. Two tables:

- **`scrape_runs`** — one row per scrape session (marketplace, keyword, scraped_at, counts)
- **`search_results`** — one row per product (asin, title, brand, position, is_sponsored, etc.)

The dashboard only reads from these tables. The scraper (`mts-scraper`) writes to them.
