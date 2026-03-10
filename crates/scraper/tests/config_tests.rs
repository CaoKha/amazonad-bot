use std::sync::Mutex;

use mts_scraper::config::load_config;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Minimal valid multi-marketplace config TOML.
fn valid_config_toml() -> String {
    r#"
[database]
url = "postgresql://mts:mts@localhost:5432/mts"

[scraper]
brand_filter = "huawei"
pages = 3

[[scraper.marketplaces]]
code = "FR"
url = "https://www.amazon.fr"
keywords = ["montre connectee", "smartwatch"]
accept_language = "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7"
languages = ["fr-FR", "fr", "en-US", "en"]

[[telegram]]
chat_id = 123456789

[monitoring]
interval_minutes = 30
"#
    .to_string()
}

/// Load config from an inline TOML string (bypasses file system).
fn load_config_from_str(toml: &str) -> anyhow::Result<mts_scraper::config::AppConfig> {
    use anyhow::{bail, Context};

    let cfg = config::Config::builder()
        .add_source(config::File::from_str(toml, config::FileFormat::Toml))
        .build()
        .context("Failed to build configuration")?;

    let app_config: mts_scraper::config::AppConfig = cfg
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    // Run the same validations as load_config()
    if app_config.telegram.is_empty() {
        bail!("telegram must contain at least one target.");
    }
    for tg in &app_config.telegram {
        if tg.chat_id == 0 {
            bail!("telegram.chat_id must be set (got 0).");
        }
    }
    if app_config.scraper.marketplaces.is_empty() {
        bail!("scraper.marketplaces must contain at least one marketplace.");
    }
    for mp in &app_config.scraper.marketplaces {
        if mp.code.is_empty() {
            bail!("Each marketplace must have a non-empty code.");
        }
        if mp.url.is_empty() {
            bail!("Marketplace '{}': url must not be empty.", mp.code);
        }
        if mp.keywords.is_empty() {
            bail!(
                "Marketplace '{}': keywords must contain at least one keyword.",
                mp.code
            );
        }
        for kw in &mp.keywords {
            if kw.trim().is_empty() {
                bail!(
                    "Marketplace '{}': keywords must not contain empty or whitespace-only entries.",
                    mp.code
                );
            }
        }
        if mp.accept_language.is_empty() {
            bail!(
                "Marketplace '{}': accept_language must not be empty.",
                mp.code
            );
        }
        if mp.languages.is_empty() {
            bail!("Marketplace '{}': languages must not be empty.", mp.code);
        }
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

#[test]
fn valid_config() {
    let config =
        load_config_from_str(&valid_config_toml()).expect("valid config should load successfully");

    let fr = &config.scraper.marketplaces[0];
    assert_eq!(
        fr.keywords,
        vec!["montre connectee".to_string(), "smartwatch".to_string()]
    );
    assert_eq!(fr.url, "https://www.amazon.fr");
    assert_eq!(fr.code, "FR");
    assert_eq!(config.scraper.brand_filter, "huawei");
    assert_eq!(config.telegram[0].chat_id, 123456789);
    assert_eq!(config.monitoring.interval_minutes, 30);
}

#[test]
fn invalid_chat_id() {
    let toml = valid_config_toml().replace("chat_id = 123456789", "chat_id = 0");
    let result = load_config_from_str(&toml);
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("chat_id"),
        "Error should mention chat_id, got: {err_msg}"
    );
}

#[test]
fn empty_keywords_list() {
    let toml = valid_config_toml().replace(
        r#"keywords = ["montre connectee", "smartwatch"]"#,
        r#"keywords = []"#,
    );
    let result = load_config_from_str(&toml);
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("keywords"),
        "Error should mention keywords, got: {err_msg}"
    );
}

#[test]
fn interval_too_low() {
    let toml = valid_config_toml().replace("interval_minutes = 30", "interval_minutes = 4");
    let result = load_config_from_str(&toml);
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("interval_minutes"),
        "Error should mention interval_minutes, got: {err_msg}"
    );
}

#[test]
fn interval_at_minimum_is_valid() {
    let toml = valid_config_toml().replace("interval_minutes = 30", "interval_minutes = 5");
    let result = load_config_from_str(&toml);
    assert!(result.is_ok(), "interval=5 should be valid");
}

#[test]
fn pages_defaults_to_3_when_not_set() {
    // Remove the pages field entirely
    let toml = valid_config_toml().replace("\npages = 3\n", "\n");
    let config =
        load_config_from_str(&toml).expect("config without pages field should load with default");
    assert_eq!(config.scraper.pages, 3, "pages should default to 3");
}

#[test]
fn pages_can_be_set_explicitly() {
    let toml = valid_config_toml().replace("pages = 3", "pages = 5");
    let config = load_config_from_str(&toml).expect("pages=5 should be valid");
    assert_eq!(config.scraper.pages, 5);
}

#[test]
fn pages_zero_is_invalid() {
    let toml = valid_config_toml().replace("pages = 3", "pages = 0");
    let result = load_config_from_str(&toml);
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("pages"),
        "Error should mention pages, got: {err_msg}"
    );
}

#[test]
fn pages_8_is_invalid() {
    let toml = valid_config_toml().replace("pages = 3", "pages = 8");
    let result = load_config_from_str(&toml);
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("pages"),
        "Error should mention pages, got: {err_msg}"
    );
}

#[test]
fn pages_7_is_valid() {
    let toml = valid_config_toml().replace("pages = 3", "pages = 7");
    let result = load_config_from_str(&toml);
    assert!(result.is_ok(), "pages=7 should be valid");
}

#[test]
fn pages_1_is_valid() {
    let toml = valid_config_toml().replace("pages = 3", "pages = 1");
    let result = load_config_from_str(&toml);
    assert!(result.is_ok(), "pages=1 should be valid");
}

#[test]
fn keywords_with_blank_entry_is_invalid() {
    let toml = valid_config_toml().replace(
        r#"keywords = ["montre connectee", "smartwatch"]"#,
        r#"keywords = ["valid", ""]"#,
    );
    let result = load_config_from_str(&toml);
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("keywords"),
        "Error should mention keywords, got: {err_msg}"
    );
}

#[test]
fn multi_marketplace_config() {
    let toml = r#"
[scraper]
brand_filter = "huawei"
pages = 2

[[scraper.marketplaces]]
code = "FR"
url = "https://www.amazon.fr"
keywords = ["montre connectee"]
accept_language = "fr-FR,fr;q=0.9"
languages = ["fr-FR", "fr"]

[[scraper.marketplaces]]
code = "DE"
url = "https://www.amazon.de"
keywords = ["smartwatch", "fitness tracker"]
accept_language = "de-DE,de;q=0.9"
languages = ["de-DE", "de"]

[[scraper.marketplaces]]
code = "ES"
url = "https://www.amazon.es"
keywords = ["reloj inteligente"]
accept_language = "es-ES,es;q=0.9"
languages = ["es-ES", "es"]

[[telegram]]
chat_id = 999888777

[monitoring]
interval_minutes = 10
"#;
    let config = load_config_from_str(toml).expect("multi-marketplace config should load");
    assert_eq!(config.scraper.marketplaces.len(), 3);
    assert_eq!(config.scraper.marketplaces[0].code, "FR");
    assert_eq!(config.scraper.marketplaces[1].code, "DE");
    assert_eq!(config.scraper.marketplaces[2].code, "ES");
    assert_eq!(
        config.scraper.marketplaces[1].keywords,
        vec!["smartwatch".to_string(), "fitness tracker".to_string()]
    );
}

// Smoke test: load_config() reads the workspace config.toml without panicking.
#[test]
fn load_config_reads_config_toml_file() {
    let _guard = ENV_LOCK.lock().unwrap();
    // load_config() may fail if config.toml is missing or TELEGRAM_BOT_TOKEN not set,
    // but we just verify it doesn't panic.
    let _ = load_config(); // result ignored — just checking no panic
}
