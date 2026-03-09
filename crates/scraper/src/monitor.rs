use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tracing::{info, warn};

use crate::amazon_scraper::AmazonScraper;
use crate::config::TelegramConfig;
use mts_common::models::KeywordState;
use mts_common::notifier::{SponsoredEntry, TelegramNotifier};
use mts_common::state::StateManager;

pub struct MonitorEngine {
    scraper: Arc<AmazonScraper>,
    state_manager: Arc<StateManager>,
    http_client: reqwest::Client,
    telegram_config: Arc<TelegramConfig>,
    brand_filter: String,
    keywords: Vec<String>,
    marketplace_url: String,
}

impl MonitorEngine {
    pub fn new(
        scraper: Arc<AmazonScraper>,
        state_manager: Arc<StateManager>,
        http_client: reqwest::Client,
        telegram_config: Arc<TelegramConfig>,
        brand_filter: String,
        keywords: Vec<String>,
        marketplace_url: String,
    ) -> Self {
        Self {
            scraper,
            state_manager,
            http_client,
            telegram_config,
            brand_filter,
            keywords,
            marketplace_url,
        }
    }

    pub async fn run_check(&self) -> Result<()> {
        // Launch ONE browser for the entire sweep
        let (mut browser, handle) = self.scraper.launch_browser().await?;

        let mut state = self.state_manager.load()?.unwrap_or_default();
        let brand_lower = self.brand_filter.to_lowercase();

        for keyword in &self.keywords {
            info!("Scraping keyword: '{}'", keyword);

            let scrape_result = match self
                .scraper
                .scrape_all_pages_with_browser(&browser, keyword)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Scrape failed for keyword '{}': {:#}", keyword, e);
                    continue; // per-keyword error: log and continue
                }
            };

            if scrape_result.results.is_empty() {
                warn!(
                    "Keyword '{}': 0 results — possible block, skipping state update",
                    keyword
                );
                continue;
            }

            info!(
                "Keyword '{}': scraped {} results",
                keyword,
                scrape_result.results.len()
            );

            let prev_ks = state
                .keywords
                .get(keyword.as_str())
                .cloned()
                .unwrap_or_default();

            // Determine brand visibility
            let brand_ad_visible = scrape_result
                .results
                .iter()
                .any(|r| {
                    r.is_sponsored
                        && (r.title.to_lowercase().contains(&brand_lower)
                            || r.brand.as_ref().map_or(false, |b| b.to_lowercase().contains(&brand_lower)))
                });

            let brand_positions: Vec<(u32, usize, Option<mts_common::models::PlacementType>)> =
                scrape_result
                    .results
                    .iter()
                    .filter(|r| r.is_sponsored && (r.title.to_lowercase().contains(&brand_lower) || r.brand.as_ref().map_or(false, |b| b.to_lowercase().contains(&brand_lower))))
                    .map(|r| (r.page, r.position_in_page, r.placement_type.clone()))
                    .collect();

            let now = Utc::now();
            let last_changed = if brand_ad_visible != prev_ks.brand_ad_visible {
                Some(now)
            } else {
                prev_ks.last_changed
            };

            // Send Telegram notification if state changed
            let search_url = AmazonScraper::build_search_url(&self.marketplace_url, keyword, 1);
            let notifier = match TelegramNotifier::new(
                &self.telegram_config,
                self.http_client.clone(),
                keyword.clone(),
                search_url,
            ) {
                Ok(n) => n,
                Err(e) => {
                    warn!(
                        "Failed to create notifier for keyword '{}': {:#}",
                        keyword, e
                    );
                    // Still update state even if notifier fails
                    let new_ks = KeywordState {
                        brand_ad_visible,
                        brand_positions,
                        last_changed,
                        last_checked: Some(now),
                        last_results: scrape_result.results,
                    };
                    state.keywords.insert(keyword.clone(), new_ks);
                    continue;
                }
            };

            if !prev_ks.brand_ad_visible && brand_ad_visible {
                info!("Keyword '{}': brand ad APPEARED", keyword);
                let sample_title = scrape_result
                    .results
                    .iter()
                    .find(|r| r.is_sponsored && (r.title.to_lowercase().contains(&brand_lower) || r.brand.as_ref().map_or(false, |b| b.to_lowercase().contains(&brand_lower))))
                    .map(|r| r.title.clone())
                    .unwrap_or_default();

                let all_sponsored: Vec<SponsoredEntry> = scrape_result
                    .results
                    .iter()
                    .filter(|r| r.is_sponsored)
                    .map(|r| {
                        (
                            r.page,
                            r.position_in_page,
                            r.title.clone(),
                            r.placement_type.clone(),
                            r.price.clone(),
                            r.rating,
                            r.review_count,
                            r.is_prime,
                            r.badge.clone(),
                        )
                    })
                    .collect();

                if let Err(e) = notifier
                    .send_ad_appeared(&brand_positions, &sample_title, &all_sponsored)
                    .await
                {
                    warn!(
                        "Failed to send ad appeared notification for '{}': {:#}",
                        keyword, e
                    );
                }
            } else if prev_ks.brand_ad_visible && !brand_ad_visible {
                info!("Keyword '{}': brand ad DISAPPEARED", keyword);
                if let Err(e) = notifier.send_ad_disappeared().await {
                    warn!(
                        "Failed to send ad disappeared notification for '{}': {:#}",
                        keyword, e
                    );
                }
            } else {
                info!(
                    "Keyword '{}': no change (brand_visible={})",
                    keyword, brand_ad_visible
                );
            }

            // Update keyword state
            let new_ks = KeywordState {
                brand_ad_visible,
                brand_positions,
                last_changed,
                last_checked: Some(now),
                last_results: scrape_result.results,
            };
            state.keywords.insert(keyword.clone(), new_ks);
        }

        // Close browser AFTER all keywords
        browser.close().await.ok();
        handle.await.ok();

        // Save state ONCE after the full sweep
        self.state_manager.save(&state)?;
        info!("Sweep complete. State saved.");

        Ok(())
    }
}
