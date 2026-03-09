use mts_scraper::amazon_scraper::AmazonScraper;

const HTML_WITH_HUAWEI_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="sg-col AdHolder">
  <div data-component-type="s-impression-logger">
    <span class="puis-label-popover puis-sponsored-label-text">
      <a href="javascript:void(0)"><span class="a-color-secondary">Sponsored</span></a>
    </span>
    <h2>Huawei Watch GT 4 Montre Connectee</h2>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
</body>
</html>
"#;

const HTML_NO_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
<div data-component-type="s-search-result" data-asin="B0SAMSUNG1">
  <h2>Samsung Galaxy Watch 6</h2>
</div>
</body>
</html>
"#;

const HTML_SPONSORED_NOT_HUAWEI: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001" class="sg-col AdHolder">
  <div data-component-type="s-impression-logger">
    <span class="puis-label-popover puis-sponsored-label-text">
      <a href="javascript:void(0)"><span class="a-color-secondary">Sponsored</span></a>
    </span>
    <h2>Apple Watch Series 9</h2>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0SAMSUNG1">
  <h2>Samsung Galaxy Watch 6</h2>
</div>
</body>
</html>
"#;

const HTML_HUAWEI_ORGANIC: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <h2>Huawei Watch GT 4 Montre Connectee</h2>
</div>
</body>
</html>
"#;

const HTML_MULTIPLE_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001" class="sg-col AdHolder">
  <div data-component-type="s-impression-logger">
    <span class="puis-label-popover puis-sponsored-label-text">
      <a href="javascript:void(0)"><span class="a-color-secondary">Sponsored</span></a>
    </span>
    <h2>Apple Watch Series 9</h2>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0SAMSUNG1">
  <h2>Samsung Galaxy Watch 6</h2>
</div>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="sg-col AdHolder">
  <div data-component-type="s-impression-logger">
    <span class="puis-label-popover puis-sponsored-label-text">
      <a href="javascript:void(0)"><span class="a-color-secondary">Sponsored</span></a>
    </span>
    <h2>Huawei Watch GT 4</h2>
  </div>
</div>
</body>
</html>
"#;

const HTML_EMPTY_RESULTS: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div class="s-no-outline">No results found</div>
</body>
</html>
"#;

const HTML_SPONSORED_TEXT_LABEL: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <h2>Huawei Watch GT 4</h2>
  <span>Sponsored</span>
</div>
</body>
</html>
"#;

const HTML_SPONSORISE_FRENCH: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <h2>Huawei Watch GT 4</h2>
  <span>Sponsoris&#233;</span>
</div>
</body>
</html>
"#;

const HTML_ADHOLDER_SPONSORED: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="AdHolder">
  <h2>Huawei Watch GT 4</h2>
</div>
</body>
</html>
"#;

const HTML_ARIA_LABEL_TITLE_ONLY: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0GGJB61JQ" class="sg-col AdHolder">
  <div data-component-type="s-impression-logger">
    <span class="puis-label-popover puis-sponsored-label-text">
      <a href="javascript:void(0)"><span class="a-color-secondary">Sponsored</span></a>
    </span>
    <h2 aria-label="Sponsored Ad – HUAWEI Band 11 Smart Watches"></h2>
  </div>
</div>
</body>
</html>
"#;

const HTML_PUIS_SPONSORED_ONLY: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01">
  <span class="puis-label-popover puis-sponsored-label-text">
    <span class="a-color-secondary">Sponsored</span>
  </span>
  <h2>Huawei Watch GT 4</h2>
</div>
</body>
</html>
"#;

const HTML_WIDGET_CAROUSEL: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
<span data-action="multi-ad-feedback-form-trigger"
  data-multi-ad-feedback-form-trigger="{&quot;multiAdfPayload&quot;:&quot;{\&quot;adCreativeMetaData\&quot;:{\&quot;adCreativeDetails\&quot;:[{\&quot;asin\&quot;:\&quot;B0ELEJAFE1\&quot;,\&quot;title\&quot;:\&quot;ELEJAFE Montre Connectee Enfant\&quot;},{\&quot;asin\&quot;:\&quot;B0D9JYKXDG\&quot;,\&quot;title\&quot;:\&quot;HUAWEI Watch GT 5 46mm\&quot;}]},\&quot;adPlacementMetaData\&quot;:{\&quot;slotName\&quot;:\&quot;sp_search_thematic-recently_rated_ww\&quot;}}&quot;}">
  <a class="s-widget-sponsored-label-text">Sponsored</a>
</span>
</body>
</html>
"#;

#[test]
fn detects_huawei_sponsored_result() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");

    assert!(
        result.huawei_sponsored_found,
        "Should detect Huawei sponsored ad"
    );
    assert_eq!(result.huawei_sponsored_positions, vec![1]);
    assert_eq!(result.results.len(), 2);
}

#[test]
fn no_sponsored_results() {
    let result = AmazonScraper::parse_results(HTML_NO_SPONSORED, "huawei");

    assert!(!result.huawei_sponsored_found);
    assert!(result.huawei_sponsored_positions.is_empty());
    assert_eq!(result.results.len(), 2);
}

#[test]
fn sponsored_but_not_huawei() {
    let result = AmazonScraper::parse_results(HTML_SPONSORED_NOT_HUAWEI, "huawei");

    assert!(
        !result.huawei_sponsored_found,
        "Apple sponsored should not trigger Huawei detection"
    );
    assert!(result.huawei_sponsored_positions.is_empty());
    let apple = result
        .results
        .iter()
        .find(|r| r.asin == "B0APPLE001")
        .unwrap();
    assert!(apple.is_sponsored);
}

#[test]
fn huawei_organic_not_detected_as_sponsored() {
    let result = AmazonScraper::parse_results(HTML_HUAWEI_ORGANIC, "huawei");

    assert!(
        !result.huawei_sponsored_found,
        "Organic Huawei result should not trigger alert"
    );
    assert!(result.huawei_sponsored_positions.is_empty());
    let huawei = result
        .results
        .iter()
        .find(|r| r.asin == "B0HUAWEI01")
        .unwrap();
    assert!(!huawei.is_sponsored);
}

#[test]
fn multiple_sponsored_huawei_at_position_3() {
    let result = AmazonScraper::parse_results(HTML_MULTIPLE_SPONSORED, "huawei");

    assert!(result.huawei_sponsored_found);
    assert_eq!(result.huawei_sponsored_positions, vec![3]);
    assert_eq!(result.results.len(), 3);
}

#[test]
fn empty_results_page() {
    let result = AmazonScraper::parse_results(HTML_EMPTY_RESULTS, "huawei");

    assert!(!result.huawei_sponsored_found);
    assert!(result.results.is_empty());
}

#[test]
fn brand_filter_case_insensitive_upper_filter() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "HUAWEI");
    assert!(
        result.huawei_sponsored_found,
        "Brand filter should be case-insensitive"
    );
}

#[test]
fn brand_filter_case_insensitive_upper_title() {
    let html_upper = HTML_WITH_HUAWEI_SPONSORED
        .replace("Huawei Watch GT 4 Montre Connectee", "HUAWEI WATCH GT 4");
    let result = AmazonScraper::parse_results(&html_upper, "huawei");
    assert!(
        result.huawei_sponsored_found,
        "Title matching should be case-insensitive"
    );
}

#[test]
fn english_sponsored_label_detected() {
    let result = AmazonScraper::parse_results(HTML_SPONSORED_TEXT_LABEL, "huawei");
    assert!(
        result.huawei_sponsored_found,
        "English 'Sponsored' label should be detected"
    );
}

#[test]
fn french_sponsorise_label_detected() {
    let result = AmazonScraper::parse_results(HTML_SPONSORISE_FRENCH, "huawei");
    assert!(
        result.huawei_sponsored_found,
        "French 'Sponsorise' label should be detected"
    );
}

#[test]
fn adholder_class_detected_as_sponsored() {
    let result = AmazonScraper::parse_results(HTML_ADHOLDER_SPONSORED, "huawei");
    let huawei = result
        .results
        .iter()
        .find(|r| r.asin == "B0HUAWEI01")
        .unwrap();
    assert!(
        huawei.is_sponsored,
        "AdHolder class should mark result as sponsored"
    );
    assert!(result.huawei_sponsored_found);
}

#[test]
fn positions_are_1_based() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");
    assert_eq!(result.results[0].position, 1);
    assert_eq!(result.results[1].position, 2);
}

#[test]
fn build_search_url_encodes_spaces() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr", "montre connectee", 1);
    assert_eq!(url, "https://www.amazon.fr/s?k=montre+connectee");
}

#[test]
fn build_search_url_trims_trailing_slash() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr/", "montre", 1);
    assert_eq!(url, "https://www.amazon.fr/s?k=montre");
}

#[test]
fn results_have_correct_asins() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");
    assert_eq!(result.results[0].asin, "B0HUAWEI01");
    assert_eq!(result.results[1].asin, "B0APPLE001");
}

#[test]
fn results_have_titles() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");
    assert!(
        result.results[0].title.contains("Huawei"),
        "First result should have Huawei in title"
    );
    assert!(
        result.results[1].title.contains("Apple"),
        "Second result should have Apple in title"
    );
}

#[test]
fn parse_results_with_offset_renumbers_globally() {
    let result =
        AmazonScraper::parse_results_with_offset(HTML_WITH_HUAWEI_SPONSORED, "huawei", 10, 1);
    assert_eq!(result.results[0].position, 11);
    assert_eq!(result.results[1].position, 12);
    assert_eq!(result.huawei_sponsored_positions, vec![11]);
}

#[test]
fn parse_results_returns_page_relative_positions() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");
    assert_eq!(result.results[0].position, 1);
    assert_eq!(result.results[1].position, 2);
}

#[test]
fn parse_results_sets_page_and_position_in_page() {
    let result = AmazonScraper::parse_results(HTML_WITH_HUAWEI_SPONSORED, "huawei");
    let first = &result.results[0];
    assert_eq!(
        first.page, 1,
        "parse_results wrapper should default to page 1"
    );
    assert_eq!(
        first.position_in_page, 1,
        "first result should have position_in_page = 1"
    );

    let result2 =
        AmazonScraper::parse_results_with_offset(HTML_WITH_HUAWEI_SPONSORED, "huawei", 0, 2);
    assert_eq!(
        result2.results[0].page, 2,
        "page param should be set on results"
    );
    assert_eq!(
        result2.results[0].position_in_page, 1,
        "position_in_page resets per page"
    );
}

#[test]
fn build_search_url_page_7() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr", "test", 7);
    assert_eq!(url, "https://www.amazon.fr/s?k=test&page=7");
}

#[test]
fn build_search_url_page_2() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr", "montre connectee", 2);
    assert_eq!(url, "https://www.amazon.fr/s?k=montre+connectee&page=2");
}

#[test]
fn build_search_url_page_1_no_page_param() {
    let url = AmazonScraper::build_search_url("https://www.amazon.fr", "montre", 1);
    assert!(
        !url.contains("page="),
        "Page 1 URL should not contain page param, got: {url}"
    );
}

#[test]
fn aria_label_title_fallback() {
    let result = AmazonScraper::parse_results(HTML_ARIA_LABEL_TITLE_ONLY, "huawei");
    assert!(
        result.huawei_sponsored_found,
        "Should detect Huawei via aria-label title fallback"
    );
    let hw = result
        .results
        .iter()
        .find(|r| r.asin == "B0GGJB61JQ")
        .unwrap();
    assert!(hw.is_sponsored);
    assert!(
        hw.title.contains("HUAWEI Band 11"),
        "Title should be extracted from aria-label, got: {}",
        hw.title
    );
    assert!(
        !hw.title.starts_with("Sponsored"),
        "Sponsored Ad prefix should be stripped from title, got: {}",
        hw.title
    );
}

#[test]
fn puis_sponsored_class_only_detection() {
    let result = AmazonScraper::parse_results(HTML_PUIS_SPONSORED_ONLY, "huawei");
    assert!(
        result.huawei_sponsored_found,
        "Should detect sponsored via puis-sponsored-label-text class in inner HTML"
    );
    let hw = result
        .results
        .iter()
        .find(|r| r.asin == "B0HUAWEI01")
        .unwrap();
    assert!(hw.is_sponsored);
}

#[test]
fn widget_carousel_products_are_listed_as_sponsored() {
    let result = AmazonScraper::parse_results(HTML_WIDGET_CAROUSEL, "huawei");

    let elejafe = result
        .results
        .iter()
        .find(|r| r.asin == "B0ELEJAFE1")
        .unwrap();
    assert!(
        elejafe.is_sponsored,
        "Carousel product must be marked sponsored"
    );
    assert!(
        elejafe.title.contains("ELEJAFE"),
        "Title must come from widget JSON"
    );
    assert_eq!(elejafe.position, 0, "Widget sentinel position must be 0");
    assert_eq!(
        elejafe.position_in_page, 0,
        "Widget sentinel position_in_page must be 0"
    );

    let huawei = result
        .results
        .iter()
        .find(|r| r.asin == "B0D9JYKXDG")
        .unwrap();
    assert!(huawei.is_sponsored);
    assert!(huawei.title.contains("HUAWEI"));

    let apple = result
        .results
        .iter()
        .find(|r| r.asin == "B0APPLE001")
        .unwrap();
    assert!(
        !apple.is_sponsored,
        "Organic inline product must not be marked sponsored"
    );

    assert_eq!(result.results.len(), 3);

    assert!(result.huawei_sponsored_found);
}

// ===== Enrichment field tests =====

const HTML_WITH_PRICE: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="sg-col AdHolder">
  <span class="puis-label-popover puis-sponsored-label-text">
    <span class="a-color-secondary">Sponsored</span>
  </span>
  <h2>Huawei Watch GT 4</h2>
  <span class="a-price">
    <span class="a-offscreen">149,99 €</span>
  </span>
</div>
</body>
</html>
"#;

const HTML_WITH_RATING: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="sg-col AdHolder">
  <span class="puis-label-popover puis-sponsored-label-text">
    <span class="a-color-secondary">Sponsored</span>
  </span>
  <h2>Huawei Watch GT 4</h2>
  <span class="a-icon-alt">4,5 sur 5 étoiles</span>
</div>
</body>
</html>
"#;

const HTML_WITH_BEST_SELLER_BADGE: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0HUAWEI01" class="sg-col AdHolder">
  <span class="puis-label-popover puis-sponsored-label-text">
    <span class="a-color-secondary">Sponsored</span>
  </span>
  <h2>Huawei Watch GT 4</h2>
  <span class="a-badge-text">Meilleur vendeur</span>
</div>
</body>
</html>
"#;

#[test]
fn price_parsed_from_offscreen_span() {
    let result = AmazonScraper::parse_results(HTML_WITH_PRICE, "huawei");
    let huawei = result.results.iter().find(|r| r.asin == "B0HUAWEI01").unwrap();
    assert_eq!(
        huawei.price.as_deref(),
        Some("149,99 €"),
        "Price should be parsed from .a-price .a-offscreen"
    );
}

#[test]
fn rating_parsed_from_icon_alt() {
    let result = AmazonScraper::parse_results(HTML_WITH_RATING, "huawei");
    let huawei = result.results.iter().find(|r| r.asin == "B0HUAWEI01").unwrap();
    assert!(
        huawei.rating.is_some(),
        "Rating should be parsed from span.a-icon-alt"
    );
    let rating = huawei.rating.unwrap();
    assert!(
        (rating - 4.5).abs() < 0.01,
        "Rating should be 4.5, got: {rating}"
    );
}

#[test]
fn best_seller_badge_detected() {
    use mts_common::models::BadgeType;
    let result = AmazonScraper::parse_results(HTML_WITH_BEST_SELLER_BADGE, "huawei");
    let huawei = result.results.iter().find(|r| r.asin == "B0HUAWEI01").unwrap();
    assert_eq!(
        huawei.badge,
        Some(BadgeType::BestSeller),
        "BestSeller badge should be detected from span.a-badge-text"
    );
}


const HTML_SBV_VIDEO_SINGLE_PRODUCT: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
<span data-component-type="sbv-video-single-product">
  <span class="sponsored-brand-label-info-desktop">Sponsored</span>
  <h2>Soft Silicone Strap for Huawei Band 8/9/10</h2>
  <a href="/dp/B0F2ZZDMX7/ref=some_ref">Product Link</a>
</span>
</body>
</html>
"#;

#[test]
fn sbv_video_single_product_detected() {
    let result = AmazonScraper::parse_results(HTML_SBV_VIDEO_SINGLE_PRODUCT, "huawei");

    let sbv = result
        .results
        .iter()
        .find(|r| r.asin == "B0F2ZZDMX7")
        .expect("SBV product should be found via /dp/ link extraction");
    assert!(sbv.is_sponsored, "SBV product must be sponsored");
    assert!(
        sbv.title.contains("Huawei"),
        "Title should contain Huawei, got: {}",
        sbv.title
    );
    assert_eq!(
        sbv.placement_type,
        Some(mts_common::models::PlacementType::SponsoredBrandVideo),
        "Placement type should be SponsoredBrandVideo"
    );

    assert!(
        result.huawei_sponsored_found,
        "Huawei sponsored should be detected in SBV section"
    );
}

const HTML_SBV_VIDEO_LEGACY: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
<div data-component-type="sbv-video" data-asin="B0HUAWEI01">
  <h2>HUAWEI Watch GT 5 Pro</h2>
</div>
</body>
</html>
"#;

#[test]
fn sbv_video_legacy_format_still_works() {
    let result = AmazonScraper::parse_results(HTML_SBV_VIDEO_LEGACY, "huawei");

    let sbv = result
        .results
        .iter()
        .find(|r| r.asin == "B0HUAWEI01")
        .expect("Legacy SBV product should still be detected via data-asin");
    assert!(sbv.is_sponsored);
    assert_eq!(
        sbv.placement_type,
        Some(mts_common::models::PlacementType::SponsoredBrandVideo),
    );
    assert!(result.huawei_sponsored_found);
}

const HTML_BRAND_FIELD_ONLY: &str = r#"
<!DOCTYPE html>
<html>
<body>
<div data-component-type="s-search-result" data-asin="B0GGJB61JQ" class="sg-col AdHolder">
  <div data-component-type="s-impression-logger">
    <span class="puis-label-popover puis-sponsored-label-text">
      <a href="javascript:void(0)"><span class="a-color-secondary">Sponsored</span></a>
    </span>
    <h2>Smart Watch Band 11 Fitness Tracker</h2>
    <span class="a-size-base a-color-secondary">HUAWEI</span>
  </div>
</div>
<div data-component-type="s-search-result" data-asin="B0APPLE001">
  <h2>Apple Watch Series 9</h2>
</div>
</body>
</html>
"#;

#[test]
fn brand_field_matches_even_when_title_has_no_brand() {
    let result = AmazonScraper::parse_results(HTML_BRAND_FIELD_ONLY, "huawei");

    let hw = result
        .results
        .iter()
        .find(|r| r.asin == "B0GGJB61JQ")
        .unwrap();
    assert!(hw.is_sponsored, "AdHolder should mark as sponsored");
    assert!(
        !hw.title.to_lowercase().contains("huawei"),
        "Title should NOT contain huawei for this test case"
    );
    assert_eq!(
        hw.brand.as_deref(),
        Some("HUAWEI"),
        "Brand field should be extracted from span.a-size-base.a-color-secondary"
    );

    assert!(
        result.huawei_sponsored_found,
        "Should detect Huawei sponsored via brand field even when title lacks 'huawei'"
    );
}