use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Latest snapshot per (marketplace, keyword) pair.
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub marketplace: String,
    pub keyword: String,
    pub scraped_at: DateTime<Utc>,
    pub total_results: i32,
    pub sponsored_count: i32,
    pub brand_match_count: i32,
    pub sov_pct: f64,
}

/// Daily average share-of-voice per marketplace.
#[derive(Debug, Serialize, Deserialize)]
pub struct SovPoint {
    pub day: String,
    pub marketplace: String,
    pub avg_sov: f64,
}

/// Count of sponsored results by placement type and marketplace.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlacementPoint {
    pub marketplace: String,
    pub placement_type: String,
    pub count: i64,
}

/// Competitor brand appearing in sponsored slots.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompetitorRow {
    pub marketplace: String,
    pub keyword: String,
    pub brand: String,
    pub times_seen: i64,
    pub avg_position: f64,
}

/// Daily SOV comparison across FR / DE / ES.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrGapPoint {
    pub day: String,
    pub fr_sov: f64,
    pub de_sov: f64,
    pub es_sov: f64,
}
