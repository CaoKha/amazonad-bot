use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub chat_id: i64,
    #[serde(default = "default_bot_token_env")]
    pub bot_token_env: String,
}

fn default_bot_token_env() -> String {
    "TELEGRAM_BOT_TOKEN".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct MonitoringConfig {
    pub interval_minutes: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}
