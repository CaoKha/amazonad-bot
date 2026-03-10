use crate::api;
use crate::components::competitors::CompetitorsTable;
use crate::components::frgap::FrGapChart;
use crate::components::placement::PlacementChart;
use crate::components::snapshot::SnapshotCards;
use crate::components::sov::SovChart;
use gloo_timers::callback::Interval;
use leptos::prelude::*;
use wasm_bindgen::JsValue;

/// Check synchronously whether window.__TAURI__.core exists.
fn tauri_status() -> String {
    let Some(window) = web_sys::window() else {
        return "❌ no window object".to_string();
    };
    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null());
    let Some(tauri) = tauri else {
        return "❌ __TAURI__ not injected (is trunk running? inside Tauri?)".to_string();
    };
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core"))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null());
    if core.is_some() {
        "✅ Tauri IPC ready".to_string()
    } else {
        "⚠️ __TAURI__ found but .core missing".to_string()
    }
}

#[component]
pub fn App() -> impl IntoView {
    let refresh = RwSignal::new(0u32);

    // Check IPC once on mount (synchronous — __TAURI__ injected before scripts run).
    let ipc_status = StoredValue::new(tauri_status());

    // 30-second auto-refresh
    let interval = Interval::new(30_000, move || refresh.update(|n| *n += 1));
    std::mem::forget(interval);

    // Resources now return Result so errors bubble up to the UI.
    let snapshots = LocalResource::new(move || {
        refresh.get();
        async move { api::get_snapshots().await }
    });
    let sov_trend = LocalResource::new(move || {
        refresh.get();
        async move { api::get_sov_trend().await }
    });
    let placement_mix = LocalResource::new(move || {
        refresh.get();
        async move { api::get_placement_mix().await }
    });
    let top_competitors = LocalResource::new(move || {
        refresh.get();
        async move { api::get_top_competitors().await }
    });
    let fr_gap = LocalResource::new(move || {
        refresh.get();
        async move { api::get_fr_gap().await }
    });

    view! {
        <div class="min-h-screen bg-gray-900 text-white p-6">
            // ── Header ──────────────────────────────────────────────────────────
            <header class="flex items-center justify-between mb-8">
                <h1 class="text-3xl font-bold tracking-tight">"🟦 MTS Ad Monitor"</h1>
                <div class="text-right space-y-1">
                    <div class="text-xs font-mono">{ipc_status.get_value()}</div>
                    <div class="text-gray-500 text-xs">
                        "Auto-refresh 30s · cycle #" {move || refresh.get()}
                    </div>
                </div>
            </header>

            // ── Snapshot cards ───────────────────────────────────────────────────
            <section class="mb-8">
                {move || match snapshots.get() {
                    None => view! {
                        <p class="text-gray-400 animate-pulse text-sm">"⟳ Loading snapshots…"</p>
                    }.into_any(),
                    Some(res) => match (*res).clone() {
                        Err(e) => view! {
                            <div class="p-4 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
                                <b>"Snapshots error: "</b>{e}
                            </div>
                        }.into_any(),
                        Ok(data) if data.is_empty() => view! {
                            <p class="text-gray-500 text-sm">"No snapshot data yet — scraper still collecting."</p>
                        }.into_any(),
                        Ok(data) => view! { <SnapshotCards data=data /> }.into_any(),
                    },
                }}
            </section>

            // ── Row 1: SOV trend + FR gap ────────────────────────────────────────
            <section class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
                {move || match sov_trend.get() {
                    None => view! {
                        <p class="text-gray-400 animate-pulse text-sm">"⟳ Loading SOV trend…"</p>
                    }.into_any(),
                    Some(res) => match (*res).clone() {
                        Err(e) => view! {
                            <div class="p-4 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
                                <b>"SOV error: "</b>{e}
                            </div>
                        }.into_any(),
                        Ok(data) => view! { <SovChart data=data /> }.into_any(),
                    },
                }}
                {move || match fr_gap.get() {
                    None => view! {
                        <p class="text-gray-400 animate-pulse text-sm">"⟳ Loading FR gap…"</p>
                    }.into_any(),
                    Some(res) => match (*res).clone() {
                        Err(e) => view! {
                            <div class="p-4 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
                                <b>"FR gap error: "</b>{e}
                            </div>
                        }.into_any(),
                        Ok(data) => view! { <FrGapChart data=data /> }.into_any(),
                    },
                }}
            </section>

            // ── Row 2: Placement mix + Competitors ───────────────────────────────
            <section class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                {move || match placement_mix.get() {
                    None => view! {
                        <p class="text-gray-400 animate-pulse text-sm">"⟳ Loading placement mix…"</p>
                    }.into_any(),
                    Some(res) => match (*res).clone() {
                        Err(e) => view! {
                            <div class="p-4 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
                                <b>"Placement error: "</b>{e}
                            </div>
                        }.into_any(),
                        Ok(data) => view! { <PlacementChart data=data /> }.into_any(),
                    },
                }}
                {move || match top_competitors.get() {
                    None => view! {
                        <p class="text-gray-400 animate-pulse text-sm">"⟳ Loading competitors…"</p>
                    }.into_any(),
                    Some(res) => match (*res).clone() {
                        Err(e) => view! {
                            <div class="p-4 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
                                <b>"Competitors error: "</b>{e}
                            </div>
                        }.into_any(),
                        Ok(data) => view! { <CompetitorsTable data=data /> }.into_any(),
                    },
                }}
            </section>
        </div>
    }
}
