use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub use mts_common::config::{MonitoringConfig, TelegramConfig};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub scraper: ScraperConfig,
    pub telegram: TelegramConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScraperConfig {
    pub keyword: String,
    pub marketplace_url: String,
    pub brand_filter: String,
    #[serde(default = "default_pages")]
    pub pages: u32,
    #[serde(default)]
    pub chrome_executable: Option<String>,
}

fn default_pages() -> u32 {
    3
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
    if app_config.scraper.pages < 1 || app_config.scraper.pages > 7 {
        bail!(
            "scraper.pages must be between 1 and 7 (got {})",
            app_config.scraper.pages
        );
    }

    Ok(app_config)
}
