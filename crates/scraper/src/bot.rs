use mts_common::escape_html;

use std::sync::Arc;

use serde::Deserialize;
use tracing::{info, warn};

use crate::amazon_scraper::AmazonScraper;
use mts_common::models::MonitorState;
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

pub struct CommandListener {
    client: reqwest::Client,
    bot_token: String,
    chat_id: i64,
    scraper: Arc<AmazonScraper>,
    state_manager: Arc<StateManager>,
    brand_filter: String,
}

impl CommandListener {
    pub fn new(
        bot_token: String,
        chat_id: i64,
        scraper: Arc<AmazonScraper>,
        state_manager: Arc<StateManager>,
        brand_filter: String,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            bot_token,
            chat_id,
            scraper,
            state_manager,
            brand_filter,
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

            let response = match self.client.get(&url).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    warn!("getUpdates request failed: {e}. Retrying in 5s...");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let body = match response.json::<TelegramResponse<Vec<Update>>>().await {
                Ok(body) => body,
                Err(e) => {
                    warn!("Failed to parse getUpdates response: {e}. Retrying in 5s...");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
                    warn!(
                        "Ignoring message from unknown chat_id: {}",
                        message.chat.id
                    );
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
                    "/filter" => {
                        self.handle_filter(&text).await
                    }
                    _ => {}
                }
            }
        }
    }

    async fn handle_start(&self) {
        let text = " <b>amazonad-bot</b>\n\nCommands:\n/status - current monitoring state\n/check - scrape amazon.fr right now\n/list - all sponsored products + brands\n/filter &lt;brand&gt; - filter sponsored by brand name";
        self.send_reply(text).await;
    }

    async fn handle_status(&self) {
        let state = match self.state_manager.load() {
            Ok(Some(s)) => s,
            Ok(None) => {
                self.send_reply("No data yet - daemon has not run a check yet.")
                    .await;
                return;
            }
            Err(e) => {
                self.send_reply(&format!(" Failed to load state: {e:#}"))
                    .await;
                return;
            }
        };

        let pos_str = if state.huawei_positions.is_empty() {
            "-".to_string()
        } else {
            state
                .huawei_positions
                .iter()
                .map(|p| if *p == 0 { "Carousel".to_string() } else { p.to_string() })
                .collect::<Vec<_>>()
                .join(", ")
        };

        let visible = if state.huawei_ad_visible {
            "Yes"
        } else {
            "No"
        };

        let text = format!(
            "Status
             
             Huawei ad visible: <b>{visible}</b>
             Position(s): {pos_str}
             Last checked: {}
             Total results scraped: {}",
            state.updated_at.format("%Y-%m-%d %H:%M UTC "),
            state.total_results_scraped,
        );

        self.send_reply(&text).await;
    }

    async fn handle_check(&self) {
        self.send_reply(" Scraping amazon.fr...").await;

        let scrape_result = match self.scraper.scrape_search_page().await {
            Ok(r) => r,
            Err(e) => {
                self.send_reply(&format!(" Scrape failed: {e:#}")).await;
                return;
            }
        };

        let brand_lower = self.brand_filter.to_lowercase();
        let sponsored: Vec<(u32, usize, String, bool)> = scrape_result
            .results
            .iter()
            .filter(|r| r.is_sponsored)
            .map(|r| {
                let is_huawei = r.title.to_lowercase().contains(&brand_lower);
                (r.page, r.position_in_page, r.title.clone(), is_huawei)
            })
            .collect();

        let new_state = MonitorState {
            huawei_ad_visible: scrape_result.huawei_sponsored_found,
            huawei_positions: scrape_result.huawei_sponsored_positions.clone(),
            total_results_scraped: scrape_result.results.len(),
            updated_at: chrono::Utc::now(),
        };

        if let Err(e) = self.state_manager.save(&new_state) {
            self.send_reply(&format!(" Scrape succeeded but failed to save state: {e:#}"))
                .await;
            return;
        }

        let huawei_line = if scrape_result.huawei_sponsored_found {
            let pos_str = scrape_result
                .results
                .iter()
                .filter(|r| r.is_sponsored && r.title.to_lowercase().contains(&brand_lower))
                .map(|r| if r.position_in_page == 0 {
                    format!("Page {} Carousel", r.page)
                } else {
                    format!("Page {} #{}", r.page, r.position_in_page)
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("Huawei ad: <b>visible at {pos_str}</b>")
        } else {
            "Huawei ad: <b>not visible</b>".to_string()
        };

        let display_items = sponsored.iter().take(20).collect::<Vec<_>>();
        let truncated = sponsored.len().saturating_sub(20);

        let mut sponsored_list = String::new();
        for (page, pos, title, is_huawei) in &display_items {
            let marker = if *is_huawei { " " } else { "" };
            let loc = if *pos == 0 { format!("Page {page} Carousel") } else { format!("Page {page} #{pos}") };
            sponsored_list.push_str(&format!("* {loc} - {}{marker}\n", escape_html(title)));
        }
        if truncated > 0 {
            sponsored_list.push_str(&format!("... and {truncated} more (use /list to see all)"));
        }

        let text = format!(
            " <b>Check complete</b>

             

             {huawei_line}

             Sponsored products ({} total):

             {sponsored_list}

             (State saved)",
            sponsored.len(),
        );

        self.send_reply(text.trim()).await;
    }

    async fn handle_list(&self) {
        self.send_reply(" Scraping amazon.fr...").await;

        let scrape_result = match self.scraper.scrape_search_page().await {
            Ok(r) => r,
            Err(e) => {
                self.send_reply(&format!(" Scrape failed: {e:#}")).await;
                return;
            }
        };

        let sponsored: Vec<(u32, usize, &str)> = scrape_result
            .results
            .iter()
            .filter(|r| r.is_sponsored)
            .map(|r| (r.page, r.position_in_page, r.title.as_str()))
            .collect();

        let new_state = MonitorState {
            huawei_ad_visible: scrape_result.huawei_sponsored_found,
            huawei_positions: scrape_result.huawei_sponsored_positions.clone(),
            total_results_scraped: scrape_result.results.len(),
            updated_at: chrono::Utc::now(),
        };

        if let Err(e) = self.state_manager.save(&new_state) {
            warn!("Failed to save state after /list: {e:#}");
        }

        if sponsored.is_empty() {
            self.send_reply(" No sponsored products found.").await;
            return;
        }

        let display_items = sponsored.iter().take(20).collect::<Vec<_>>();
        let truncated = sponsored.len().saturating_sub(20);

        let mut list = String::new();
        for (page, pos, title) in &display_items {
            let loc = if *pos == 0 { format!("Page {page} Carousel") } else { format!("Page {page} #{pos}") };
            list.push_str(&format!("* {loc} - {}\n", escape_html(title)));
        }
        if truncated > 0 {
            list.push_str(&format!("... and {truncated} more "));
        }

        let mut brands: Vec<String> = sponsored
            .iter()
            .filter_map(|(_, _, title)| {
                title.split_whitespace().next().map(|s| s.to_string())
            })
            .collect();
        brands.sort();
        brands.dedup();
        let brands_str = brands.join(", ");

        let text = format!(
            " <b>Sponsored products right now ({} total):</b>

             

             {list}

              Brands: {brands_str}",
            sponsored.len(),
        );

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
            self.send_reply("Usage: /filter <brand name> - Example: /filter samsung ")
                .await;
            return;
        }

        self.send_reply(" Scraping amazon.fr...").await;

        let scrape_result = match self.scraper.scrape_search_page().await {
            Ok(r) => r,
            Err(e) => {
                self.send_reply(&format!(" Scrape failed: {e:#}")).await;
                return;
            }
        };

        let brand_lower = arg.to_lowercase();
        let filtered: Vec<(u32, usize, &str)> = scrape_result
            .results
            .iter()
            .filter(|r| r.is_sponsored && r.title.to_lowercase().contains(&brand_lower))
            .map(|r| (r.page, r.position_in_page, r.title.as_str()))
            .collect();

        if filtered.is_empty() {
            self.send_reply(&format!(r#"No sponsored products matching "{arg}" found. "#))
            .await;
            return;
        }

        let display_items = filtered.iter().take(20).collect::<Vec<_>>();
        let truncated = filtered.len().saturating_sub(20);

        let mut list = String::new();
        for (page, pos, title) in &display_items {
            let loc = if *pos == 0 { format!("Page {page} Carousel") } else { format!("Page {page} #{pos}") };
            list.push_str(&format!("* {loc} - {}\n", escape_html(title)));
        }
        if truncated > 0 {
            list.push_str(&format!("... and {truncated} more "));
        }

        let text = format!(
            r#" <b>Sponsored products matching "{arg}" ({} found):</b>
             
             {list}"#,
            filtered.len(),
        );

        self.send_reply(text.trim()).await;
    }

    async fn send_reply(&self, text: &str) {
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
