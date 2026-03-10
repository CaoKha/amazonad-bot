pub mod config;
pub mod db;
pub mod models;
pub mod notifier;
pub mod state;

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
