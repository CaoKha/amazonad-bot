use crate::models::*;
use sqlx::{PgPool, Row};
use tauri::State;

#[tauri::command]
pub async fn get_snapshots(pool: State<'_, PgPool>) -> Result<String, String> {
    let rows = sqlx::query(
        "SELECT DISTINCT ON (marketplace, keyword) \
             marketplace, keyword, scraped_at, total_results, sponsored_count, brand_match_count \
         FROM scrape_runs \
         ORDER BY marketplace, keyword, scraped_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let snapshots: Vec<MarketSnapshot> = rows
        .iter()
        .map(|row| {
            let sponsored: i32 = row.get("sponsored_count");
            let brand_match: i32 = row.get("brand_match_count");
            MarketSnapshot {
                marketplace: row.get("marketplace"),
                keyword: row.get("keyword"),
                scraped_at: row.get("scraped_at"),
                total_results: row.get("total_results"),
                sponsored_count: sponsored,
                brand_match_count: brand_match,
                sov_pct: if sponsored > 0 {
                    brand_match as f64 / sponsored as f64 * 100.0
                } else {
                    0.0
                },
            }
        })
        .collect();

    serde_json::to_string(&snapshots).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_sov_trend(pool: State<'_, PgPool>) -> Result<String, String> {
    let rows = sqlx::query(
        "SELECT \
             TO_CHAR(DATE_TRUNC('day', scraped_at), 'YYYY-MM-DD') as day, \
             marketplace, \
             AVG(CASE WHEN sponsored_count > 0 \
                 THEN brand_match_count::float8 / sponsored_count::float8 * 100.0 \
                 ELSE 0.0 END)::float8 as avg_sov \
         FROM scrape_runs \
         WHERE scraped_at >= NOW() - INTERVAL '30 days' \
         GROUP BY DATE_TRUNC('day', scraped_at), marketplace \
         ORDER BY day",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let points: Vec<SovPoint> = rows
        .iter()
        .map(|row| SovPoint {
            day: row.get("day"),
            marketplace: row.get("marketplace"),
            avg_sov: row.get("avg_sov"),
        })
        .collect();

    serde_json::to_string(&points).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_placement_mix(pool: State<'_, PgPool>) -> Result<String, String> {
    let rows = sqlx::query(
        "SELECT \
             r.marketplace, \
             COALESCE(sr.placement_type, 'Unknown') as placement_type, \
             COUNT(*)::bigint as count \
         FROM search_results sr \
         JOIN scrape_runs r ON sr.run_id = r.id \
         WHERE sr.is_sponsored = true \
             AND r.scraped_at >= NOW() - INTERVAL '30 days' \
         GROUP BY r.marketplace, sr.placement_type \
         ORDER BY r.marketplace, count DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let points: Vec<PlacementPoint> = rows
        .iter()
        .map(|row| PlacementPoint {
            marketplace: row.get("marketplace"),
            placement_type: row.get("placement_type"),
            count: row.get("count"),
        })
        .collect();

    serde_json::to_string(&points).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_top_competitors(pool: State<'_, PgPool>) -> Result<String, String> {
    let rows = sqlx::query(
        "SELECT \
             r.marketplace, \
             r.keyword, \
             COALESCE(sr.brand, 'Unknown') as brand, \
             COUNT(*)::bigint as times_seen, \
             AVG(sr.position::float8)::float8 as avg_position \
         FROM search_results sr \
         JOIN scrape_runs r ON sr.run_id = r.id \
         WHERE sr.is_sponsored = true \
             AND r.scraped_at >= NOW() - INTERVAL '30 days' \
         GROUP BY r.marketplace, r.keyword, sr.brand \
         ORDER BY times_seen DESC \
         LIMIT 50",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let competitors: Vec<CompetitorRow> = rows
        .iter()
        .map(|row| CompetitorRow {
            marketplace: row.get("marketplace"),
            keyword: row.get("keyword"),
            brand: row.get("brand"),
            times_seen: row.get("times_seen"),
            avg_position: row.get("avg_position"),
        })
        .collect();

    serde_json::to_string(&competitors).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_fr_gap(pool: State<'_, PgPool>) -> Result<String, String> {
    let rows = sqlx::query(
        "WITH daily_sov AS ( \
             SELECT \
                 TO_CHAR(DATE_TRUNC('day', scraped_at), 'YYYY-MM-DD') as day, \
                 marketplace, \
                 AVG(CASE WHEN sponsored_count > 0 \
                     THEN brand_match_count::float8 / sponsored_count::float8 * 100.0 \
                     ELSE 0.0 END)::float8 as avg_sov \
             FROM scrape_runs \
             WHERE scraped_at >= NOW() - INTERVAL '30 days' \
             GROUP BY DATE_TRUNC('day', scraped_at), marketplace \
         ) \
         SELECT \
             day, \
             COALESCE(MAX(CASE WHEN marketplace = 'FR' THEN avg_sov END), 0.0)::float8 as fr_sov, \
             COALESCE(MAX(CASE WHEN marketplace = 'DE' THEN avg_sov END), 0.0)::float8 as de_sov, \
             COALESCE(MAX(CASE WHEN marketplace = 'ES' THEN avg_sov END), 0.0)::float8 as es_sov \
         FROM daily_sov \
         GROUP BY day \
         ORDER BY day",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let points: Vec<FrGapPoint> = rows
        .iter()
        .map(|row| FrGapPoint {
            day: row.get("day"),
            fr_sov: row.get("fr_sov"),
            de_sov: row.get("de_sov"),
            es_sov: row.get("es_sov"),
        })
        .collect();

    serde_json::to_string(&points).map_err(|e| e.to_string())
}
