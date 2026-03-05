use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single product from amazon.fr search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub asin: String,
    pub title: String,
    pub position: usize,
    pub page: u32,
    pub position_in_page: usize,
    pub is_sponsored: bool,
}

/// Outcome of scraping amazon.fr search results page.
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub results: Vec<SearchResult>,
    pub huawei_sponsored_found: bool,
    pub huawei_sponsored_positions: Vec<usize>,
    pub scraped_at: DateTime<Utc>,
}

/// Persisted state (saved to state.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorState {
    pub huawei_ad_visible: bool,
    pub huawei_positions: Vec<usize>,
    pub total_results_scraped: usize,
    pub updated_at: DateTime<Utc>,
}

/// What happened after a check cycle.
#[derive(Debug)]
pub enum CheckOutcome {
    /// Huawei sponsored ad appeared (wasn't there before)
    AdAppeared {
        positions: Vec<usize>,
        sample_title: String,
    },
    /// Huawei sponsored ad disappeared (was there before)
    AdDisappeared,
    /// No change from previous state
    NoChange,
    /// Scrape failed (CAPTCHA, network error, etc.)
    ScrapeError(String),
    /// First run, baseline saved
    FirstRun,
}
