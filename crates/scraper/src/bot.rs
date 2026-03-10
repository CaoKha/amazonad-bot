use mts_common::escape_html;

use std::sync::Arc;

use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::amazon_scraper::AmazonScraper;
use mts_common::state::StateManager;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    chat: Chat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
}

pub struct BotMarketplace {
    pub code: String,
    pub url: String,
    pub keywords: Vec<String>,
}

pub struct CommandListener {
    client: reqwest::Client,
    bot_token: String,
    chat_id: i64,
    #[allow(dead_code)]
    scraper: Arc<AmazonScraper>,
    state_manager: Arc<StateManager>,
    brand_filter: String,
    marketplaces: Vec<BotMarketplace>,
    cancel_token: CancellationToken,
}

impl CommandListener {
    pub fn new(
        bot_token: String,
        chat_id: i64,
        scraper: Arc<AmazonScraper>,
        state_manager: Arc<StateManager>,
        brand_filter: String,
        marketplaces: Vec<BotMarketplace>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            bot_token,
            chat_id,
            scraper,
            state_manager,
            brand_filter,
            marketplaces,
            cancel_token,
        }
    }

    pub async fn run(self) {
        info!("Bot command listener started (chat_id: {})", self.chat_id);
        let mut offset: i64 = 0;

        loop {
            let url = format!(
                "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=25&allowed_updates=[\"message\"]",
                self.bot_token, offset
            );

            // Race the HTTP long-poll against shutdown cancellation
            let response = tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    info!("Bot listener shutting down.");
                    return;
                }
                result = self.client.get(&url).send() => result,
            };

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    warn!("getUpdates request failed: {e}. Retrying in 5s...");
                    tokio::select! {
                        _ = self.cancel_token.cancelled() => return,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
                    }
                    continue;
                }
            };

            let body = match response.json::<TelegramResponse<Vec<Update>>>().await {
                Ok(body) => body,
                Err(e) => {
                    warn!("Failed to parse getUpdates response: {e}. Retrying in 5s...");
                    tokio::select! {
                        _ = self.cancel_token.cancelled() => return,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
                    }
                    continue;
                }
            };

            let updates = match body.result {
                Some(updates) => updates,
                None => continue,
            };

            for update in updates {
                offset = update.update_id + 1;

                let message = match update.message {
                    Some(m) => m,
                    None => continue,
                };

                if message.chat.id != self.chat_id {
                    warn!("Ignoring message from unknown chat_id: {}", message.chat.id);
                    continue;
                }

                let text = match message.text {
                    Some(t) => t,
                    None => continue,
                };

                let command = text
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .split('@')
                    .next()
                    .unwrap_or("");

                match command {
                    "/start" => self.handle_start().await,
                    "/status" => self.handle_status().await,
                    "/check" => self.handle_check().await,
                    "/list" => self.handle_list().await,
                    "/filter" => self.handle_filter(&text).await,
                    _ => {}
                }
            }
        }
    }

    async fn handle_start(&self) {
        let total_keywords: usize = self.marketplaces.iter().map(|m| m.keywords.len()).sum();
        let mut text = format!(
            "🔍 <b>amazonad-bot</b>\n\nMonitoring {} keyword(s):\n",
            total_keywords
        );

        for mp in &self.marketplaces {
            text.push_str(&format!("\n<b>[{}]</b>\n", escape_html(&mp.code)));
            for kw in &mp.keywords {
                let url = AmazonScraper::build_search_url(&mp.url, kw, 1);
                let url_escaped = escape_html(&url);
                text.push_str(&format!(
                    "• <code>{}</code> → <a href=\"{url_escaped}\">{url_escaped}</a>\n",
                    escape_html(kw)
                ));
            }
        }

        text.push_str(
            "\nCommands:\n\
             /status — current monitoring state\n\
             /check — show cached last-sweep results\n\
             /list — all sponsored products from cache\n\
             /filter &lt;brand&gt; — filter by brand name",
        );

        self.send_reply(&text).await;
    }

    async fn handle_status(&self) {
        let state = match self.state_manager.load() {
            Ok(Some(s)) => s,
            Ok(None) => {
                self.send_reply("⏳ No data yet — daemon has not completed a sweep.")
                    .await;
                return;
            }
            Err(e) => {
                self.send_reply(&format!("❌ Failed to load state: {e:#}"))
                    .await;
                return;
            }
        };

        let total_keywords: usize = self.marketplaces.iter().map(|m| m.keywords.len()).sum();
        let mut lines = Vec::new();
        let mut last_check: Option<chrono::DateTime<chrono::Utc>> = None;

        for mp in &self.marketplaces {
            lines.push(format!("\n<b>[{}]</b>", escape_html(&mp.code)));
            for kw in &mp.keywords {
                let state_key = format!("{}:{}", mp.code, kw);
                let ks = state.keywords.get(state_key.as_str());
                match ks {
                    Some(ks) => {
                        if let Some(lc) = ks.last_checked {
                            last_check = Some(
                                last_check
                                    .map_or(lc, |prev: chrono::DateTime<chrono::Utc>| prev.max(lc)),
                            );
                        }
                        let vis = if ks.brand_ad_visible {
                            "✅ visible"
                        } else {
                            "❌ not visible"
                        };
                        lines.push(format!(
                            "• <code>{}</code>: {} {}",
                            escape_html(kw),
                            escape_html(&self.brand_filter),
                            vis
                        ));
                    }
                    None => {
                        lines.push(format!(
                            "• <code>{}</code>: not yet checked",
                            escape_html(kw)
                        ));
                    }
                }
            }
        }

        let last_str = last_check
            .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "never".to_string());

        let text = format!(
            "⚙️ <b>Monitoring {} keyword(s)</b>\nLast sweep: {}\n{}",
            total_keywords,
            last_str,
            lines.join("\n"),
        );
        self.send_reply(&text).await;
    }

    async fn handle_check(&self) {
        let state = match self.state_manager.load() {
            Ok(Some(s)) => s,
            Ok(None) => {
                self.send_reply("⏳ No data yet — daemon has not completed a sweep.")
                    .await;
                return;
            }
            Err(e) => {
                self.send_reply(&format!("❌ Failed to load state: {e:#}"))
                    .await;
                return;
            }
        };

        let mut lines = Vec::new();

        for mp in &self.marketplaces {
            lines.push(format!("\n<b>[{}]</b>", escape_html(&mp.code)));
            for kw in &mp.keywords {
                let state_key = format!("{}:{}", mp.code, kw);
                match state.keywords.get(state_key.as_str()) {
                    Some(ks) => {
                        let time_str = ks
                            .last_checked
                            .map(|t| format!("checked {}", t.format("%H:%M")))
                            .unwrap_or_else(|| "not yet checked".to_string());

                        if ks.brand_ad_visible {
                            let pos_str = ks
                                .brand_positions
                                .iter()
                                .map(|(page, pos, pt)| {
                                    let loc = if *pos == 0 {
                                        format!("Page {} Carousel", page)
                                    } else {
                                        format!("Page {} #{}", page, pos)
                                    };
                                    if let Some(pt) = pt {
                                        format!("{loc} [{pt}]")
                                    } else {
                                        loc
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            lines.push(format!(
                                "• <code>{}</code>: {} ✅ {} ({})",
                                escape_html(kw),
                                escape_html(&self.brand_filter),
                                pos_str,
                                time_str,
                            ));
                        } else {
                            lines.push(format!(
                                "• <code>{}</code>: {} ❌ not visible ({})",
                                escape_html(kw),
                                escape_html(&self.brand_filter),
                                time_str,
                            ));
                        }
                    }
                    None => {
                        lines.push(format!(
                            "• <code>{}</code>: not yet checked",
                            escape_html(kw)
                        ));
                    }
                }
            }
        }

        let text = format!("📊 <b>Last sweep results:</b>\n{}", lines.join("\n"));
        self.send_reply(&text).await;
    }

    async fn handle_list(&self) {
        let state = match self.state_manager.load() {
            Ok(Some(s)) => s,
            Ok(None) => {
                self.send_reply("⏳ No data yet — daemon has not completed a sweep.")
                    .await;
                return;
            }
            Err(e) => {
                self.send_reply(&format!("❌ Failed to load state: {e:#}"))
                    .await;
                return;
            }
        };

        let total_keywords: usize = self.marketplaces.iter().map(|m| m.keywords.len()).sum();
        let mut text = String::from("📋 <b>All sponsored products (cached):</b>\n");
        let mut total = 0usize;

        for mp in &self.marketplaces {
            text.push_str(&format!("\n<b>[{}]</b>\n", escape_html(&mp.code)));
            for kw in &mp.keywords {
                let state_key = format!("{}:{}", mp.code, kw);
                match state.keywords.get(state_key.as_str()) {
                    Some(ks) => {
                        let sponsored: Vec<_> =
                            ks.last_results.iter().filter(|r| r.is_sponsored).collect();
                        if sponsored.is_empty() {
                            text.push_str(&format!(
                                "\n<b>{}</b>: no sponsored products\n",
                                escape_html(kw)
                            ));
                        } else {
                            text.push_str(&format!(
                                "\n<b>{}</b> ({} sponsored):\n",
                                escape_html(kw),
                                sponsored.len()
                            ));
                            for r in sponsored.iter().take(15) {
                                let loc = if r.position_in_page == 0 {
                                    format!("Page {} Carousel", r.page)
                                } else {
                                    format!("Page {} #{}", r.page, r.position_in_page)
                                };
                                let tag = r
                                    .placement_type
                                    .as_ref()
                                    .map(|t| format!(" [{t}]"))
                                    .unwrap_or_default();
                                text.push_str(&format!(
                                    "• {loc}{tag} — {}\n",
                                    escape_html(&r.title)
                                ));
                            }
                            let remaining = sponsored.len().saturating_sub(15);
                            if remaining > 0 {
                                text.push_str(&format!("... and {} more\n", remaining));
                            }
                            total += sponsored.len();
                        }
                    }
                    None => {
                        text.push_str(&format!("\n<b>{}</b>: not yet checked\n", escape_html(kw)));
                    }
                }
            }
        }

        text.push_str(&format!(
            "\n<i>Total: {} sponsored across {} keywords</i>",
            total, total_keywords
        ));
        self.send_reply(text.trim()).await;
    }

    async fn handle_filter(&self, full_text: &str) {
        let arg = full_text
            .trim()
            .split_once(' ')
            .map(|x| x.1)
            .unwrap_or("")
            .trim()
            .to_string();

        if arg.is_empty() {
            self.send_reply("Usage: /filter &lt;brand name&gt;\nExample: /filter samsung")
                .await;
            return;
        }

        let state = match self.state_manager.load() {
            Ok(Some(s)) => s,
            Ok(None) => {
                self.send_reply("⏳ No data yet — daemon has not completed a sweep.")
                    .await;
                return;
            }
            Err(e) => {
                self.send_reply(&format!("❌ Failed to load state: {e:#}"))
                    .await;
                return;
            }
        };

        let filter_lower = arg.to_lowercase();
        let mut text = format!(
            "🔎 <b>Sponsored products matching \"{}\":</b>\n",
            escape_html(&arg)
        );
        let mut total = 0usize;

        for mp in &self.marketplaces {
            let mut mp_lines = Vec::new();
            for kw in &mp.keywords {
                let state_key = format!("{}:{}", mp.code, kw);
                if let Some(ks) = state.keywords.get(state_key.as_str()) {
                    let matched: Vec<_> = ks
                        .last_results
                        .iter()
                        .filter(|r| {
                            r.is_sponsored && r.title.to_lowercase().contains(&filter_lower)
                        })
                        .collect();
                    if !matched.is_empty() {
                        mp_lines.push(format!("\n<b>{}</b>:\n", escape_html(kw)));
                        for r in matched.iter().take(10) {
                            let loc = if r.position_in_page == 0 {
                                format!("Page {} Carousel", r.page)
                            } else {
                                format!("Page {} #{}", r.page, r.position_in_page)
                            };
                            let tag = r
                                .placement_type
                                .as_ref()
                                .map(|t| format!(" [{t}]"))
                                .unwrap_or_default();
                            mp_lines.push(format!("• {loc}{tag} — {}\n", escape_html(&r.title)));
                        }
                        total += matched.len();
                    }
                }
            }
            if !mp_lines.is_empty() {
                text.push_str(&format!("\n<b>[{}]</b>", escape_html(&mp.code)));
                for line in mp_lines {
                    text.push_str(&line);
                }
            }
        }

        if total == 0 {
            self.send_reply(&format!(
                "No sponsored products matching \"{}\" found in cache.",
                escape_html(&arg)
            ))
            .await;
        } else {
            text.push_str(&format!("\n<i>{} match(es) total</i>", total));
            self.send_reply(text.trim()).await;
        }
    }

    async fn send_reply(&self, text: &str) {
        // Split long messages at newline boundaries
        if text.len() <= 4000 {
            self.send_single_reply(text).await;
            return;
        }

        let mut remaining = text;
        while !remaining.is_empty() {
            if remaining.len() <= 4000 {
                self.send_single_reply(remaining).await;
                break;
            }
            let mut end = 4000;
            while !remaining.is_char_boundary(end) {
                end -= 1;
            }
            let split_at = remaining[..end].rfind('\n').map(|p| p + 1).unwrap_or(end);
            self.send_single_reply(&remaining[..split_at]).await;
            remaining = &remaining[split_at..];
        }
    }

    async fn send_single_reply(&self, text: &str) {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();
                    warn!("sendMessage returned {status}: {body_text}");
                }
            }
            Err(e) => {
                warn!("sendMessage request failed: {e}");
            }
        }
    }
}
