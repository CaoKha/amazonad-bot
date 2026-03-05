use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub chat_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct MonitoringConfig {
    pub interval_minutes: u64,
}
