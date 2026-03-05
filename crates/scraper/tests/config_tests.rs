use std::sync::Mutex;

use mts_scraper::config::load_config;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn set_valid_config_env() {
    unsafe {
        std::env::set_var("APP__SCRAPER__KEYWORDS", "montre connectee,smartwatch");
        std::env::set_var("APP__SCRAPER__MARKETPLACE_URL", "https://www.amazon.fr");
        std::env::set_var("APP__SCRAPER__BRAND_FILTER", "huawei");
        std::env::set_var("APP__TELEGRAM__CHAT_ID", "123456789");
        std::env::set_var("APP__MONITORING__INTERVAL_MINUTES", "30");
    }
}

fn clear_config_env() {
    unsafe {
        std::env::remove_var("APP__SCRAPER__KEYWORDS");
        std::env::remove_var("APP__SCRAPER__MARKETPLACE_URL");
        std::env::remove_var("APP__SCRAPER__BRAND_FILTER");
        std::env::remove_var("APP__SCRAPER__PAGES");
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

    assert_eq!(config.scraper.keywords, vec!["montre connectee".to_string(), "smartwatch".to_string()]);
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
fn empty_keywords_list() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__KEYWORDS", ",");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("keywords"),
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

#[test]
fn pages_defaults_to_3_when_not_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();

    let config = load_config().expect("config without pages field should load with default");
    assert_eq!(config.scraper.pages, 3, "pages should default to 3");

    clear_config_env();
}

#[test]
fn pages_can_be_set_explicitly() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__PAGES", "5");
    }

    let config = load_config().expect("pages=5 should be valid");
    assert_eq!(config.scraper.pages, 5);

    clear_config_env();
}

#[test]
fn pages_zero_is_invalid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__PAGES", "0");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("pages"),
        "Error should mention pages, got: {err_msg}"
    );

    clear_config_env();
}

#[test]
fn pages_8_is_invalid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__PAGES", "8");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("pages"),
        "Error should mention pages, got: {err_msg}"
    );

    clear_config_env();
}

#[test]
fn pages_7_is_valid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__PAGES", "7");
    }

    let result = load_config();
    assert!(result.is_ok(), "pages=7 should be valid");

    clear_config_env();
}

#[test]
fn pages_1_is_valid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__PAGES", "1");
    }

    let result = load_config();
    assert!(result.is_ok(), "pages=1 should be valid");

    clear_config_env();
}

#[test]
fn keywords_with_blank_entry_is_invalid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_config_env();
    set_valid_config_env();
    unsafe {
        std::env::set_var("APP__SCRAPER__KEYWORDS", "valid, ");
    }

    let result = load_config();
    assert!(result.is_err());
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("keywords"),
        "Error should mention keywords, got: {err_msg}"
    );

    clear_config_env();
}
