use std::sync::Mutex;

use mts_common::config::TelegramConfig;
use mts_common::notifier::TelegramNotifier;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn new_fails_without_bot_token() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    let config = TelegramConfig { chat_id: 123456789 };
    let result = TelegramNotifier::new(&config, reqwest::Client::new(), "montre connectee".into(), "https://www.amazon.fr/s?k=montre+connectee".into());

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
    let result = TelegramNotifier::new(&config, reqwest::Client::new(), "montre connectee".into(), "https://www.amazon.fr/s?k=montre+connectee".into());
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
    let result = TelegramNotifier::new(&config, reqwest::Client::new(), "montre connectee".into(), "https://www.amazon.fr/s?k=montre+connectee".into());

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
