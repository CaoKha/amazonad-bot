use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of paid ad placement on Amazon search results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlacementType {
    /// Inline search result marked "Sponsored" (sp-sponsored-result / AdHolder / label)
    SponsoredProduct,
    /// Carousel widget at bottom of results (multi-ad-feedback-form-trigger JSON)
    SponsoredProductCarousel,
    /// Headline banner at top of page — brand logo + headline + 2-3 products
    SponsoredBrand,
    /// Inline video ad with product info
    SponsoredBrandVideo,
    /// "Editorial recommendations" / "Recommandations éditoriales" section
    EditorialRecommendation,
}

impl fmt::Display for PlacementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SponsoredProduct => write!(f, "Sponsored Product"),
            Self::SponsoredProductCarousel => write!(f, "SP Carousel"),
            Self::SponsoredBrand => write!(f, "Sponsored Brand"),
            Self::SponsoredBrandVideo => write!(f, "SB Video"),
            Self::EditorialRecommendation => write!(f, "Editorial Pick"),
        }
    }
}

/// A single product from amazon.fr search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub asin: String,
    pub title: String,
    pub position: usize,
    pub page: u32,
    pub position_in_page: usize,
    pub is_sponsored: bool,
    pub placement_type: Option<PlacementType>,
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
