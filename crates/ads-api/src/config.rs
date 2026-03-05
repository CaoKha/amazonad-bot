use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub use mts_common::config::{MonitoringConfig, TelegramConfig};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub ads_api: AdsApiConfig,
    pub telegram: TelegramConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdsApiConfig {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    pub profile_id: String,
    pub marketplace: String,
    pub brand_filter: String,
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
        bail!("telegram.chat_id must be set (got 0).");
    }
    if app_config.ads_api.client_id.is_empty() {
        bail!("ads_api.client_id must not be empty.");
    }
    if app_config.monitoring.interval_minutes < 5 {
        bail!(
            "monitoring.interval_minutes must be at least 5 (got {})",
            app_config.monitoring.interval_minutes
        );
    }

    Ok(app_config)
}
