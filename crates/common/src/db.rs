use crate::models::{BadgeType, ScrapeResult};
use anyhow::{Context, Result};
use sqlx::PgPool;

/// Create a Postgres connection pool from a DATABASE_URL string.
pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPool::connect(database_url)
        .await
        .context("Failed to connect to Postgres")
}

/// Run embedded SQL migrations from the workspace `migrations/` directory.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")
}

/// Insert a scrape run and all its search results into the database.
/// Returns the run ID.
pub async fn insert_scrape_run(
    pool: &PgPool,
    marketplace: &str,
    keyword: &str,
    scrape_result: &ScrapeResult,
    brand_filter: &str,
) -> Result<i32> {
    let brand_lower = brand_filter.to_lowercase();
    let sponsored_count = scrape_result
        .results
        .iter()
        .filter(|r| r.is_sponsored)
        .count() as i32;
    let brand_match_count = scrape_result
        .results
        .iter()
        .filter(|r| {
            r.is_sponsored
                && (r.title.to_lowercase().contains(&brand_lower)
                    || r.brand
                        .as_ref()
                        .is_some_and(|b| b.to_lowercase().contains(&brand_lower)))
        })
        .count() as i32;

    // Insert the run
    let run_id = sqlx::query_scalar::<_, i32>(
        "INSERT INTO scrape_runs (marketplace, keyword, scraped_at, pages_scraped, total_results, sponsored_count, brand_match_count)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id"
    )
    .bind(marketplace)
    .bind(keyword)
    .bind(scrape_result.scraped_at)
    .bind(scrape_result.results.iter().map(|r| r.page).max().unwrap_or(0) as i32)
    .bind(scrape_result.results.len() as i32)
    .bind(sponsored_count)
    .bind(brand_match_count)
    .fetch_one(pool)
    .await
    .context("Failed to insert scrape_run")?;

    // Batch insert search results
    for r in &scrape_result.results {
        let placement_str = r.placement_type.as_ref().map(|p| format!("{}", p));
        let badge_str = r.badge.as_ref().map(|b| match b {
            BadgeType::BestSeller => "BestSeller",
            BadgeType::AmazonChoice => "AmazonChoice",
            BadgeType::HighlyRated => "HighlyRated",
        });

        sqlx::query(
            "INSERT INTO search_results (run_id, asin, title, brand, position, page, position_in_page, is_sponsored, placement_type, price, rating, review_count, is_prime, badge)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
        )
        .bind(run_id)
        .bind(&r.asin)
        .bind(&r.title)
        .bind(r.brand.as_deref())
        .bind(r.position as i32)
        .bind(r.page as i32)
        .bind(r.position_in_page as i32)
        .bind(r.is_sponsored)
        .bind(placement_str.as_deref())
        .bind(r.price.as_deref())
        .bind(r.rating)
        .bind(r.review_count.map(|v| v as i32))
        .bind(r.is_prime)
        .bind(badge_str)
        .execute(pool)
        .await
        .context("Failed to insert search_result")?;
    }

    Ok(run_id)
}
