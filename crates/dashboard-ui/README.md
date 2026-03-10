# mts-dashboard-ui (Leptos Frontend)

WASM frontend for the dashboard. Compiled with Trunk, rendered inside the Tauri webview. Calls the Tauri backend via IPC, renders charts with ECharts 5.

## Quick Start

```bash
# Dev (hot-reload)
cd crates/dashboard-ui && trunk serve --port 1420

# Build (production)
cd crates/dashboard-ui && trunk build --release
```

Port 1420 must match `devUrl` in `crates/dashboard/tauri.conf.json`.

## Architecture

```
index.html
  ├── ECharts 5 (CDN)
  ├── Tailwind CSS (CDN)
  ├── mts_update_chart() JS helper
  └── WASM blob (Leptos app)

main.rs → mount_to_body(App)
  └── app.rs (App component)
        ├── SnapshotCards   — stat cards grid
        ├── SovChart        — ECharts line (SOV trend 30d)
        ├── FrGapChart      — ECharts line+area (FR vs DE/ES)
        ├── PlacementChart  — ECharts stacked bar (placement types)
        └── CompetitorsTable — HTML table (top brands)
```

## This Is a Standalone Workspace

`Cargo.toml` has `[workspace]` at the top. This is intentional — it prevents the root workspace from trying to compile this crate with native targets. This crate only builds for `wasm32-unknown-unknown` via Trunk.

## Files

| File | Purpose |
|------|---------|
| `index.html` | Entry point. Loads ECharts CDN, Tailwind CDN, defines `mts_update_chart()` JS bridge |
| `Trunk.toml` | Trunk config: port 1420, output to `dist/` |
| `src/main.rs` | Sets panic hook, mounts Leptos `App` component to body |
| `src/app.rs` | Root `App` component: IPC check, 5 `LocalResource`s, 30s auto-refresh, layout |
| `src/api.rs` | Tauri IPC wrappers: `wasm_bindgen` extern for `invoke()`, 5 typed API functions |
| `src/models.rs` | Data structs mirroring backend models (Deserialize, Clone, Default) |
| `src/chart.rs` | `update_chart(id, json)` — calls `window.mts_update_chart()` via `web_sys::Reflect` |
| `src/components/` | One file per visual section (see below) |

## Components

| Component | File | Chart Type | ECharts ID |
|-----------|------|------------|------------|
| `SnapshotCards` | `snapshot.rs` | Card grid (no chart) | — |
| `SovChart` | `sov.rs` | Line chart (3 series: FR/DE/ES) | `sov-chart` |
| `FrGapChart` | `frgap.rs` | Line + area (FR filled, DE/ES dashed) | `frgap-chart` |
| `PlacementChart` | `placement.rs` | Stacked bar (by placement type) | `placement-chart` |
| `CompetitorsTable` | `competitors.rs` | HTML table | — |

Each chart component follows the same pattern:

```rust
#[component]
pub fn SovChart(data: Vec<SovPoint>) -> impl IntoView {
    let option_json = build_option(&data);       // Build ECharts JSON
    let json_clone = option_json.clone();
    Timeout::new(0, move || {                     // Defer to next tick (DOM must exist)
        chart::update_chart("sov-chart", &json_clone);
    }).forget();
    view! {
        <div class="bg-gray-800 ...">
            <div id="sov-chart" style="height: 320px;"></div>  // ECharts container
        </div>
    }
}
```

## Data Flow

```
1. App component creates 5 LocalResources, each calling an api::get_*() function
2. api::get_*() calls window.__TAURI__.core.invoke("command_name")
3. Tauri backend runs SQL, returns JSON string
4. api::get_*() deserializes JSON into Vec<Model>
5. LocalResource triggers reactive update → component re-renders
6. Chart components call mts_update_chart(id, json) to push options to ECharts
7. Every 30 seconds, refresh signal increments → all resources re-fetch
```

## Leptos 0.7 Patterns Used

**`LocalResource` (not `Resource`)** — WASM futures aren't `Send`. `LocalResource` has no `Send` bound.

```rust
// LocalResource takes ONE closure that returns a future (not source + fetcher)
let data = LocalResource::new(move || {
    refresh.get();                     // Track dependency synchronously
    async move { api::get_something().await }
});

// .get() returns Option<SendWrapper<T>>
// Use (*result).clone() to unwrap SendWrapper
match data.get() {
    None => /* loading */,
    Some(res) => match (*res).clone() {
        Ok(val) => /* render val */,
        Err(e) => /* show error */,
    },
}
```

**`StoredValue`** — for values computed once on mount (not reactive):
```rust
let ipc_status = StoredValue::new(tauri_status());
```

**`Interval` + `forget()`** — for auto-refresh timer:
```rust
let interval = Interval::new(30_000, move || refresh.update(|n| *n += 1));
std::mem::forget(interval);  // Must forget or it's dropped immediately
```

## ECharts Bridge

Charts are rendered by ECharts 5 (loaded via CDN in `index.html`). Rust can't call ECharts directly, so there's a JS bridge:

```
index.html:   window.mts_update_chart(id, optionJson)  // JS function
chart.rs:     update_chart(id, json)                    // Rust wrapper via web_sys::Reflect
components/:  build_option(&data) → JSON string         // Each component builds its ECharts option
```

To modify a chart: edit the `build_option()` function in the relevant component file. The JSON structure follows ECharts option spec: https://echarts.apache.org/en/option.html

## Adding a New Chart

1. Add model in `src/models.rs` (must match backend model)
2. Add API wrapper in `src/api.rs`:
   ```rust
   pub async fn get_new_data() -> Result<Vec<NewModel>, String> {
       let json = tauri_invoke("get_new_data").await?;
       serde_json::from_str(&json).map_err(|e| e.to_string())
   }
   ```
3. Create component in `src/components/new_chart.rs` following the pattern above
4. Export in `src/components/mod.rs`
5. Add `LocalResource` + render block in `src/app.rs`
6. Add the corresponding `#[tauri::command]` in the backend (see dashboard README)

## Key Gotchas

- **`Timeout::new(0, ...)`** is required for chart rendering. ECharts needs the DOM element to exist before `init()`. The zero-ms timeout defers execution to after Leptos mounts the view.
- **`.forget()` on Interval/Timeout** — Rust drops these immediately otherwise, canceling them.
- **`(*res).clone()` on `SendWrapper`** — `LocalResource.get()` returns `Option<SendWrapper<T>>`. Deref with `*` then clone to get the inner `T`.
- **Models use `String` for dates** (not `chrono::DateTime`) — WASM serde doesn't have chrono support by default, and the backend already formats dates as strings.
- **CDN dependencies** — ECharts and Tailwind are loaded from CDN in `index.html`. No npm/node required. For offline use, download and serve locally.
