use anyhow::{bail, Context, Result};
use serde::de::{self, Deserializer, SeqAccess, Visitor};
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
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub keywords: Vec<String>,
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
    if app_config.scraper.keywords.is_empty() {
        bail!("scraper.keywords must contain at least one keyword.");
    }
    for kw in &app_config.scraper.keywords {
        if kw.trim().is_empty() {
            bail!("scraper.keywords must not contain empty or whitespace-only entries.");
        }
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

/// Deserializes keywords from either a TOML array or an env var string.
/// - TOML array: `keywords = ["a", "b"]` → `vec!["a", "b"]`
/// - Env var string: `APP__SCRAPER__KEYWORDS=a,b` → `vec!["a", "b"]`
/// - Env var single: `APP__SCRAPER__KEYWORDS=montre` → `vec!["montre"]`
/// - Env var empty: `APP__SCRAPER__KEYWORDS=` → `vec![]`
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec;

    impl<'de> Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string or array of strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
            if v.is_empty() {
                Ok(vec![])
            } else if v.contains(',') {
                Ok(v.split(',').map(|s| s.trim().to_string()).collect())
            } else {
                Ok(vec![v.to_string()])
            }
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error> {
            let mut vec = Vec::new();
            while let Some(val) = seq.next_element()? {
                vec.push(val);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}
