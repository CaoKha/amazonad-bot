use std::collections::HashMap;
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

/// Badge displayed on a product in search results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BadgeType {
    BestSeller,
    AmazonChoice,
    HighlyRated,
}

impl fmt::Display for BadgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BestSeller => write!(f, "🏆 Meilleur vendeur"),
            Self::AmazonChoice => write!(f, "✅ Choix d'Amazon"),
            Self::HighlyRated => write!(f, "⭐ Très bien noté"),
        }
    }
}


/// A single product from amazon.fr search results.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResult {
    pub asin: String,
    pub title: String,
    pub position: usize,
    pub page: u32,
    pub position_in_page: usize,
    pub is_sponsored: bool,
    pub placement_type: Option<PlacementType>,
    pub price: Option<String>,
    pub rating: Option<f32>,
    pub review_count: Option<u32>,
    pub is_prime: bool,
    pub badge: Option<BadgeType>,
    pub brand: Option<String>,
}

/// Outcome of scraping amazon.fr search results page.
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub results: Vec<SearchResult>,
    pub huawei_sponsored_found: bool,
    pub huawei_sponsored_positions: Vec<usize>,
    pub scraped_at: DateTime<Utc>,
}

/// Per-keyword monitoring state, stored in MonitorState.keywords.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeywordState {
    /// Whether the monitored brand's ad was visible in the last sweep.
    pub brand_ad_visible: bool,
    /// Positions where the brand's ads appeared: (page, position_in_page, placement_type).
    pub brand_positions: Vec<(u32, usize, Option<PlacementType>)>,
    /// When brand visibility last changed (appeared or disappeared).
    pub last_changed: Option<DateTime<Utc>>,
    /// When this keyword was last checked.
    pub last_checked: Option<DateTime<Utc>>,
    /// Full result set from the last sweep (cached for bot commands).
    pub last_results: Vec<SearchResult>,
}

/// Top-level persisted state — one entry per monitored keyword.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonitorState {
    pub keywords: HashMap<String, KeywordState>,
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
