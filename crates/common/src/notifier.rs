use crate::escape_html;

use anyhow::{bail, Context, Result};
use tracing::warn;

use crate::config::TelegramConfig;

pub struct TelegramNotifier {
    client: reqwest::Client,
    bot_token: String,
    chat_id: i64,
    keyword: String,
    search_url: String,
}

impl TelegramNotifier {
    pub fn new(config: &TelegramConfig, client: reqwest::Client, keyword: String, search_url: String) -> Result<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .context("TELEGRAM_BOT_TOKEN environment variable not set")?;

        if bot_token.is_empty() {
            bail!("TELEGRAM_BOT_TOKEN is empty");
        }

        Ok(Self {
            client,
            bot_token,
            chat_id: config.chat_id,
            keyword,
            search_url,
        })
    }

    pub async fn send_ad_appeared(
        &self,
        positions: &[(u32, usize)],
        sample_title: &str,
        all_sponsored: &[(u32, usize, String)],
    ) -> Result<()> {
        let pos_str = positions
            .iter()
            .map(|(page, pos)| if *pos == 0 { format!("Page {page} Carousel") } else { format!("Page {page} #{pos}") })
            .collect::<Vec<_>>()
            .join(", ");

        let message = if all_sponsored.is_empty() {
            format!(
                "\u{1f50d} <b>Huawei ad detected on <a href=\"{search_url}\">{search_url}</a></b>\n\
                 Keyword: <code>{keyword}</code>\n\
                 Position(s): <b>{pos_str}</b>\n\
                 Title: {}",
                escape_html(sample_title),
                search_url = escape_html(&self.search_url),
                keyword = escape_html(&self.keyword),
            )
        } else {
            let display_items = all_sponsored.iter().take(20).collect::<Vec<_>>();
            let truncated = all_sponsored.len().saturating_sub(20);

            let mut sponsored_list = String::new();
            for (page, pos, title) in &display_items {
                let suffix = if positions.contains(&(*page, *pos)) { " \u{2713}" } else { "" };
                let loc = if *pos == 0 { format!("Page {page} Carousel") } else { format!("Page {page} #{pos}") };
                sponsored_list.push_str(&format!("• {loc} — {}{}\n", escape_html(title), suffix));
            }
            if truncated > 0 {
                sponsored_list.push_str(&format!("... and {truncated} more"));
            } else {
                sponsored_list.pop(); // Remove trailing newline only if no truncation
            }
            
            format!(
                "\u{1f50d} <b>Huawei ad detected on <a href=\"{search_url}\">{search_url}</a></b>\n\
                 Keyword: <code>{keyword}</code>\n\
                 Position(s): <b>{pos_str}</b>\n\
                 Title: {}\n\n\
                 \u{1f4cb} Sponsored products on page ({} total):\n{}",
                escape_html(sample_title),
                all_sponsored.len(),
                sponsored_list,
                search_url = escape_html(&self.search_url),
                keyword = escape_html(&self.keyword),
            )
        };
        self.send_message(&message).await
    }

    pub async fn send_ad_disappeared(&self) -> Result<()> {
        let msg = format!(
            "\u{1f4ed} Huawei ad no longer visible on <a href=\"{search_url}\">{search_url}</a> for \u{2018}{keyword}\u{2019}",
            search_url = escape_html(&self.search_url),
            keyword = escape_html(&self.keyword),
        );
        self.send_message(&msg).await
    }

    pub async fn send_test_message(&self) -> Result<()> {
        self.send_message("\u{1f9b7} amazonad-bot connected successfully")
            .await
    }

    async fn send_message(&self, text: &str) -> Result<()> {
        // Safety net: truncate to 4000 chars if needed
        let text = if text.len() > 4000 {
            let mut end = 4000;
            while !text.is_char_boundary(end) { end -= 1; }
            &text[..end]
        } else {
            text
        };

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });

        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Telegram request failed: {e}. Skipping notification.");
                return Ok(());
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            warn!("Telegram API returned {status}: {body_text}. Skipping notification.");
            return Ok(());
        }

        Ok(())

    }
}
