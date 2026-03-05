use std::sync::Arc;
use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::amazon_scraper::AmazonScraper;
use mts_common::models::{CheckOutcome, MonitorState};
use mts_common::notifier::TelegramNotifier;
use mts_common::state::StateManager;

pub struct MonitorEngine {
    scraper: Arc<AmazonScraper>,
    state_manager: Arc<StateManager>,
    notifier: TelegramNotifier,
    brand_filter: String,
}

impl MonitorEngine {
    pub fn new(
        scraper: Arc<AmazonScraper>,
        state_manager: Arc<StateManager>,
        notifier: TelegramNotifier,
        brand_filter: String,
    ) -> Self {
        Self {
            scraper,
            state_manager,
            notifier,
            brand_filter,
        }
    }

    pub async fn run_check(&self) -> Result<CheckOutcome> {
        let scrape_result = match self.scraper.scrape_search_page().await {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("{e:#}");
                tracing::error!("Scrape failed: {msg}");
                return Ok(CheckOutcome::ScrapeError(msg));
            }
        };

        info!(
            "Scraped {} results, huawei_sponsored={}",
            scrape_result.results.len(),
            scrape_result.huawei_sponsored_found
        );

        if scrape_result.results.is_empty() {
            tracing::warn!(
                "Scrape returned 0 results — Amazon may be blocking silently. Skipping state update."
            );
            return Ok(CheckOutcome::ScrapeError(
                "0 results returned — possible silent block".to_string(),
            ));
        }

        let brand_lower = self.brand_filter.to_lowercase();
        let sample_title = scrape_result
            .results
            .iter()
            .find(|r| {
                r.is_sponsored
                    && r.title.to_lowercase().contains(&brand_lower)
            })
            .map(|r| r.title.clone())
            .unwrap_or_default();

        let new_state = MonitorState {
            huawei_ad_visible: scrape_result.huawei_sponsored_found,
            huawei_positions: scrape_result.huawei_sponsored_positions.clone(),
            total_results_scraped: scrape_result.results.len(),
            updated_at: Utc::now(),
        };

        let previous = self.state_manager.load()?;

        let outcome = match previous {
            None => {
                info!("First run — saving baseline state, no alert sent");
                self.state_manager.save(&new_state)?;
                CheckOutcome::FirstRun
            }
            Some(prev) => {
                let outcome = if !prev.huawei_ad_visible && new_state.huawei_ad_visible {
                    info!(
                        "Huawei ad appeared at positions: {:?}",
                        new_state.huawei_positions
                    );
                    let huawei_page_positions: Vec<(u32, usize)> = scrape_result
                        .results
                        .iter()
                        .filter(|r| r.is_sponsored && r.title.to_lowercase().contains(&brand_lower))
                        .map(|r| (r.page, r.position_in_page))
                        .collect();
                    let all_sponsored: Vec<(u32, usize, String)> = scrape_result
                        .results
                        .iter()
                        .filter(|r| r.is_sponsored)
                        .map(|r| (r.page, r.position_in_page, r.title.clone()))
                        .collect();
                    self.notifier
                        .send_ad_appeared(&huawei_page_positions, &sample_title, &all_sponsored)
                        .await?;
                    CheckOutcome::AdAppeared {
                        positions: new_state.huawei_positions.clone(),
                        sample_title,
                    }
                } else if prev.huawei_ad_visible && !new_state.huawei_ad_visible {
                    info!("Huawei ad disappeared");
                    self.notifier.send_ad_disappeared().await?;
                    CheckOutcome::AdDisappeared
                } else {
                    info!(
                        "No change: huawei_ad_visible={}, positions={:?}",
                        new_state.huawei_ad_visible, new_state.huawei_positions
                    );
                    CheckOutcome::NoChange
                };

                self.state_manager.save(&new_state)?;
                outcome
            }
        };

        Ok(outcome)
    }
}
