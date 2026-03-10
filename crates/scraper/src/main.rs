use mts_common::{notifier, state};
use mts_scraper::{amazon_scraper, bot, config, monitor};

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::info;

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
    }
}

async fn cmd_run() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    let config = Arc::new(app_config);
    let http_client = reqwest::Client::new();
    let scraper = Arc::new(amazon_scraper::AmazonScraper::new(Arc::new(
        config.scraper.clone(),
    ))?);
    let state_manager = Arc::new(state::StateManager::new(PathBuf::from("state.json")));
    let telegram_config = Arc::new(config.telegram.clone());

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

    let shutdown_flag = Arc::new(AtomicBool::new(false));

    let engine = Arc::new(monitor::MonitorEngine::new(
        scraper.clone(),
        state_manager.clone(),
        http_client.clone(),
        telegram_config,
        config.scraper.brand_filter.clone(),
        db_pool,
        shutdown_flag.clone(),
    ));

    // Bot uses first marketplace's keywords and URL for on-demand commands
    let first_mp = config
        .scraper
        .marketplaces
        .first()
        .expect("at least one marketplace required (validated in load_config)");
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let listener = bot::CommandListener::new(
        bot_token,
        config.telegram.chat_id,
        scraper.clone(),
        state_manager.clone(),
        config.scraper.brand_filter.clone(),
        first_mp.keywords.clone(),
        first_mp.url.clone(),
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

    let mut interval =
        tokio::time::interval(Duration::from_secs(config.monitoring.interval_minutes * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let shutdown = shutdown_signal(shutdown_flag.clone());
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                for marketplace in &config.scraper.marketplaces {
                    if shutdown_flag.load(Ordering::Relaxed) { break; }
                    match engine.run_check_marketplace(marketplace).await {
                        Ok(()) => info!("Sweep complete for {}", marketplace.code),
                        Err(e) => tracing::error!("Sweep failed for {}: {:#}", marketplace.code, e),
                    }
                }
            }
            _ = &mut shutdown => {
                break;
            }
        }
    }

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
    let telegram_config = Arc::new(config.telegram.clone());

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

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = shutdown_flag.clone();
    tokio::spawn(async move {
        shutdown_signal(flag_clone).await;
    });

    let engine = monitor::MonitorEngine::new(
        scraper,
        state_manager,
        http_client,
        telegram_config,
        config.scraper.brand_filter.clone(),
        db_pool,
        shutdown_flag.clone(),
    );

    for marketplace in &config.scraper.marketplaces {
        if shutdown_flag.load(Ordering::Relaxed) {
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
    let notifier = notifier::TelegramNotifier::new(
        &config.telegram,
        reqwest::Client::new(),
        first_kw.to_string(),
        search_url,
    )?;
    notifier.send_test_message().await?;
    info!("Telegram: OK");

    info!("\nAll checks passed. Ready to run: cargo run -p mts-scraper -- run");

    Ok(())
}

async fn shutdown_signal(flag: Arc<AtomicBool>) {
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
    flag.store(true, Ordering::SeqCst);
}
