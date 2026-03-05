# monitoring-the-situation

## What This Does

Watches amazon.fr sponsored ad placements for a configured brand (default: Huawei smartwatches). When the brand appears or disappears from sponsored results, it fires a Telegram alert.

Two independent monitoring modes, same Telegram alerts:

| Mode | Crate | How it works |
|------|-------|-------------|
| **Scraper** | `mts-scraper` | Launches headless Chrome, scrapes the public search results page. No Amazon account needed. |
| **Ads API** | `mts-ads-api` | Calls the Amazon Advertising API directly. Requires an Amazon Ads partner account. More reliable, no CAPTCHA risk. |

---

## Prerequisites

- **Rust** — install with:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Telegram account** (free, any device)
- **24/7 host** — your System76 laptop running Pop!_OS, or an Oracle Cloud Always Free VM
- **Chrome or Chromium** (scraper mode only)

---

## Telegram Bot Setup

1. Open Telegram and search for **@BotFather**, then tap Start.
2. Send `/newbot`, choose a display name, then choose a username ending in `bot`.
3. BotFather gives you a token like `123456:ABCdef...`. Save it.
4. Find your **chat_id**:

   ```bash
   # First, send any message to your new bot in Telegram. Then:
   curl https://api.telegram.org/botYOUR_TOKEN/getUpdates
   # Look for: {"chat":{"id": 123456789, ...}}
   # That number is your chat_id.
   ```

---

## Quick Start — Scraper Mode

```bash
# 1. Clone and build
git clone https://github.com/YOUR_USERNAME/monitoring-the-situation.git
cd monitoring-the-situation
cargo build --release

# 2. Configure
cp .env.example .env
cp config.toml.example config.toml
# Edit .env: set TELEGRAM_BOT_TOKEN
# Edit config.toml: set telegram.chat_id

# 3. Validate config and test Telegram
cargo run -p mts-scraper -- dry-run

# 4. Run
cargo run -p mts-scraper -- run
```

---

## Quick Start — Ads API Mode

Complete the [Amazon Ads API Setup](#amazon-ads-api-setup) section below first to get your credentials, then:

```bash
# 1. Configure (in addition to .env with TELEGRAM_BOT_TOKEN)
# Edit config.toml: fill in the [ads_api] section

# 2. Run
cargo run -p mts-ads-api -- run
```

---

## Amazon Ads API Setup

### Overview

The Amazon Advertising API uses OAuth2. You need four values:

| Value | What it is |
|-------|-----------|
| `client_id` | Your app's public identifier |
| `client_secret` | Your app's private key |
| `refresh_token` | Long-lived token granting API access to your account |
| `profile_id` | The numeric ID for your amazon.fr advertising profile |

### Step 1 — Register a Login with Amazon app

1. Go to [https://developer.amazon.com/loginwithamazon/console/site/lwa/overview.html](https://developer.amazon.com/loginwithamazon/console/site/lwa/overview.html)
2. Click **Create a New Security Profile**.
3. Fill in any name and description (e.g. "MTS Monitor").
4. Under **Web Settings**, add `https://localhost` as an **Allowed Return URL**.
5. Save. You will see your **Client ID** and **Client Secret** — copy both.

### Step 2 — Authorize your Amazon Ads account

Open this URL in your browser (replace `YOUR_CLIENT_ID`):

```
https://www.amazon.com/ap/oa?client_id=YOUR_CLIENT_ID&scope=advertising::campaign_management&response_type=code&redirect_uri=https://localhost&state=mts
```

Log in with the Amazon account that has your Ads partner access. After authorization, your browser will redirect to a URL like:

```
https://localhost/?code=ANBXxxxxxxxxxxx&scope=...&state=mts
```

Copy the `code=` value. **It expires in 5 minutes.**

### Step 3 — Exchange the code for a refresh token

```bash
curl -X POST https://api.amazon.com/auth/o2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=ANBXxxxxxxxxxxx" \
  -d "redirect_uri=https://localhost" \
  -d "client_id=YOUR_CLIENT_ID" \
  -d "client_secret=YOUR_CLIENT_SECRET"
```

The response contains a `refresh_token`. Copy it — this is the long-lived token (does not expire unless revoked).

```json
{
  "access_token": "Atza|...",
  "refresh_token": "Atzr|...",
  "token_type": "bearer",
  "expires_in": 3600
}
```

### Step 4 — Find your profile ID

```bash
curl -X GET https://advertising-api-eu.amazon.com/v2/profiles \
  -H "Amazon-Advertising-API-ClientId: YOUR_CLIENT_ID" \
  -H "Authorization: Bearer ACCESS_TOKEN_FROM_ABOVE"
```

Look for the profile where `"countryCode": "FR"`. The `profileId` field is your `profile_id`.

```json
[
  {
    "profileId": 1234567890,
    "countryCode": "FR",
    "currencyCode": "EUR",
    "accountInfo": { "marketplaceStringId": "A13V1IB3VIYZZH" }
  }
]
```

### Step 5 — Configure config.toml

Add an `[ads_api]` section:

```toml
[ads_api]
client_id     = "amzn1.application-oa2-client.xxxxxxxxxxxx"
client_secret = "amzn1.oa2-cs.v1.xxxxxxxxxxxx"
refresh_token = "Atzr|xxxxxxxxxxxx"
profile_id    = "1234567890"
marketplace   = "FR"
brand_filter  = "huawei"

[telegram]
chat_id = YOUR_CHAT_ID

[monitoring]
interval_minutes = 5
```

---

## Configuration Reference

### `.env`

| Field | Description |
|-------|-------------|
| `TELEGRAM_BOT_TOKEN` | Telegram bot token from BotFather |

### `config.toml` — Scraper mode

| Field | Default | Description |
|-------|---------|-------------|
| `scraper.keyword` | `"montre connectee"` | Search keyword on amazon.fr |
| `scraper.marketplace_url` | `"https://www.amazon.fr"` | Amazon marketplace base URL |
| `scraper.brand_filter` | `"huawei"` | Brand to detect in sponsored results (case-insensitive) |
| `scraper.pages` | `3` | Number of result pages to scrape (1–7) |
| `telegram.chat_id` | required | Your Telegram user ID |
| `monitoring.interval_minutes` | `5` | How often to check (minimum 5) |

### `config.toml` — Ads API mode

| Field | Required | Description |
|-------|----------|-------------|
| `ads_api.client_id` | yes | Login with Amazon app client ID |
| `ads_api.client_secret` | yes | Login with Amazon app client secret |
| `ads_api.refresh_token` | yes | OAuth2 refresh token from Step 3 above |
| `ads_api.profile_id` | yes | Numeric profile ID for amazon.fr from Step 4 above |
| `ads_api.marketplace` | yes | Country code, e.g. `"FR"` |
| `ads_api.brand_filter` | yes | Brand to detect (case-insensitive) |
| `telegram.chat_id` | yes | Your Telegram user ID |
| `monitoring.interval_minutes` | yes | How often to check (minimum 5) |

---

## CLI Commands

### Scraper mode

```bash
cargo run -p mts-scraper -- run        # Start the monitoring daemon
cargo run -p mts-scraper -- check-now  # Run a single check, then exit
cargo run -p mts-scraper -- dry-run    # Validate config and send a test Telegram message
RUST_LOG=debug cargo run -p mts-scraper -- run  # Verbose logging
```

### Ads API mode

```bash
cargo run -p mts-ads-api -- run        # Start the monitoring daemon
```

---

## Telegram Bot Commands

The scraper daemon runs a bot that responds to commands in your chat:

| Command | Description |
|---------|-------------|
| `/status` | Current monitoring state (is Huawei visible, last check time) |
| `/check` | Scrape amazon.fr right now and show result |
| `/list` | List all sponsored products currently on the page |
| `/filter samsung` | Filter sponsored products by brand name |

---

## Deploy 24/7 (Two Options)

### Option A — System76 Laptop with Pop!_OS (Recommended)

Full guide: `deploy/POPOS_SETUP.md`

```bash
make setup-local
scp .env user@laptop:/opt/ads-monitor/.env
scp config.toml user@laptop:/opt/ads-monitor/config.toml
make deploy-local
make status-local
```

### Option B — Oracle Cloud Free VM

Full guide: `deploy/ORACLE_SETUP.md`

```bash
make setup-cloud
scp .env user@vm:/opt/ads-monitor/.env
scp config.toml user@vm:/opt/ads-monitor/config.toml
make deploy-cloud
make status-cloud
```

---

## How It Works

### Scraper mode

1. Every `interval_minutes`, launches a headless Chrome browser and navigates to `https://www.amazon.fr/s?k=montre+connectee`.
2. Waits for search results to load (handles WAF JS challenges automatically).
3. Parses the HTML for sponsored products: inline `.AdHolder` results and carousel widget JSON payloads.
4. Sponsored results with the configured brand in the title trigger an alert.
5. State is persisted to `state.json` so the daemon only alerts on **changes** (appeared / disappeared), not on every check.

### Ads API mode

1. Every `interval_minutes`, requests an access token using the refresh token.
2. Calls the Amazon Advertising API to retrieve sponsored product placements for the configured profile.
3. Checks whether the configured brand appears in the results.
4. Alerts on changes, same as scraper mode.

---

## Limitations

### Scraper mode
- Amazon may change their HTML structure at any time, breaking the CSS selectors.
- Amazon may serve a CAPTCHA page if requests are too frequent — keep `interval_minutes` at 5 or higher.
- The daemon does not rotate IP addresses. If blocked, wait a few hours before retrying.
- Results may vary by geographic location — run the daemon from a French IP for best accuracy.

### Ads API mode
- Requires an active Amazon Ads partner account.
- API access may be subject to Amazon's rate limits and approval process.
- The refresh token must be re-generated if it is revoked (e.g. after a password change).

---

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `CAPTCHA or bot-detection page detected` | Amazon is rate-limiting | Increase `interval_minutes`, wait a few hours |
| `Amazon returned 503` | Temporary block | Wait and retry |
| `TELEGRAM_BOT_TOKEN not set` | Missing env var | Check your `.env` file |
| Telegram not sending | Wrong `chat_id` | Re-run `getUpdates` (see Telegram Bot Setup) |
| `interval_minutes must be at least 5` | Poll interval too low | Set `interval_minutes = 5` in config.toml |
| `Connection refused` on SSH | Firewall blocking port 22 | Check Oracle Security List |
| `ads_api.client_id must not be empty` | Missing Ads API config | Complete the Amazon Ads API Setup section |
| `invalid_grant` from token exchange | Authorization code expired | Re-run Step 2 — codes expire in 5 minutes |
| Empty `profileId` list | Wrong Amazon account | Log in with the account that has Ads access |
