use crate::escape_html;
use crate::models::{BadgeType, PlacementType};

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

pub type SponsoredEntry = (
    u32,
    usize,
    String,
    Option<PlacementType>,
    Option<String>,
    Option<f32>,
    Option<u32>,
    bool,
    Option<BadgeType>,
);

impl TelegramNotifier {
    pub fn new(
        config: &TelegramConfig,
        client: reqwest::Client,
        keyword: String,
        search_url: String,
    ) -> Result<Self> {
        let bot_token = std::env::var(&config.bot_token_env)
            .with_context(|| format!("{} environment variable not set", config.bot_token_env))?;

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
        positions: &[(u32, usize, Option<PlacementType>)],
        sample_title: &str,
        all_sponsored: &[SponsoredEntry],
    ) -> Result<()> {
        let pos_str = positions
            .iter()
            .map(|(page, pos, pt)| {
                let loc = format_location(*page, *pos);
                if let Some(pt) = pt {
                    format!("{loc} [{}]", abbreviate_placement(pt))
                } else {
                    loc
                }
            })
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
            let mut sponsored_list = String::new();
            for (page, pos, title, pt, price, rating, review_count, is_prime, badge) in
                all_sponsored
            {
                let is_brand_match = positions.iter().any(|(p, po, _)| p == page && *po == *pos);
                sponsored_list.push_str(&format_product_line(ProductLineArgs {
                    page: *page,
                    pos: *pos,
                    title,
                    pt: pt.as_ref(),
                    is_brand_match,
                    price: price.as_deref(),
                    rating: *rating,
                    review_count: *review_count,
                    is_prime: *is_prime,
                    badge: badge.as_ref(),
                }));
                sponsored_list.push('\n');
            }
            // Remove trailing newline
            sponsored_list.pop();

            format!(
                "\u{1f50d} <b>Huawei ad detected on <a href=\"{search_url}\">{search_url}</a></b>\n\
                 Keyword: <code>{keyword}</code>\n\
                 Position(s): <b>{pos_str}</b>\n\
                 Title: {}\n\n\
                 \u{1f4cb} All paid placements ({} total):\n{}",
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
        let chunks = split_message(text, 4000);

        for chunk in &chunks {
            self.send_single_message(chunk).await?;
        }

        Ok(())
    }

    async fn send_single_message(&self, text: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

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

/// Format a page+position into a human-readable location string.
fn format_location(page: u32, pos: usize) -> String {
    if pos == 0 {
        format!("Page {page} Top/Carousel")
    } else {
        format!("Page {page} #{pos}")
    }
}

/// Abbreviate a placement type to a short tag.
fn abbreviate_placement(pt: &PlacementType) -> &'static str {
    match pt {
        PlacementType::SponsoredProduct => "SP",
        PlacementType::SponsoredProductCarousel => "SPC",
        PlacementType::SponsoredBrand => "SB",
        PlacementType::SponsoredBrandVideo => "SBV",
        PlacementType::EditorialRecommendation => "ED",
    }
}

struct ProductLineArgs<'a> {
    page: u32,
    pos: usize,
    title: &'a str,
    pt: Option<&'a PlacementType>,
    is_brand_match: bool,
    price: Option<&'a str>,
    rating: Option<f32>,
    review_count: Option<u32>,
    is_prime: bool,
    badge: Option<&'a BadgeType>,
}

/// Format a single product line with enrichment data.
fn format_product_line(args: ProductLineArgs<'_>) -> String {
    let ProductLineArgs {
        page,
        pos,
        title,
        pt,
        is_brand_match,
        price,
        rating,
        review_count,
        is_prime,
        badge,
    } = args;
    let loc = format_location(page, pos);

    let mut segments: Vec<String> = Vec::new();

    // Placement type tag
    if let Some(pt) = pt {
        segments.push(format!("[{}]", abbreviate_placement(pt)));
    }

    // Rating + review count
    if let Some(r) = rating {
        let rating_str = if let Some(count) = review_count {
            format!("⭐{r} ({count})")
        } else {
            format!("⭐{r}")
        };
        segments.push(rating_str);
    } else if let Some(count) = review_count {
        segments.push(format!("({count})"));
    }

    // Price
    if let Some(p) = price {
        segments.push(p.to_string());
    }

    // Prime
    if is_prime {
        segments.push("Prime".to_string());
    }

    // Badge
    if let Some(b) = badge {
        segments.push(b.to_string());
    }

    let meta = if segments.is_empty() {
        String::new()
    } else {
        format!(" {}", segments.join(" | "))
    };

    let suffix = if is_brand_match { " ✓" } else { "" };

    format!("• {loc}{meta} — {}{suffix}", escape_html(title))
}

/// Split a message into chunks that fit within Telegram's character limit.
/// Splits at newline boundaries when possible; falls back to char-boundary splits.
fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Find the safe byte boundary at max_len
        let mut safe_end = max_len;
        while !remaining.is_char_boundary(safe_end) {
            safe_end -= 1;
        }

        // Try to split at the last newline within the safe range
        let split_at = remaining[..safe_end]
            .rfind('\n')
            .map(|pos| pos + 1) // include the newline in the current chunk
            .unwrap_or(safe_end); // no newline — split at char boundary

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_message_stays_single_chunk() {
        let msg = "Hello world";
        let chunks = split_message(msg, 100);
        assert_eq!(chunks, vec!["Hello world"]);
    }

    #[test]
    fn exact_limit_stays_single_chunk() {
        let msg = "abcde";
        let chunks = split_message(msg, 5);
        assert_eq!(chunks, vec!["abcde"]);
    }

    #[test]
    fn splits_at_newline_boundary() {
        let msg = "line1\nline2\nline3\nline4";
        // max_len=12 fits "line1\nline2\n" (12 chars)
        let chunks = split_message(msg, 12);
        assert_eq!(chunks, vec!["line1\nline2\n", "line3\nline4"]);
    }

    #[test]
    fn splits_long_line_at_char_boundary() {
        // No newlines — must split at char boundary
        let msg = "abcdefghij"; // 10 chars
        let chunks = split_message(msg, 4);
        assert_eq!(chunks, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn handles_multibyte_chars() {
        // 'é' is 2 bytes in UTF-8
        let msg = "aaébb"; // a(1) a(1) é(2) b(1) b(1) = 6 bytes
        let chunks = split_message(msg, 3);
        // Can't split inside 'é', so first chunk is "aa" (2 bytes)
        assert_eq!(chunks[0], "aa");
        assert_eq!(chunks[1], "éb");
        assert_eq!(chunks[2], "b");
    }

    #[test]
    fn empty_message_returns_single_empty_chunk() {
        let chunks = split_message("", 100);
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn realistic_telegram_split() {
        // Simulate a long ad list message
        let mut msg = String::from("📋 All paid placements (50 total):\n");
        for i in 1..=50 {
            msg.push_str(&format!(
                "• Page 1 #{i} [Sponsored Product] — Some Product Title Here\n"
            ));
        }
        let chunks = split_message(&msg, 4000);
        // Should produce multiple chunks
        assert!(!chunks.is_empty());
        // Every chunk should be within limit
        for chunk in &chunks {
            assert!(chunk.len() <= 4000, "Chunk too long: {} bytes", chunk.len());
        }
        // Reassembled content should equal original
        let reassembled: String = chunks.concat();
        assert_eq!(reassembled, msg);
    }
}
