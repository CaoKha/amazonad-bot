use mts_common::{notifier, state};
use mts_scraper::{amazon_scraper, bot, config, monitor};

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;
use tracing::info;

/// Sentinel file used by `stop` subcommand to signal a running daemon.
const SENTINEL_FILE: &str = ".mts-stop";

#[derive(Parser)]
#[command(
    name = "mts-scraper",
    about = "Monitors Amazon marketplaces for sponsored ads (scraper mode)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run,
    CheckNow,
    DryRun,
    /// Signal a running daemon to stop gracefully
    Stop,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mts_scraper=info".parse().unwrap())
                .add_directive("mts_common=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Run => cmd_run().await,
        Commands::CheckNow => cmd_check_now().await,
        Commands::DryRun => cmd_dry_run().await,
        Commands::Stop => cmd_stop(),
    }
}

/// Write a sentinel file that the running daemon watches for.
fn cmd_stop() -> anyhow::Result<()> {
    let path = PathBuf::from(SENTINEL_FILE);
    std::fs::write(&path, b"stop")?;
    println!("Stop signal sent. The running daemon will shut down shortly.");
    Ok(())
}

async fn cmd_run() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    let config = Arc::new(app_config);
    let http_client = reqwest::Client::new();
    let scraper = Arc::new(amazon_scraper::AmazonScraper::new(Arc::new(
        config.scraper.clone(),
    ))?);
    let state_manager = Arc::new(state::StateManager::new(PathBuf::from("state.json")));
    let telegram_configs = Arc::new(config.telegram.clone());

    // Connect to Postgres if DATABASE_URL is configured
    let db_pool = match config.database_url() {
        Some(url) => {
            info!("Connecting to Postgres...");
            match mts_common::db::connect(&url).await {
                Ok(pool) => {
                    info!("Postgres connected. Running migrations...");
                    if let Err(e) = mts_common::db::run_migrations(&pool).await {
                        tracing::warn!("Migration failed (continuing without DB): {:#}", e);
                        None
                    } else {
                        info!("Migrations complete.");
                        Some(pool)
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Postgres connection failed (continuing without DB): {:#}",
                        e
                    );
                    None
                }
            }
        }
        None => {
            info!("No DATABASE_URL configured — running without Postgres persistence.");
            None
        }
    };

    let cancel_token = CancellationToken::new();

    let engine = Arc::new(monitor::MonitorEngine::new(
        scraper.clone(),
        state_manager.clone(),
        http_client.clone(),
        telegram_configs.clone(),
        config.scraper.brand_filter.clone(),
        db_pool,
        cancel_token.clone(),
    ));

    // Bot uses all marketplaces' keywords and URLs for on-demand commands
    let bot_marketplaces: Vec<bot::BotMarketplace> = config
        .scraper
        .marketplaces
        .iter()
        .map(|mp| bot::BotMarketplace {
            code: mp.code.clone(),
            url: mp.url.clone(),
            keywords: mp.keywords.clone(),
        })
        .collect();
    let first_tg = &config.telegram[0];
    let bot_token = std::env::var(&first_tg.bot_token_env)
        .unwrap_or_else(|_| panic!("{} must be set", first_tg.bot_token_env));
    let listener = bot::CommandListener::new(
        bot_token,
        first_tg.chat_id,
        scraper.clone(),
        state_manager.clone(),
        config.scraper.brand_filter.clone(),
        bot_marketplaces,
        cancel_token.clone(),
    );
    tokio::spawn(async move { listener.run().await });

    let total_keywords: usize = config
        .scraper
        .marketplaces
        .iter()
        .map(|m| m.keywords.len())
        .sum();
    info!(
        "Starting monitoring loop ({} marketplaces, {} total keywords, interval: {} min)",
        config.scraper.marketplaces.len(),
        total_keywords,
        config.monitoring.interval_minutes
    );

    // Clean up any stale sentinel file from a previous run
    let _ = std::fs::remove_file(SENTINEL_FILE);

    let mut interval =
        tokio::time::interval(Duration::from_secs(config.monitoring.interval_minutes * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Spawn signal handler — cancels token on SIGINT/SIGTERM
    let cancel_for_signal = cancel_token.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        cancel_for_signal.cancel();
    });

    // Spawn sentinel file watcher — cancels token when .mts-stop appears
    let cancel_for_sentinel = cancel_token.clone();
    tokio::spawn(async move {
        let sentinel = PathBuf::from(SENTINEL_FILE);
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if sentinel.exists() {
                info!("Stop sentinel file detected. Initiating shutdown.");
                std::fs::remove_file(&sentinel).ok();
                cancel_for_sentinel.cancel();
                break;
            }
        }
    });

    // Force-exit watchdog: if graceful shutdown takes >30s after cancellation, force kill.
    // This prevents indefinite hangs from stuck browser sessions or network timeouts.
    let cancel_for_watchdog = cancel_token.clone();
    tokio::spawn(async move {
        cancel_for_watchdog.cancelled().await;
        tokio::time::sleep(Duration::from_secs(30)).await;
        tracing::warn!("Graceful shutdown timed out after 30s. Forcing exit.");
        std::process::exit(1);
    });

    loop {
        if cancel_token.is_cancelled() {
            break;
        }

        // Wait for either the next tick OR cancellation.
        // This select only covers the WAIT period — not the sweep itself.
        tokio::select! {
            _ = interval.tick() => {}
            _ = cancel_token.cancelled() => {
                info!("Shutdown received while waiting for next sweep.");
                break;
            }
        }

        if cancel_token.is_cancelled() {
            break;
        }

        // Run sweep. Cancellation is checked between keywords inside
        // run_check_marketplace, and between marketplaces here.
        for marketplace in &config.scraper.marketplaces {
            if cancel_token.is_cancelled() {
                break;
            }
            match engine.run_check_marketplace(marketplace).await {
                Ok(()) => info!("Sweep complete for {}", marketplace.code),
                Err(e) => tracing::error!("Sweep failed for {}: {:#}", marketplace.code, e),
            }
        }
    }

    info!("Shutdown complete.");
    Ok(())
}

async fn cmd_check_now() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    let config = Arc::new(app_config);
    let http_client = reqwest::Client::new();
    let scraper = Arc::new(amazon_scraper::AmazonScraper::new(Arc::new(
        config.scraper.clone(),
    ))?);
    let state_manager = Arc::new(state::StateManager::new(PathBuf::from("state.json")));
    let telegram_configs = Arc::new(config.telegram.clone());

    // Connect to Postgres if DATABASE_URL is configured
    let db_pool = match config.database_url() {
        Some(url) => {
            info!("Connecting to Postgres...");
            match mts_common::db::connect(&url).await {
                Ok(pool) => {
                    if let Err(e) = mts_common::db::run_migrations(&pool).await {
                        tracing::warn!("Migration failed (continuing without DB): {:#}", e);
                        None
                    } else {
                        Some(pool)
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Postgres connection failed (continuing without DB): {:#}",
                        e
                    );
                    None
                }
            }
        }
        None => None,
    };

    let cancel_token = CancellationToken::new();
    let cancel_for_signal = cancel_token.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        cancel_for_signal.cancel();
    });

    let engine = monitor::MonitorEngine::new(
        scraper,
        state_manager,
        http_client,
        telegram_configs,
        config.scraper.brand_filter.clone(),
        db_pool,
        cancel_token.clone(),
    );

    for marketplace in &config.scraper.marketplaces {
        if cancel_token.is_cancelled() {
            break;
        }
        engine.run_check_marketplace(marketplace).await?;
    }
    info!("Check complete");

    Ok(())
}

async fn cmd_dry_run() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    let total_keywords: usize = app_config
        .scraper
        .marketplaces
        .iter()
        .map(|m| m.keywords.len())
        .sum();
    info!(
        "Config loaded: OK ({} marketplaces, {} total keywords)",
        app_config.scraper.marketplaces.len(),
        total_keywords
    );

    let config = Arc::new(app_config);

    // Use first marketplace's first keyword for the test message URL
    let first_mp = config
        .scraper
        .marketplaces
        .first()
        .expect("at least one marketplace required");
    let first_kw = first_mp
        .keywords
        .first()
        .map(|s| s.as_str())
        .unwrap_or("test");
    let search_url = amazon_scraper::AmazonScraper::build_search_url(&first_mp.url, first_kw, 1);
    for (i, tg) in config.telegram.iter().enumerate() {
        let notifier = notifier::TelegramNotifier::new(
            tg,
            reqwest::Client::new(),
            first_kw.to_string(),
            search_url.clone(),
        )?;
        notifier.send_test_message().await?;
        info!("Telegram target {} (chat_id={}): OK", i + 1, tg.chat_id);
    }

    info!("\nAll checks passed. Ready to run: cargo run -p mts-scraper -- run");

    Ok(())
}

/// Wait for a process termination signal (SIGINT or SIGTERM).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("SIGINT received. Shutting down.");
            }
            _ = sigterm.recv() => {
                info!("SIGTERM received. Shutting down.");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received.");
    }
}
