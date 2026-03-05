use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rand::Rng;
use scraper::{Html, Selector};
use tracing::{debug, warn};

use crate::config::ScraperConfig;
use mts_common::models::{ScrapeResult, SearchResult};

pub struct AmazonScraper {
    config: Arc<ScraperConfig>,
}

impl AmazonScraper {
    pub fn new(config: Arc<ScraperConfig>) -> Result<Self> {
        Ok(Self { config })
    }

    /// Returns the page-1 search URL (e.g. `https://www.amazon.fr/s?k=montre+connectee`).
    pub fn search_url(&self) -> String {
        Self::build_search_url(&self.config.marketplace_url, &self.config.keyword, 1)
    }

    fn find_chrome(config: &ScraperConfig) -> Result<std::path::PathBuf> {
        if let Some(ref path) = config.chrome_executable {
            let p = std::path::PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
            bail!("chrome_executable path not found: {}", path);
        }
        // macOS
        let mac_path = std::path::PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        );
        if mac_path.exists() {
            return Ok(mac_path);
        }
        // Linux
        for name in &[
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
            "/snap/bin/chromium",
        ] {
            let pb = std::path::PathBuf::from(name);
            if pb.exists() {
                return Ok(pb);
            }
        }
        bail!("Chrome/Chromium not found. Install Chrome or set chrome_executable in config.toml")
    }

    pub async fn scrape_search_page(&self) -> Result<ScrapeResult> {
        use chromiumoxide::browser::{Browser, BrowserConfig};
        use futures::StreamExt;

        let chrome_path = Self::find_chrome(&self.config)?;

        let (mut browser, mut handler) = tokio::time::timeout(
            std::time::Duration::from_secs(45),
            Browser::launch(
                BrowserConfig::builder()
                    .chrome_executable(chrome_path)
                    .no_sandbox()
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build BrowserConfig: {}", e))?,
            ),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Browser launch timed out"))?
        .context("Failed to launch Chrome")?;

        let handle = tokio::spawn(async move {
            while handler.next().await.is_some() {}
        });

        let result = self.do_scrape(&browser).await;

        browser.close().await.ok();
        handle.await.ok();

        result
    }

    async fn do_scrape(&self, browser: &chromiumoxide::Browser) -> Result<ScrapeResult> {
        const MODERN_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
            AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

        let mut all_results: Vec<SearchResult> = Vec::new();
        let mut global_position = 0usize;
        let mut huawei_sponsored_found = false;
        let mut huawei_sponsored_positions: Vec<usize> = Vec::new();

        for page_num in 1..=self.config.pages {
            let url = Self::build_search_url(
                &self.config.marketplace_url,
                &self.config.keyword,
                page_num,
            );
            debug!("Scraping page {}/{}: {}", page_num, self.config.pages, url);

            let page = match tokio::time::timeout(
                std::time::Duration::from_secs(45),
                browser.new_page(&url),
            )
            .await
            {
                Ok(Ok(p)) => p,
                Ok(Err(e)) => {
                    if page_num == 1 {
                        return Err(anyhow::anyhow!(e)).context("Failed to open page");
                    }
                    warn!("Page {} failed to open, stopping: {}", page_num, e);
                    break;
                }
                Err(_) => {
                    if page_num == 1 {
                        bail!("Page navigation timed out");
                    }
                    warn!("Page {} navigation timed out, stopping", page_num);
                    break;
                }
            };

            if let Err(e) = page.enable_stealth_mode_with_agent(MODERN_UA).await {
                warn!("Stealth mode failed on page {}: {}", page_num, e);
            }

            let js_check =
                r#"document.querySelector('div[data-component-type="s-search-result"]') !== null"#;

            let results_appeared = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                async {
                    loop {
                        let found = page
                            .evaluate(js_check)
                            .await
                            .and_then(|v| Ok(v.into_value::<bool>()?))
                            .unwrap_or(false);
                        if found {
                            return true;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                },
            )
            .await
            .unwrap_or(false);

            if !results_appeared {
                let url_after = page
                    .url()
                    .await
                    .unwrap_or_default()
                    .unwrap_or_default();
                let on_amazon = url_after.contains("amazon.fr");
                drop(page);

                if page_num == 1 {
                    if !on_amazon {
                        bail!(
                            "WAF redirected away from amazon.fr to: {}",
                            url_after
                        );
                    }
                    bail!(
                        "No search results appeared within 30s — possible WAF block or CAPTCHA"
                    );
                } else {
                    warn!("Page {} had no results within 30s, stopping", page_num);
                    break;
                }
            }

            let html = match page.content().await {
                Ok(h) => h,
                Err(e) => {
                    drop(page);
                    if page_num == 1 {
                        return Err(anyhow::anyhow!(e)).context("Failed to get page content");
                    }
                    warn!("Page {} content failed: {}", page_num, e);
                    break;
                }
            };
            drop(page);

            let page_result = Self::parse_results_with_offset(
                &html,
                &self.config.brand_filter,
                global_position,
                page_num,
            );

            if page_result.results.is_empty() {
                debug!("Page {} parsed 0 results, stopping", page_num);
                break;
            }

            if page_result.huawei_sponsored_found {
                huawei_sponsored_found = true;
                huawei_sponsored_positions.extend(&page_result.huawei_sponsored_positions);
            }

            global_position += page_result.results.len();
            all_results.extend(page_result.results);

            if page_num < self.config.pages {
                let delay = rand::thread_rng().gen_range(2..=4u64);
                debug!("Waiting {}s before next page...", delay);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }

        huawei_sponsored_positions.dedup();

        debug!(
            "Scraped {} total results across {} page(s), huawei_sponsored_found={}",
            all_results.len(),
            self.config.pages,
            huawei_sponsored_found
        );

        Ok(ScrapeResult {
            results: all_results,
            huawei_sponsored_found,
            huawei_sponsored_positions,
            scraped_at: Utc::now(),
        })
    }

    pub fn parse_results(html: &str, brand_filter: &str) -> ScrapeResult {
        Self::parse_results_with_offset(html, brand_filter, 0, 1)
    }

    pub fn parse_results_with_offset(html: &str, brand_filter: &str, offset: usize, page: u32) -> ScrapeResult {
        use std::sync::LazyLock;

        static RESULT_SEL: LazyLock<Selector> = LazyLock::new(||
            Selector::parse(r#"div[data-component-type="s-search-result"]"#).unwrap());
        static SPONSORED_SEL: LazyLock<Selector> = LazyLock::new(||
            Selector::parse(r#"div[data-component-type="sp-sponsored-result"]"#).unwrap());
        static ADHOLDER_SEL: LazyLock<Selector> = LazyLock::new(||
            Selector::parse(".AdHolder").unwrap());
        static H2_SEL: LazyLock<Selector> = LazyLock::new(||
            Selector::parse("h2").unwrap());
        static SPONSORED_LABEL_SEL: LazyLock<Selector> = LazyLock::new(||
            Selector::parse(".puis-sponsored-label-text").unwrap());
        static FEEDBACK_SEL: LazyLock<Selector> = LazyLock::new(||
            Selector::parse(r#"span[data-action="multi-ad-feedback-form-trigger"]"#).unwrap());

        let document = Html::parse_document(html);
        let brand_lower = brand_filter.to_lowercase();

        let sponsored_asins: HashSet<String> = document
            .select(&SPONSORED_SEL)
            .filter_map(|el| el.value().attr("data-asin").map(String::from))
            .collect();

        let adholder_asins: HashSet<String> = document
            .select(&ADHOLDER_SEL)
            .filter_map(|el| el.value().attr("data-asin").map(String::from))
            .collect();

        debug!(
            "parse page={} result_count={} sp_sponsored_count={} adholder_count={}",
            page,
            document.select(&RESULT_SEL).count(),
            document.select(&SPONSORED_SEL).count(),
            document.select(&ADHOLDER_SEL).count(),
        );
        debug!(
            "adholder_asins: {:?}, sponsored_asins: {:?}",
            adholder_asins.iter().take(10).collect::<Vec<_>>(),
            sponsored_asins.iter().take(10).collect::<Vec<_>>(),
        );

        let mut results = Vec::new();
        let mut position = offset;
        let mut pos_in_page: usize = 0;
        for element in document.select(&RESULT_SEL) {
            let asin = match element.value().attr("data-asin") {
                Some(a) if !a.is_empty() => a.to_string(),
                _ => continue,
            };

            position += 1;
            pos_in_page += 1;

            let title = element
                .select(&H2_SEL)
                .next()
                .map(|h| {
                    let text = h.text().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        text
                    } else {
                        h.value()
                            .attr("aria-label")
                            .unwrap_or("")
                            .trim_start_matches("Sponsored Ad \u{2013} ")
                            .trim_start_matches("Sponsored Ad - ")
                            .to_string()
                    }
                })
                .unwrap_or_default();

            let has_sponsored_text = element.text().any(|t| t.contains("Sponsorisé") || t.contains("Sponsored"));
            let has_sponsored_class = element.select(&SPONSORED_LABEL_SEL).next().is_some();
            let is_sponsored = sponsored_asins.contains(&asin)
                || adholder_asins.contains(&asin)
                || has_sponsored_text
                || has_sponsored_class;

            if pos_in_page <= 5 {
                debug!(
                    "result page={} pos={} asin={} title_len={} sp={} adholder={} text_sp={} class_sp={} => is_sponsored={}",
                    page, pos_in_page, asin, title.len(),
                    sponsored_asins.contains(&asin),
                    adholder_asins.contains(&asin),
                    has_sponsored_text,
                    has_sponsored_class,
                    is_sponsored,
                );
            }

            results.push(SearchResult {
                asin,
                title,
                position,
                page,
                position_in_page: pos_in_page,
                is_sponsored,
            });
        }

        // === Parse thematic sponsored carousel widgets ===
        let already_seen: HashSet<String> = results.iter().map(|r| r.asin.clone()).collect();
        let mut widget_added = 0usize;

        for widget in document.select(&FEEDBACK_SEL) {
            let Some(attr) = widget.value().attr("data-multi-ad-feedback-form-trigger") else {
                continue;
            };
            let outer = match serde_json::from_str::<serde_json::Value>(attr) {
                Ok(v) => v,
                Err(e) => { debug!("widget: outer JSON parse failed: {e}"); continue; }
            };
            let Some(inner_str) = outer["multiAdfPayload"].as_str() else { continue; };
            let inner = match serde_json::from_str::<serde_json::Value>(inner_str) {
                Ok(v) => v,
                Err(e) => { debug!("widget: inner JSON parse failed: {e}"); continue; }
            };
            let Some(ads) = inner["adCreativeMetaData"]["adCreativeDetails"].as_array() else {
                continue;
            };
            debug!(
                "widget: {} ads in slot={}",
                ads.len(),
                inner["adPlacementMetaData"]["slotName"].as_str().unwrap_or("?"),
            );
        for ad in ads {
            let asin = ad["asin"].as_str().unwrap_or("");
            let title = ad["title"].as_str().unwrap_or("").trim().to_string();
            if asin.is_empty() || already_seen.contains(asin) { continue; }
            debug!("widget: added asin={}", asin);
            results.push(SearchResult {
                asin: asin.to_string(),
                title,
                position: 0,
                page,
                position_in_page: 0,
                is_sponsored: true,
            });
            widget_added += 1;
        }
        }
        debug!("parse page={} widget_carousel_added={}", page, widget_added);

        let huawei_sponsored: Vec<&SearchResult> = results
            .iter()
            .filter(|r| r.is_sponsored && r.title.to_lowercase().contains(&brand_lower))
            .collect();

        let huawei_sponsored_found = !huawei_sponsored.is_empty();
        let mut huawei_sponsored_positions: Vec<usize> =
            huawei_sponsored.iter().map(|r| r.position).collect();
        huawei_sponsored_positions.dedup();

        if huawei_sponsored_found {
            warn!(
                "Huawei sponsored ad found at position(s): {:?}",
                huawei_sponsored_positions
            );
        }

        ScrapeResult {
            results,
            huawei_sponsored_found,
            huawei_sponsored_positions,
            scraped_at: Utc::now(),
        }
    }

    pub fn build_search_url(base: &str, keyword: &str, page: u32) -> String {
        let encoded = keyword.replace(' ', "+");
        if page <= 1 {
            format!("{}/s?k={}", base.trim_end_matches('/'), encoded)
        } else {
            format!("{}/s?k={}&page={}", base.trim_end_matches('/'), encoded, page)
        }
    }
}
