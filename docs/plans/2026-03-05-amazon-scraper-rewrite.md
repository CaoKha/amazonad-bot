# Amazon Scraper Rewrite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the Amazon Advertising API-based monitoring daemon with a web scraper that detects Huawei smartwatch sponsored ads on amazon.fr search results for "montre connectee".

**Architecture:** The daemon scrapes `https://www.amazon.fr/s?k=montre+connectee` on a configurable interval, parses the HTML response using the `scraper` crate to find sponsored results containing "huawei" in the title, compares against persisted state in `state.json`, and sends Telegram alerts when the ad appears or disappears.

**Tech Stack:** Rust, reqwest (HTTP), scraper 0.22 (HTML parsing), rand 0.8 (User-Agent rotation), tokio (async runtime), serde/serde_json (serialization), config (TOML config), clap (CLI), tracing (logging), chrono (timestamps), mockito (test HTTP mocking)

---

## Task 1: Update Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Edit Cargo.toml**

Replace the `[dependencies]` section. Remove `flate2 = "1"`, add `scraper = "0.22"` and `rand = "0.8"`:

```toml
[package]
name = "monitoring-the-situation"
version = "0.1.0"
edition = "2024"

[dependencies]
reqwest = { version = "0.13", features = ["json", "gzip", "form"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
config = { version = "0.15", features = ["toml"] }
dotenvy = "0.15"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
scraper = "0.22"
rand = "0.8"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
mockito = "1"
tokio-test = "0.4"
```

**Step 2: Verify it compiles (dependencies only)**

```bash
cargo fetch
```

Expected: Downloads scraper and rand crates, no errors.

---

## Task 2: Delete obsolete files

**Files:**
- Delete: `src/amazon_ads/` (entire directory)
- Delete: `tests/auth_tests.rs`

**Step 1: Delete the amazon_ads module directory**

```bash
rm -rf src/amazon_ads/
```

**Step 2: Delete the auth tests file**

```bash
rm tests/auth_tests.rs
```

**Step 3: Verify deletion**

```bash
ls src/ && ls tests/
```

Expected: `src/` has no `amazon_ads/` dir. `tests/` has no `auth_tests.rs`.

---

## Task 3: Rewrite src/models.rs

**Files:**
- Modify: `src/models.rs`

**Step 1: Replace entire file content**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single product from amazon.fr search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub asin: String,
    pub title: String,
    pub position: usize,
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
```

**Step 2: Verify no compile errors on models alone**

```bash
cargo check 2>&1 | head -30
```

Expected: Errors only about missing modules (amazon_ads, etc.) — not about models.rs itself.

---

## Task 4: Rewrite src/config.rs

**Files:**
- Modify: `src/config.rs`

**Step 1: Replace entire file content**

```rust
use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub scraper: ScraperConfig,
    pub telegram: TelegramConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize)]
pub struct ScraperConfig {
    pub keyword: String,
    pub marketplace_url: String,
    pub brand_filter: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub chat_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct MonitoringConfig {
    pub interval_minutes: u64,
}

pub fn load_config() -> Result<AppConfig> {
    dotenvy::dotenv().ok();

    let config = config::Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("APP").separator("__"))
        .build()
        .context("Failed to build configuration")?;

    let app_config: AppConfig = config.try_deserialize().context(
        "Failed to deserialize configuration. Check config.toml and environment variables.",
    )?;

    // Validation
    if app_config.telegram.chat_id == 0 {
        bail!("telegram.chat_id must be set (got 0). See README for how to find your chat ID.");
    }
    if app_config.scraper.keyword.is_empty() {
        bail!("scraper.keyword must not be empty.");
    }
    if app_config.scraper.marketplace_url.is_empty() {
        bail!("scraper.marketplace_url must not be empty.");
    }
    if app_config.scraper.brand_filter.is_empty() {
        bail!("scraper.brand_filter must not be empty.");
    }
    if app_config.monitoring.interval_minutes < 5 {
        bail!(
            "monitoring.interval_minutes must be at least 5 (got {})",
            app_config.monitoring.interval_minutes
        );
    }

    Ok(app_config)
}
```

---

## Task 5: Create src/amazon_scraper.rs

**Files:**
- Create: `src/amazon_scraper.rs`

**Step 1: Write the full scraper module**

```rust
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rand::seq::SliceRandom;
use scraper::{Html, Selector};
use tracing::{debug, warn};

use crate::config::ScraperConfig;
use crate::models::{ScrapeResult, SearchResult};

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:123.0) Gecko/20100101 Firefox/123.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_3_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3.1 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36 Edg/121.0.0.0",
];

pub struct AmazonScraper {
    client: reqwest::Client,
    config: Arc<ScraperConfig>,
}

impl AmazonScraper {
    pub fn new(config: Arc<ScraperConfig>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client, config })
    }

    pub async fn scrape_search_page(&self) -> Result<ScrapeResult> {
        let url = Self::build_search_url(&self.config.marketplace_url, &self.config.keyword);
        let user_agent = Self::random_user_agent();

        debug!("Scraping URL: {url} with UA: {user_agent}");

        let response = self
            .client
            .get(&url)
            .header("User-Agent", user_agent)
            .header("Accept-Language", "fr-FR,fr;q=0.9,en;q=0.8")
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
            .context("HTTP request to amazon.fr failed")?;

        let status = response.status();
        if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
            bail!("Amazon returned 503 Service Unavailable — likely rate-limited or blocked");
        }
        if !status.is_success() {
            bail!("Amazon returned unexpected status: {status}");
        }

        let html = response
            .text()
            .await
            .context("Failed to read response body")?;

        let lower = html.to_lowercase();
        if lower.contains("captcha") || lower.contains("robot") || lower.contains("automated") {
            bail!("CAPTCHA or bot-detection page detected — Amazon is blocking the scraper");
        }

        let result = Self::parse_results(&html, &self.config.brand_filter);
        debug!(
            "Scraped {} results, huawei_sponsored_found={}",
            result.results.len(),
            result.huawei_sponsored_found
        );

        Ok(result)
    }

    /// Parse HTML and extract search results. Pure function — no I/O.
    pub fn parse_results(html: &str, brand_filter: &str) -> ScrapeResult {
        let document = Html::parse_document(html);
        let brand_lower = brand_filter.to_lowercase();

        // Selectors — these are valid CSS so unwrap() is safe here
        let result_sel =
            Selector::parse(r#"div[data-component-type="s-search-result"]"#).unwrap();
        let sponsored_sel =
            Selector::parse(r#"div[data-component-type="sp-sponsored-result"]"#).unwrap();
        let adholder_sel = Selector::parse(".AdHolder").unwrap();
        let h2_sel = Selector::parse("h2").unwrap();

        // Collect all sponsored ASINs for fast lookup
        let sponsored_asins: std::collections::HashSet<String> = document
            .select(&sponsored_sel)
            .filter_map(|el| el.value().attr("data-asin").map(String::from))
            .collect();

        let adholder_asins: std::collections::HashSet<String> = document
            .select(&adholder_sel)
            .filter_map(|el| el.value().attr("data-asin").map(String::from))
            .collect();

        let mut results = Vec::new();
        let mut position = 0usize;

        for element in document.select(&result_sel) {
            let asin = match element.value().attr("data-asin") {
                Some(a) if !a.is_empty() => a.to_string(),
                _ => continue,
            };

            position += 1;

            // Extract title from h2 text
            let title = element
                .select(&h2_sel)
                .next()
                .map(|h| h.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Check sponsorship: ASIN in sponsored set, AdHolder set, or text contains "Sponsorisé"/"Sponsored"
            let text_content = element.text().collect::<String>();
            let is_sponsored = sponsored_asins.contains(&asin)
                || adholder_asins.contains(&asin)
                || text_content.contains("Sponsorisé")
                || text_content.contains("Sponsored");

            results.push(SearchResult {
                asin,
                title,
                position,
                is_sponsored,
            });
        }

        // Find Huawei sponsored results
        let huawei_sponsored: Vec<&SearchResult> = results
            .iter()
            .filter(|r| r.is_sponsored && r.title.to_lowercase().contains(&brand_lower))
            .collect();

        let huawei_sponsored_found = !huawei_sponsored.is_empty();
        let huawei_sponsored_positions: Vec<usize> =
            huawei_sponsored.iter().map(|r| r.position).collect();

        if huawei_sponsored_found {
            warn!(
                "Huawei sponsored ad found at position(s): {:?}",
                huawei_sponsored_positions
            );
        }

        ScrapeResult {
            results,
            huawei_sponsored_found,
            huawei_sponsored_positions,
            scraped_at: Utc::now(),
        }
    }

    fn random_user_agent() -> &'static str {
        let mut rng = rand::thread_rng();
        USER_AGENTS.choose(&mut rng).unwrap_or(&USER_AGENTS[0])
    }

    pub fn build_search_url(base: &str, keyword: &str) -> String {
        let encoded = keyword.replace(' ', "+");
        format!("{}/s?k={}", base.trim_end_matches('/'), encoded)
    }
}
```

**Step 2: Quick compile check**

```bash
cargo check 2>&1 | grep "amazon_scraper\|error\[" | head -20
```

---

## Task 6: Rewrite src/monitor.rs

**Files:**
- Modify: `src/monitor.rs`

**Step 1: Replace entire file content**

```rust
use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::amazon_scraper::AmazonScraper;
use crate::models::{CheckOutcome, MonitorState};
use crate::notifier::TelegramNotifier;
use crate::state::StateManager;

pub struct MonitorEngine {
    scraper: AmazonScraper,
    state_manager: StateManager,
    notifier: TelegramNotifier,
}

impl MonitorEngine {
    pub fn new(
        scraper: AmazonScraper,
        state_manager: StateManager,
        notifier: TelegramNotifier,
    ) -> Self {
        Self {
            scraper,
            state_manager,
            notifier,
        }
    }

    pub async fn run_check(&self) -> Result<CheckOutcome> {
        // Step 1: Scrape amazon.fr
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

        // Step 2: Build new state
        let new_state = MonitorState {
            huawei_ad_visible: scrape_result.huawei_sponsored_found,
            huawei_positions: scrape_result.huawei_sponsored_positions.clone(),
            total_results_scraped: scrape_result.results.len(),
            updated_at: Utc::now(),
        };

        // Step 3: Load previous state
        let previous = self.state_manager.load()?;

        // Step 4: Compare and act
        let outcome = match previous {
            None => {
                info!("First run — saving baseline state, no alert sent");
                self.state_manager.save(&new_state)?;
                CheckOutcome::FirstRun
            }
            Some(prev) => {
                let outcome = if !prev.huawei_ad_visible && new_state.huawei_ad_visible {
                    // Ad appeared
                    let sample_title = scrape_result
                        .results
                        .iter()
                        .find(|r| {
                            r.is_sponsored
                                && r.title.to_lowercase().contains(
                                    // We don't have brand_filter here, but positions match
                                    &new_state
                                        .huawei_positions
                                        .first()
                                        .map(|_| "huawei")
                                        .unwrap_or("huawei"),
                                )
                        })
                        .map(|r| r.title.clone())
                        .unwrap_or_default();

                    info!(
                        "Huawei ad appeared at positions: {:?}",
                        new_state.huawei_positions
                    );
                    self.notifier
                        .send_ad_appeared(&new_state.huawei_positions, &sample_title)
                        .await?;
                    CheckOutcome::AdAppeared {
                        positions: new_state.huawei_positions.clone(),
                        sample_title,
                    }
                } else if prev.huawei_ad_visible && !new_state.huawei_ad_visible {
                    // Ad disappeared
                    info!("Huawei ad disappeared");
                    self.notifier.send_ad_disappeared().await?;
                    CheckOutcome::AdDisappeared
                } else {
                    // No change
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
```

**Note on sample_title extraction:** The monitor doesn't have access to `brand_filter` directly. A cleaner approach is to pass the first Huawei result's title from `scrape_result`. The implementation above uses a simple heuristic — if positions are non-empty, find the first sponsored result at that position.

**Revised Step 1 — cleaner monitor.rs:**

```rust
use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::amazon_scraper::AmazonScraper;
use crate::models::{CheckOutcome, MonitorState};
use crate::notifier::TelegramNotifier;
use crate::state::StateManager;

pub struct MonitorEngine {
    scraper: AmazonScraper,
    state_manager: StateManager,
    notifier: TelegramNotifier,
}

impl MonitorEngine {
    pub fn new(
        scraper: AmazonScraper,
        state_manager: StateManager,
        notifier: TelegramNotifier,
    ) -> Self {
        Self {
            scraper,
            state_manager,
            notifier,
        }
    }

    pub async fn run_check(&self) -> Result<CheckOutcome> {
        // Step 1: Scrape amazon.fr
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

        // Step 2: Find sample title for notification
        let sample_title = scrape_result
            .results
            .iter()
            .find(|r| {
                r.is_sponsored
                    && scrape_result
                        .huawei_sponsored_positions
                        .contains(&r.position)
            })
            .map(|r| r.title.clone())
            .unwrap_or_default();

        // Step 3: Build new state
        let new_state = MonitorState {
            huawei_ad_visible: scrape_result.huawei_sponsored_found,
            huawei_positions: scrape_result.huawei_sponsored_positions.clone(),
            total_results_scraped: scrape_result.results.len(),
            updated_at: Utc::now(),
        };

        // Step 4: Load previous state
        let previous = self.state_manager.load()?;

        // Step 5: Compare and act
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
                    self.notifier
                        .send_ad_appeared(&new_state.huawei_positions, &sample_title)
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
```

---

## Task 7: Update src/notifier.rs

**Files:**
- Modify: `src/notifier.rs`

**Step 1: Replace entire file content**

```rust
use anyhow::{bail, Context, Result};
use tracing::warn;

use crate::config::TelegramConfig;

pub struct TelegramNotifier {
    client: reqwest::Client,
    bot_token: String,
    chat_id: i64,
}

impl TelegramNotifier {
    pub fn new(config: &TelegramConfig) -> Result<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .context("TELEGRAM_BOT_TOKEN environment variable not set")?;

        if bot_token.is_empty() {
            bail!("TELEGRAM_BOT_TOKEN is empty");
        }

        Ok(Self {
            client: reqwest::Client::new(),
            bot_token,
            chat_id: config.chat_id,
        })
    }

    /// Send alert when Huawei sponsored ad appears on amazon.fr.
    pub async fn send_ad_appeared(&self, positions: &[usize], sample_title: &str) -> Result<()> {
        let pos_str = positions
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let message = format!(
            "\u{1f50d} <b>Huawei ad detected on amazon.fr!</b>\n\
             Keyword: <code>montre connectee</code>\n\
             Position(s): <b>{pos_str}</b>\n\
             Title: {sample_title}"
        );

        self.send_message(&message).await
    }

    /// Send notification when Huawei sponsored ad disappears from amazon.fr.
    pub async fn send_ad_disappeared(&self) -> Result<()> {
        let message =
            "\u{1f4ed} Huawei ad no longer visible on amazon.fr for \u{2018}montre connectee\u{2019}"
                .to_string();

        self.send_message(&message).await
    }

    /// Send a test message to verify connectivity (used in dry-run mode).
    pub async fn send_test_message(&self) -> Result<()> {
        self.send_message("\u{1f9b7} monitoring-the-situation connected successfully")
            .await
    }

    async fn send_message(&self, text: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
        });

        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Telegram request failed: {e}. Skipping notification.");
                return Ok(());
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            warn!(
                "Telegram API returned {status}: {body_text}. Skipping notification."
            );
            return Ok(());
        }

        Ok(())
    }
}
```

---

## Task 8: Update src/lib.rs

**Files:**
- Modify: `src/lib.rs`

**Step 1: Replace entire file content**

```rust
pub mod amazon_scraper;
pub mod config;
pub mod models;
pub mod monitor;
pub mod notifier;
pub mod state;
```

---

## Task 9: Rewrite src/main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace entire file content**

```rust
use monitoring_the_situation::{amazon_scraper, config, monitor, notifier, state};

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(
    name = "monitoring-the-situation",
    about = "Monitors amazon.fr for Huawei smartwatch sponsored ads"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the monitoring daemon (polls every interval_minutes)
    Run,
    /// Run a single check immediately and exit
    CheckNow,
    /// Validate config and send a test Telegram message
    DryRun,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("monitoring_the_situation=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Run => cmd_run().await,
        Commands::CheckNow => cmd_check_now().await,
        Commands::DryRun => cmd_dry_run().await,
    }
}

async fn cmd_run() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    let config = Arc::new(app_config);
    let scraper =
        amazon_scraper::AmazonScraper::new(Arc::new(config.scraper.clone()))?;
    let state_manager = state::StateManager::new(PathBuf::from("state.json"));
    let notifier = notifier::TelegramNotifier::new(&config.telegram)?;
    let engine = monitor::MonitorEngine::new(scraper, state_manager, notifier);

    info!(
        "Starting monitoring loop (interval: {} min)",
        config.monitoring.interval_minutes
    );

    let mut interval =
        tokio::time::interval(Duration::from_secs(config.monitoring.interval_minutes * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match engine.run_check().await {
                    Ok(outcome) => info!("Check complete: {:?}", outcome),
                    Err(e) => tracing::error!("Check failed: {e:#}"),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Exiting.");
                break;
            }
        }
    }

    Ok(())
}

async fn cmd_check_now() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    let config = Arc::new(app_config);
    let scraper =
        amazon_scraper::AmazonScraper::new(Arc::new(config.scraper.clone()))?;
    let state_manager = state::StateManager::new(PathBuf::from("state.json"));
    let notifier = notifier::TelegramNotifier::new(&config.telegram)?;
    let engine = monitor::MonitorEngine::new(scraper, state_manager, notifier);

    let outcome = engine.run_check().await?;
    info!("Check complete: {:?}", outcome);

    Ok(())
}

async fn cmd_dry_run() -> anyhow::Result<()> {
    // Step 1: Load config
    let app_config = config::load_config()?;
    info!("Config loaded: OK");

    let config = Arc::new(app_config);

    // Step 2: Test Telegram connectivity
    let notifier = notifier::TelegramNotifier::new(&config.telegram)?;
    notifier.send_test_message().await?;
    info!("Telegram: OK");

    info!("\nAll checks passed. Ready to run: cargo run -- run");

    Ok(())
}
```

**Note:** `ScraperConfig` needs `Clone` derived. Add `#[derive(Debug, Deserialize, Clone)]` to `ScraperConfig` in config.rs.

---

## Task 10: Update config files

**Files:**
- Modify: `.env.example`
- Modify: `config.toml.example`

**Step 1: Rewrite .env.example**

```
# Telegram
TELEGRAM_BOT_TOKEN=your-bot-token-here
```

**Step 2: Rewrite config.toml.example**

```toml
[scraper]
keyword = "montre connectee"
marketplace_url = "https://www.amazon.fr"
brand_filter = "huawei"

[telegram]
chat_id = 0  # Your Telegram chat ID

[monitoring]
interval_minutes = 30
```

---

## Task 11: Rewrite test files

**Files:**
- Modify: `tests/config_tests.rs`
- Modify: `tests/models_tests.rs`
- Modify: `tests/monitor_tests.rs`
- Modify: `tests/notifier_tests.rs`
- Modify: `tests/state_tests.rs` (minimal changes)
- Create: `tests/scraper_tests.rs`

### tests/config_tests.rs

```rust
use std::sync::Mutex;

use monitoring_the_situation::config::load_config;

/// Serialize all config tests — they share process-level env vars.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn set_valid_config_env() {
    unsafe {
        std::env::set_var("APP__SCRAPER__KEYWORD", "montre connectee");
        std::env::set_var("APP__SCRAPER__MARKETPLACE_URL", "https://www.amazon.fr");
        std::env::set_var("APP__SCRAPER__BRAND_FILTER", "huawei");
        std::env::set_var("APP__TELEGRAM__CHAT_ID", "123456789");
        std::env::set_var("APP__MONITORING__INTERVAL_MINUTES", "30");
    }
}

fn clear_config_env() {
    unsafe {
        std::env::remove_var("APP__SCRAPER__KEYWORD");
        std::env::remove_var("APP__SCRAPER__MARKETPLACE_URL");
        std::env::remove_var("APP__SCRAPER__BRAND_FILTER");
        std::env::remove_var("APP__TELEGRAM__CHAT_ID");
        std::env::remove_var("APP__MONITORING__INTERVAL_MINUTES");
    }
}

#[test]
fn valid_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();

    let config = load_config().expect("valid config should load successfully");

    assert_eq!(config.scraper.keyword, "montre connectee");
    assert_eq!(config.scraper.marketplace_url, "https://www.amazon.fr");
    assert_eq!(config.scraper.brand_filter, "huawei");
    assert_eq!(config.telegram.chat_id, 123456789);
    assert_eq!(config.monitoring.interval_minutes, 30);

    clear_config_env();
}

#[test]
fn invalid_chat_id() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__TELEGRAM__CHAT_ID", "0");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("chat_id"),
        "Error should mention chat_id, got: {err_msg}"
    );

    clear_config_env();
}

#[test]
fn empty_keyword() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__KEYWORD", "");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("keyword"),
        "Error should mention keyword, got: {err_msg}"
    );

    clear_config_env();
}

#[test]
fn interval_too_low() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__MONITORING__INTERVAL_MINUTES", "4");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("interval_minutes"),
        "Error should mention interval_minutes, got: {err_msg}"
    );

    clear_config_env();
}

#[test]
fn interval_at_minimum_is_valid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__MONITORING__INTERVAL_MINUTES", "5");
    }

    let result = load_config();
    assert!(result.is_ok(), "interval=5 should be valid");

    clear_config_env();
}
```

### tests/models_tests.rs

```rust
use monitoring_the_situation::models::{MonitorState, SearchResult};

#[test]
fn search_result_serialization() {
    let result = SearchResult {
        asin: "B0ABCDEF12".to_string(),
        title: "Huawei Watch GT 4".to_string(),
        position: 3,
        is_sponsored: true,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: SearchResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.asin, "B0ABCDEF12");
    assert_eq!(deserialized.title, "Huawei Watch GT 4");
    assert_eq!(deserialized.position, 3);
    assert!(deserialized.is_sponsored);
}

#[test]
fn monitor_state_round_trip() {
    let state = MonitorState {
        huawei_ad_visible: true,
        huawei_positions: vec![1, 3],
        total_results_scraped: 48,
        updated_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&state).unwrap();
    let deserialized: MonitorState = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.huawei_ad_visible, state.huawei_ad_visible);
    assert_eq!(deserialized.huawei_positions, state.huawei_positions);
    assert_eq!(
        deserialized.total_results_scraped,
        state.total_results_scraped
    );
    assert_eq!(deserialized.updated_at, state.updated_at);
}

#[test]
fn monitor_state_no_ad() {
    let state = MonitorState {
        huawei_ad_visible: false,
        huawei_positions: vec![],
        total_results_scraped: 24,
        updated_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&state).unwrap();
    let deserialized: MonitorState = serde_json::from_str(&json).unwrap();

    assert!(!deserialized.huawei_ad_visible);
    assert!(deserialized.huawei_positions.is_empty());
}
```

### tests/monitor_tests.rs

```rust
// ---------------------------------------------------------------------------
// Monitor business-logic tests
//
// Tests the state-comparison logic: ad appeared, disappeared, no change, first run.
// Uses MonitorState directly — no HTTP mocking needed.
// ---------------------------------------------------------------------------

use monitoring_the_situation::models::{CheckOutcome, MonitorState};

fn make_state(huawei_visible: bool, positions: Vec<usize>) -> MonitorState {
    MonitorState {
        huawei_ad_visible: huawei_visible,
        huawei_positions: positions,
        total_results_scraped: 48,
        updated_at: chrono::Utc::now(),
    }
}

/// Mirrors the comparison logic in MonitorEngine::run_check.
fn determine_outcome(prev: &MonitorState, current: &MonitorState) -> &'static str {
    if !prev.huawei_ad_visible && current.huawei_ad_visible {
        "appeared"
    } else if prev.huawei_ad_visible && !current.huawei_ad_visible {
        "disappeared"
    } else {
        "no_change"
    }
}

#[test]
fn ad_appeared_when_was_absent() {
    let prev = make_state(false, vec![]);
    let current = make_state(true, vec![2]);
    assert_eq!(determine_outcome(&prev, &current), "appeared");
}

#[test]
fn ad_disappeared_when_was_present() {
    let prev = make_state(true, vec![1]);
    let current = make_state(false, vec![]);
    assert_eq!(determine_outcome(&prev, &current), "disappeared");
}

#[test]
fn no_change_when_ad_stays_visible() {
    let prev = make_state(true, vec![1]);
    let current = make_state(true, vec![1]);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn no_change_when_ad_stays_absent() {
    let prev = make_state(false, vec![]);
    let current = make_state(false, vec![]);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn no_change_when_position_changes_but_still_visible() {
    // Position changed from 1 to 3, but still visible — no state change
    let prev = make_state(true, vec![1]);
    let current = make_state(true, vec![3]);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn check_outcome_variants_are_debug() {
    // Ensure all variants can be formatted with {:?}
    let outcomes: Vec<CheckOutcome> = vec![
        CheckOutcome::AdAppeared {
            positions: vec![1],
            sample_title: "Huawei Watch".to_string(),
        },
        CheckOutcome::AdDisappeared,
        CheckOutcome::NoChange,
        CheckOutcome::ScrapeError("test error".to_string()),
        CheckOutcome::FirstRun,
    ];
    for o in &outcomes {
        let _ = format!("{:?}", o);
    }
}
```

### tests/notifier_tests.rs

```rust
use std::sync::Mutex;

use monitoring_the_situation::config::TelegramConfig;
use monitoring_the_situation::notifier::TelegramNotifier;

/// Serialize notifier tests — they share TELEGRAM_BOT_TOKEN env var.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn new_fails_without_bot_token() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    let config = TelegramConfig { chat_id: 123456789 };
    let result = TelegramNotifier::new(&config);

    assert!(result.is_err());
    let err_msg = format!("{:#}", result.err().unwrap());
    assert!(
        err_msg.contains("TELEGRAM_BOT_TOKEN"),
        "Error should mention TELEGRAM_BOT_TOKEN, got: {err_msg}"
    );
}

#[test]
fn new_succeeds_with_valid_token() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::set_var(
            "TELEGRAM_BOT_TOKEN",
            "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11",
        );
    }

    let config = TelegramConfig { chat_id: 123456789 };
    let result = TelegramNotifier::new(&config);
    assert!(result.is_ok(), "Should succeed with valid token");

    unsafe {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }
}

#[test]
fn new_fails_with_empty_token() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "");
    }

    let config = TelegramConfig { chat_id: 123456789 };
    let result = TelegramNotifier::new(&config);

    assert!(result.is_err());
    let err_msg = format!("{:#}", result.err().unwrap());
    assert!(
        err_msg.contains("empty"),
        "Error should mention empty token, got: {err_msg}"
    );

    unsafe {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }
}
```

### tests/state_tests.rs

Update `sample_state()` to use new `MonitorState` fields:

```rust
use std::path::PathBuf;

use chrono::Utc;
use monitoring_the_situation::models::MonitorState;
use monitoring_the_situation::state::StateManager;

fn temp_state_path() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mts_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("state_{nanos}.json"))
}

fn sample_state() -> MonitorState {
    MonitorState {
        huawei_ad_visible: true,
        huawei_positions: vec![2, 5],
        total_results_scraped: 48,
        updated_at: Utc::now(),
    }
}

#[test]
fn round_trip() {
    let path = temp_state_path();
    let manager = StateManager::new(path.clone());
    let original = sample_state();

    manager.save(&original).unwrap();
    let loaded = manager.load().unwrap().expect("should load saved state");

    assert_eq!(loaded.huawei_ad_visible, original.huawei_ad_visible);
    assert_eq!(loaded.huawei_positions, original.huawei_positions);
    assert_eq!(loaded.total_results_scraped, original.total_results_scraped);
    assert_eq!(loaded.updated_at, original.updated_at);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn missing_file() {
    let path = PathBuf::from("/tmp/mts_nonexistent_state_99999.json");
    let _ = std::fs::remove_file(&path);
    let manager = StateManager::new(path);

    let result = manager.load().unwrap();
    assert!(result.is_none(), "missing file should return Ok(None)");
}

#[test]
fn corrupt_json() {
    let path = temp_state_path();
    std::fs::write(&path, "this is not json {{{").unwrap();

    let manager = StateManager::new(path.clone());
    let result = manager.load().unwrap();
    assert!(
        result.is_none(),
        "corrupt JSON should return Ok(None), not panic"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn atomic_tmp_cleanup() {
    let path = temp_state_path();
    let manager = StateManager::new(path.clone());
    let state = sample_state();

    manager.save(&state).unwrap();

    let tmp_path = path.with_extension("json.tmp");
    assert!(
        !tmp_path.exists(),
        "temp file should be cleaned up after atomic save"
    );

    let _ = std::fs::remove_file(&path);
}
```

### tests/scraper_tests.rs (NEW — most important)

```rust
use monitoring_the_situation::amazon_scraper::AmazonScraper;

// ── HTML fixtures ────────────────────────────────────────────────────────────

/// Minimal HTML with one sponsored Huawei result and one organic result.
const HTML_WITH_HUAWEI_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <div data-component-type="sp-sponsored-result" data-asin="B0HUAWEI01">
    <h2>Huawei Watch GT 4 Montre Connectée</h2>
    <span>Sponsorisé</span>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
</body>
</html>
"#;

/// HTML with no sponsored results at all.
const HTML_NO_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
<div data-component-type="s-search-result" data-asin="B0SAMSUNG1">
  <h2>Samsung Galaxy Watch 6</h2>
</div>
</body>
</html>
"#;

/// HTML with a sponsored result that is NOT Huawei.
const HTML_SPONSORED_NOT_HUAWEI: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <div data-component-type="sp-sponsored-result" data-asin="B0APPLE001">
    <h2>Apple Watch Series 9</h2>
    <span>Sponsorisé</span>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0SAMSUNG1">
  <h2>Samsung Galaxy Watch 6</h2>
</div>
</body>
</html>
"#;

/// HTML with Huawei in title but NOT sponsored.
const HTML_HUAWEI_ORGANIC: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <h2>Huawei Watch GT 4 Montre Connectée</h2>
</div>
</body>
</html>
"#;

/// HTML with multiple sponsored results including Huawei at different positions.
const HTML_MULTIPLE_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <div data-component-type="sp-sponsored-result" data-asin="B0APPLE001">
    <h2>Apple Watch Series 9</h2>
    <span>Sponsorisé</span>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0SAMSUNG1">
  <h2>Samsung Galaxy Watch 6</h2>
</div>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <div data-component-type="sp-sponsored-result" data-asin="B0HUAWEI01">
    <h2>Huawei Watch GT 4</h2>
    <span>Sponsorisé</span>
  </div>
</div>
</body>
</html>
"#;

/// Empty search results page.
const HTML_EMPTY_RESULTS: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div class="s-no-outline">No results found</div>
</body>
</html>
"#;

/// CAPTCHA page.
const HTML_CAPTCHA: &str = r#"
<!DOCTYPE html>
<html>
<body>
<h4>Enter the characters you see below</h4>
<p>Sorry, we just need to make sure you're not a robot.</p>
<form action="/errors/validateCaptcha">
</form>
</body>
</html>
"#;

/// HTML using "Sponsored" (English) instead of "Sponsorisé" (French).
const HTML_ENGLISH_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <h2>Huawei Watch GT 4</h2>
  <span>Sponsored</span>
</div>
</body>
</html>
"#;

/// HTML using AdHolder class for sponsored detection.
const HTML_ADHOLDER_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="AdHolder">
  <h2>Huawei Watch GT 4</h2>
</div>
</body>
</html>
"#;

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn detects_huawei_sponsored_result() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");

    assert!(result.huawei_sponsored_found, "Should detect Huawei sponsored ad");
    assert_eq!(result.huawei_sponsored_positions, vec![1]);
    assert_eq!(result.results.len(), 2);
}

#[test]
fn no_sponsored_results() {
    let result = AmazonScraper::parse_results(HTML_NO_SPONSORED, "huawei");

    assert!(!result.huawei_sponsored_found);
    assert!(result.huawei_sponsored_positions.is_empty());
    assert_eq!(result.results.len(), 2);
}

#[test]
fn sponsored_but_not_huawei() {
    let result = AmazonScraper::parse_results(HTML_SPONSORED_NOT_HUAWEI, "huawei");

    assert!(!result.huawei_sponsored_found, "Apple sponsored should not trigger Huawei detection");
    assert!(result.huawei_sponsored_positions.is_empty());
    // Apple result IS sponsored
    let apple = result.results.iter().find(|r| r.asin == "B0APPLE001").unwrap();
    assert!(apple.is_sponsored);
}

#[test]
fn huawei_organic_not_detected_as_sponsored() {
    let result = AmazonScraper::parse_results(HTML_HUAWEI_ORGANIC, "huawei");

    assert!(!result.huawei_sponsored_found, "Organic Huawei result should not trigger alert");
    assert!(result.huawei_sponsored_positions.is_empty());
    let huawei = result.results.iter().find(|r| r.asin == "B0HUAWEI01").unwrap();
    assert!(!huawei.is_sponsored);
}

#[test]
fn multiple_sponsored_huawei_at_position_3() {
    let result = AmazonScraper::parse_results(HTML_MULTIPLE_SPONSORED, "huawei");

    assert!(result.huawei_sponsored_found);
    assert_eq!(result.huawei_sponsored_positions, vec![3]);
    assert_eq!(result.results.len(), 3);
}

#[test]
fn empty_results_page() {
    let result = AmazonScraper::parse_results(HTML_EMPTY_RESULTS, "huawei");

    assert!(!result.huawei_sponsored_found);
    assert!(result.results.is_empty());
}

#[test]
fn brand_filter_case_insensitive() {
    // "HUAWEI" in brand_filter should match "Huawei" in title
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "HUAWEI");
    assert!(result.huawei_sponsored_found, "Brand filter should be case-insensitive");

    // "Huawei" in brand_filter should match "HUAWEI" in title
    let html_upper = HTML_WITH_HUAWEI_SPONSORED.replace("Huawei Watch GT 4", "HUAWEI WATCH GT 4");
    let result2 = AmazonScraper::parse_results(&html_upper, "huawei");
    assert!(result2.huawei_sponsored_found, "Title matching should be case-insensitive");
}

#[test]
fn english_sponsored_label_detected() {
    let result = AmazonScraper::parse_results(HTML_ENGLISH_SPONSORED, "huawei");
    assert!(result.huawei_sponsored_found, "English 'Sponsored' label should be detected");
}

#[test]
fn adholder_class_detected_as_sponsored() {
    let result = AmazonScraper::parse_results(HTML_ADHOLDER_SPONSORED, "huawei");
    // AdHolder class on the result div itself — need to check if our selector picks it up
    // The AdHolder selector looks for .AdHolder with data-asin attribute
    // In this HTML the result div has class="AdHolder" but the selector is .AdHolder[data-asin]
    // This tests the AdHolder detection path
    let huawei = result.results.iter().find(|r| r.asin == "B0HUAWEI01").unwrap();
    // Note: the AdHolder selector in parse_results selects .AdHolder elements and checks data-asin
    // The result div itself has class AdHolder, so it should be in adholder_asins
    assert!(huawei.is_sponsored, "AdHolder class should mark result as sponsored");
    assert!(result.huawei_sponsored_found);
}

#[test]
fn positions_are_1_based() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");
    // First result should be position 1
    assert_eq!(result.results[0].position, 1);
    assert_eq!(result.results[1].position, 2);
}

#[test]
fn build_search_url_encodes_spaces() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr", "montre connectee");
    assert_eq!(url, "https://www.amazon.fr/s?k=montre+connectee");
}

#[test]
fn build_search_url_trims_trailing_slash() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr/", "montre");
    assert_eq!(url, "https://www.amazon.fr/s?k=montre");
}
```

---

## Task 12: Rewrite README.md

**Files:**
- Modify: `README.md`

**Step 1: Replace entire README.md**

```markdown
# monitoring-the-situation

## What This Does

Watches page 1 of amazon.fr search results for "montre connectee". When a Huawei smartwatch appears as a **sponsored ad**, it fires a Telegram alert. When the ad disappears, it sends a recovery notification. Runs 24/7 on your System76 laptop (Pop!_OS) or an Oracle Cloud free VM.

No Amazon account required — it scrapes the public search page.

---

## Prerequisites

- **Rust** — install with:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Telegram account** (free, any device)
- **24/7 host** — your System76 laptop running Pop!_OS, or an Oracle Cloud Always Free VM

---

## Telegram Bot Setup

1. Open Telegram and search for **@BotFather**, then tap Start.
2. Send `/newbot`, choose a display name, then choose a username ending in `bot`.
3. BotFather gives you a token like `123456:ABCdef...`. Save it.
4. Find your **chat_id**:

   ```bash
   # First, send any message to your new bot in Telegram. Then:
   curl https://api.telegram.org/botYOUR_TOKEN/getUpdates
   # Look for: {"chat":{"id": 123456789, ...}}
   # That number is your chat_id.
   ```

---

## Quick Start

```bash
# 1. Clone and build
git clone https://github.com/YOUR_USERNAME/monitoring-the-situation.git
cd monitoring-the-situation
cargo build --release

# 2. Configure
cp .env.example .env
cp config.toml.example config.toml
# Edit .env: set TELEGRAM_BOT_TOKEN
# Edit config.toml: set telegram.chat_id

# 3. Validate config and test Telegram
cargo run -- dry-run

# 4. Run
cargo run -- run
```

---

## Configuration Reference

### `.env`

| Field | Description |
|-------|-------------|
| `TELEGRAM_BOT_TOKEN` | Telegram bot token from BotFather |

### `config.toml`

| Field | Default | Description |
|-------|---------|-------------|
| `scraper.keyword` | `"montre connectee"` | Search keyword on amazon.fr |
| `scraper.marketplace_url` | `"https://www.amazon.fr"` | Amazon marketplace base URL |
| `scraper.brand_filter` | `"huawei"` | Brand to detect in sponsored results (case-insensitive) |
| `telegram.chat_id` | required | Your Telegram user ID |
| `monitoring.interval_minutes` | `30` | How often to check (minimum 5) |

---

## CLI Commands

```bash
cargo run -- run        # Start the monitoring daemon
cargo run -- check-now  # Run a single check, then exit
cargo run -- dry-run    # Validate config and send a test Telegram message
RUST_LOG=debug cargo run -- run  # Verbose logging
```

---

## Deploy 24/7 (Two Options)

### Option A — System76 Laptop with Pop!_OS (Recommended)

Full guide: `deploy/POPOS_SETUP.md`

```bash
make setup-local
scp .env user@laptop:/opt/ads-monitor/.env
scp config.toml user@laptop:/opt/ads-monitor/config.toml
make deploy-local
make status-local
```

### Option B — Oracle Cloud Free VM

Full guide: `deploy/ORACLE_SETUP.md`

```bash
make setup-cloud
scp .env user@vm:/opt/ads-monitor/.env
scp config.toml user@vm:/opt/ads-monitor/config.toml
make deploy-cloud
make status-cloud
```

---

## How It Works

1. Every `interval_minutes`, the daemon sends an HTTP GET to `https://www.amazon.fr/s?k=montre+connectee` with a rotating browser User-Agent and French `Accept-Language` headers.
2. The HTML response is parsed using CSS selectors to find all `div[data-component-type="s-search-result"]` elements.
3. Each result is checked for sponsorship: the `sp-sponsored-result` component type, `.AdHolder` class, or the text "Sponsorisé"/"Sponsored".
4. Sponsored results with "huawei" in the title trigger an alert.
5. State is persisted to `state.json` so the daemon only alerts on **changes** (appeared / disappeared), not on every check.

---

## Limitations

- Amazon may change their HTML structure at any time, breaking the CSS selectors.
- Amazon may serve a CAPTCHA page if requests are too frequent — keep `interval_minutes` at 30 or higher for safety.
- The daemon does not rotate IP addresses. If blocked, wait a few hours before retrying.
- Results may vary by geographic location — run the daemon from a French IP for best accuracy.

---

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `CAPTCHA or bot-detection page detected` | Amazon is rate-limiting | Increase `interval_minutes`, wait a few hours |
| `Amazon returned 503` | Temporary block | Wait and retry |
| `TELEGRAM_BOT_TOKEN not set` | Missing env var | Check your `.env` file |
| Telegram not sending | Wrong `chat_id` | Re-run `getUpdates` (see Telegram Bot Setup) |
| `interval_minutes must be at least 5` | Poll interval too low | Set `interval_minutes = 30` in config.toml |
| `Connection refused` on SSH | Firewall blocking port 22 | Check Oracle Security List |
```

---

## Task 13: Final Verification

**Step 1: Build release**

```bash
cargo build --release
```

Expected: Compiles successfully with no errors.

**Step 2: Clippy clean**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings or errors.

**Step 3: Run all tests**

```bash
cargo test
```

Expected: All tests pass.

**Step 4: Help works**

```bash
cargo run -- --help
```

Expected: Shows usage with `run`, `check-now`, `dry-run` subcommands.

**Step 5: Dry-run gets past config loading**

```bash
APP__SCRAPER__KEYWORD="montre connectee" \
APP__SCRAPER__MARKETPLACE_URL="https://www.amazon.fr" \
APP__SCRAPER__BRAND_FILTER="huawei" \
APP__TELEGRAM__CHAT_ID="123456789" \
APP__MONITORING__INTERVAL_MINUTES="30" \
TELEGRAM_BOT_TOKEN="fake-token-for-test" \
cargo run -- dry-run
```

Expected: Gets past config loading, fails on Telegram (fake token) — that's OK.

---

## Key Implementation Notes

### ScraperConfig needs Clone
In `src/config.rs`, `ScraperConfig` must derive `Clone` because `main.rs` does `Arc::new(config.scraper.clone())`:
```rust
#[derive(Debug, Deserialize, Clone)]
pub struct ScraperConfig { ... }
```

### Module naming
The module is named `amazon_scraper` (not `scraper`) to avoid conflict with the `scraper` crate. In `lib.rs`: `pub mod amazon_scraper;`. In `main.rs`: `use monitoring_the_situation::amazon_scraper;`.

### parse_results is pub
`AmazonScraper::parse_results` must be `pub` so `tests/scraper_tests.rs` can call it directly without HTTP.

### build_search_url is pub
`AmazonScraper::build_search_url` must be `pub` for the URL-encoding tests.

### No unwrap() in production code
`Selector::parse(...)` calls use `unwrap()` — this is acceptable because the CSS selectors are compile-time constants and will never fail. All other error paths use `?` or `anyhow::bail!`.
