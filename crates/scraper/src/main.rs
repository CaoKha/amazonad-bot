use mts_scraper::{amazon_scraper, bot, config, monitor};
use mts_common::{notifier, state};

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(
    name = "mts-scraper",
    about = "Monitors amazon.fr for Huawei smartwatch sponsored ads (scraper mode)"
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
    let scraper = Arc::new(amazon_scraper::AmazonScraper::new(Arc::new(config.scraper.clone()))?);
    let state_manager = Arc::new(state::StateManager::new(PathBuf::from("state.json")));
    let telegram_config = Arc::new(config.telegram.clone());

    let engine = monitor::MonitorEngine::new(
        scraper.clone(),
        state_manager.clone(),
        http_client.clone(),
        telegram_config,
        config.scraper.brand_filter.clone(),
        config.scraper.keywords.clone(),
        config.scraper.marketplace_url.clone(),
    );

    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set");
    let listener = bot::CommandListener::new(
        bot_token,
        config.telegram.chat_id,
        scraper.clone(),
        state_manager.clone(),
        config.scraper.brand_filter.clone(),
        config.scraper.keywords.clone(),
        config.scraper.marketplace_url.clone(),
    );
    tokio::spawn(async move { listener.run().await });

    info!(
        "Starting monitoring loop ({} keywords, interval: {} min)",
        config.scraper.keywords.len(),
        config.monitoring.interval_minutes
    );

    let mut interval =
        tokio::time::interval(Duration::from_secs(config.monitoring.interval_minutes * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match engine.run_check().await {
                    Ok(()) => info!("Sweep complete"),
                    Err(e) => tracing::error!("Sweep failed: {e:#}"),
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
    let scraper = Arc::new(amazon_scraper::AmazonScraper::new(Arc::new(config.scraper.clone()))?);
    let state_manager = Arc::new(state::StateManager::new(PathBuf::from("state.json")));
    let telegram_config = Arc::new(config.telegram.clone());

    let engine = monitor::MonitorEngine::new(
        scraper,
        state_manager,
        http_client,
        telegram_config,
        config.scraper.brand_filter.clone(),
        config.scraper.keywords.clone(),
        config.scraper.marketplace_url.clone(),
    );

    engine.run_check().await?;
    info!("Check complete");

    Ok(())
}

async fn cmd_dry_run() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    info!("Config loaded: OK ({} keywords)", app_config.scraper.keywords.len());

    let config = Arc::new(app_config);

    // Use first keyword for the test message URL
    let first_kw = config.scraper.keywords.first().map(|s| s.as_str()).unwrap_or("test");
    let search_url = amazon_scraper::AmazonScraper::build_search_url(
        &config.scraper.marketplace_url,
        first_kw,
        1,
    );
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


async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())
            .expect("failed to register SIGTERM handler");
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