use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::amazon_scraper::AmazonScraper;
use crate::config::{MarketplaceConfig, TelegramConfig};
use mts_common::models::KeywordState;
use mts_common::notifier::{SponsoredEntry, TelegramNotifier};
use mts_common::state::StateManager;

pub struct MonitorEngine {
    scraper: Arc<AmazonScraper>,
    state_manager: Arc<StateManager>,
    http_client: reqwest::Client,
    telegram_configs: Arc<Vec<TelegramConfig>>,
    brand_filter: String,
    db_pool: Option<PgPool>,
    shutdown: CancellationToken,
}

impl MonitorEngine {
    pub fn new(
        scraper: Arc<AmazonScraper>,
        state_manager: Arc<StateManager>,
        http_client: reqwest::Client,
        telegram_configs: Arc<Vec<TelegramConfig>>,
        brand_filter: String,
        db_pool: Option<PgPool>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            scraper,
            state_manager,
            http_client,
            telegram_configs,
            brand_filter,
            db_pool,
            shutdown,
        }
    }

    /// Run a full sweep for a single marketplace (all its keywords).
    pub async fn run_check_marketplace(&self, marketplace: &MarketplaceConfig) -> Result<()> {
        // Launch ONE browser for the entire marketplace sweep
        let (mut browser, handle) = self.scraper.launch_browser().await?;

        let mut state = self.state_manager.load()?.unwrap_or_default();
        let brand_lower = self.brand_filter.to_lowercase();

        for keyword in &marketplace.keywords {
            if self.shutdown.is_cancelled() {
                info!("[{}] Shutdown requested, stopping sweep.", marketplace.code);
                break;
            }

            info!("[{}] Scraping keyword: '{}'", marketplace.code, keyword);

            let scrape_result = match self
                .scraper
                .scrape_all_pages_with_browser(
                    &browser,
                    keyword,
                    &marketplace.url,
                    &marketplace.accept_language,
                    &marketplace.languages,
                )
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                        "[{}] Scrape failed for keyword '{}': {:#}",
                        marketplace.code,
                        keyword,
                        e
                    );
                    continue;
                }
            };

            if scrape_result.results.is_empty() {
                warn!(
                    "[{}] Keyword '{}': 0 results — possible block, skipping state update",
                    marketplace.code, keyword
                );
                continue;
            }

            info!(
                "[{}] Keyword '{}': scraped {} results",
                marketplace.code,
                keyword,
                scrape_result.results.len()
            );

            // Persist to Postgres if pool is available
            if let Some(ref pool) = self.db_pool {
                match mts_common::db::insert_scrape_run(
                    pool,
                    &marketplace.code,
                    keyword,
                    &scrape_result,
                    &self.brand_filter,
                )
                .await
                {
                    Ok(run_id) => {
                        info!(
                            "[{}] Keyword '{}': saved to DB (run_id={})",
                            marketplace.code, keyword, run_id
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[{}] Keyword '{}': DB insert failed: {:#}",
                            marketplace.code, keyword, e
                        );
                    }
                }
            }

            // Use marketplace-scoped key for state: "FR:keyword"
            let state_key = format!("{}:{}", marketplace.code, keyword);

            let prev_ks = state
                .keywords
                .get(state_key.as_str())
                .cloned()
                .unwrap_or_default();

            // Determine brand visibility
            let brand_ad_visible = scrape_result.results.iter().any(|r| {
                r.is_sponsored
                    && (r.title.to_lowercase().contains(&brand_lower)
                        || r.brand
                            .as_ref()
                            .is_some_and(|b| b.to_lowercase().contains(&brand_lower)))
            });

            let brand_positions: Vec<(u32, usize, Option<mts_common::models::PlacementType>)> =
                scrape_result
                    .results
                    .iter()
                    .filter(|r| {
                        r.is_sponsored
                            && (r.title.to_lowercase().contains(&brand_lower)
                                || r.brand
                                    .as_ref()
                                    .is_some_and(|b| b.to_lowercase().contains(&brand_lower)))
                    })
                    .map(|r| (r.page, r.position_in_page, r.placement_type.clone()))
                    .collect();

            let now = Utc::now();
            let last_changed = if brand_ad_visible != prev_ks.brand_ad_visible {
                Some(now)
            } else {
                prev_ks.last_changed
            };

            // Build notifiers for all telegram targets
            let search_url = AmazonScraper::build_search_url(&marketplace.url, keyword, 1);
            let notifiers: Vec<TelegramNotifier> = self
                .telegram_configs
                .iter()
                .filter_map(|tg| {
                    match TelegramNotifier::new(
                        tg,
                        self.http_client.clone(),
                        keyword.clone(),
                        search_url.clone(),
                    ) {
                        Ok(n) => Some(n),
                        Err(e) => {
                            warn!(
                                "[{}] Failed to create notifier for chat_id={}: {:#}",
                                marketplace.code, tg.chat_id, e
                            );
                            None
                        }
                    }
                })
                .collect();

            if notifiers.is_empty() {
                warn!(
                    "[{}] No working notifiers — skipping notifications for '{}'",
                    marketplace.code, keyword
                );
                let new_ks = KeywordState {
                    brand_ad_visible,
                    brand_positions,
                    last_changed,
                    last_checked: Some(now),
                    last_results: scrape_result.results,
                };
                state.keywords.insert(state_key, new_ks);
                continue;
            }

            if !prev_ks.brand_ad_visible && brand_ad_visible {
                info!(
                    "[{}] Keyword '{}': brand ad APPEARED",
                    marketplace.code, keyword
                );
                let sample_title = scrape_result
                    .results
                    .iter()
                    .find(|r| {
                        r.is_sponsored
                            && (r.title.to_lowercase().contains(&brand_lower)
                                || r.brand
                                    .as_ref()
                                    .is_some_and(|b| b.to_lowercase().contains(&brand_lower)))
                    })
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

                for notifier in &notifiers {
                    if let Err(e) = notifier
                        .send_ad_appeared(&brand_positions, &sample_title, &all_sponsored)
                        .await
                    {
                        warn!(
                            "[{}] Failed to send ad appeared notification for '{}': {:#}",
                            marketplace.code, keyword, e
                        );
                    }
                }
            } else if prev_ks.brand_ad_visible && !brand_ad_visible {
                info!(
                    "[{}] Keyword '{}': brand ad DISAPPEARED",
                    marketplace.code, keyword
                );
                for notifier in &notifiers {
                    if let Err(e) = notifier.send_ad_disappeared().await {
                        warn!(
                            "[{}] Failed to send ad disappeared notification for '{}': {:#}",
                            marketplace.code, keyword, e
                        );
                    }
                }
            } else {
                info!(
                    "[{}] Keyword '{}': no change (brand_visible={})",
                    marketplace.code, keyword, brand_ad_visible
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
            state.keywords.insert(state_key, new_ks);
        }

        // Close browser with timeout to prevent hanging on shutdown
        match tokio::time::timeout(Duration::from_secs(5), browser.close()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => tracing::warn!("[{}] Browser close error: {:#}", marketplace.code, e),
            Err(_) => tracing::warn!("[{}] Browser close timed out after 5s", marketplace.code),
        }
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

        // Save state ONCE after the full marketplace sweep
        self.state_manager.save(&state)?;
        info!("[{}] Sweep complete. State saved.", marketplace.code);

        Ok(())
    }
}
