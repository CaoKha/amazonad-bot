use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub chat_id: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MonitoringConfig {
    pub interval_minutes: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}
