-- Scrape run metadata: one row per (marketplace, keyword, scrape_time).
CREATE TABLE scrape_runs (
    id          SERIAL PRIMARY KEY,
    marketplace VARCHAR(5)   NOT NULL,           -- 'FR', 'DE', 'ES'
    keyword     TEXT         NOT NULL,
    scraped_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    pages_scraped   INT      NOT NULL,
    total_results   INT      NOT NULL,
    sponsored_count INT      NOT NULL,
    brand_match_count INT    NOT NULL
);

-- Individual search results: every product seen on every scrape.
CREATE TABLE search_results (
    id              BIGSERIAL PRIMARY KEY,
    run_id          INT          NOT NULL REFERENCES scrape_runs(id) ON DELETE CASCADE,
    asin            VARCHAR(20)  NOT NULL,
    title           TEXT         NOT NULL,
    brand           TEXT,
    position        INT          NOT NULL,
    page            INT          NOT NULL,
    position_in_page INT         NOT NULL,
    is_sponsored    BOOLEAN      NOT NULL DEFAULT FALSE,
    placement_type  VARCHAR(50),
    price           TEXT,
    rating          REAL,
    review_count    INT,
    is_prime        BOOLEAN      NOT NULL DEFAULT FALSE,
    badge           VARCHAR(50)
);

-- Query patterns: "ad strategy for keyword X on marketplace Y over time"
CREATE INDEX idx_runs_marketplace     ON scrape_runs(marketplace);
CREATE INDEX idx_runs_keyword         ON scrape_runs(keyword);
CREATE INDEX idx_runs_scraped_at      ON scrape_runs(scraped_at);
CREATE INDEX idx_runs_mkt_kw_time     ON scrape_runs(marketplace, keyword, scraped_at);

CREATE INDEX idx_results_run_id       ON search_results(run_id);
CREATE INDEX idx_results_asin         ON search_results(asin);
CREATE INDEX idx_results_sponsored    ON search_results(is_sponsored) WHERE is_sponsored = TRUE;
