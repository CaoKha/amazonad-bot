#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;

use sqlx::PgPool;

fn main() {
    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://mts:mts@localhost:5435/mts".to_string());

    let pool = tauri::async_runtime::block_on(PgPool::connect(&db_url))
        .expect("Failed to connect to database");

    tauri::Builder::default()
        .manage(pool)
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshots,
            commands::get_sov_trend,
            commands::get_placement_mix,
            commands::get_top_competitors,
            commands::get_fr_gap,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
