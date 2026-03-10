use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketSnapshot {
    pub marketplace: String,
    pub keyword: String,
    pub scraped_at: String,
    pub total_results: i32,
    pub sponsored_count: i32,
    pub brand_match_count: i32,
    pub sov_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SovPoint {
    pub day: String,
    pub marketplace: String,
    pub avg_sov: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlacementPoint {
    pub marketplace: String,
    pub placement_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompetitorRow {
    pub marketplace: String,
    pub keyword: String,
    pub brand: String,
    pub times_seen: i64,
    pub avg_position: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrGapPoint {
    pub day: String,
    pub fr_sov: f64,
    pub de_sov: f64,
    pub es_sov: f64,
}
