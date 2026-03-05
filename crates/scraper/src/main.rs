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
    let notifier = notifier::TelegramNotifier::new(&config.telegram, http_client.clone())?;
    let engine = monitor::MonitorEngine::new(scraper.clone(), state_manager.clone(), notifier, config.scraper.brand_filter.clone());

    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set");
    let listener = bot::CommandListener::new(
        bot_token,
        config.telegram.chat_id,
        scraper.clone(),
        state_manager.clone(),
        config.scraper.brand_filter.clone(),
    );
    tokio::spawn(async move { listener.run().await });

    info!(
        "Starting monitoring loop (interval: {} min)",
        config.monitoring.interval_minutes
    );

    let mut interval =
        tokio::time::interval(Duration::from_secs(config.monitoring.interval_minutes * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match engine.run_check().await {
                    Ok(outcome) => info!("Check complete: {:?}", outcome),
                    Err(e) => tracing::error!("Check failed: {e:#}"),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Exiting.");
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
    let notifier = notifier::TelegramNotifier::new(&config.telegram, http_client)?;
    let engine = monitor::MonitorEngine::new(scraper, state_manager, notifier, config.scraper.brand_filter.clone());

    let outcome = engine.run_check().await?;
    info!("Check complete: {:?}", outcome);

    Ok(())
}

async fn cmd_dry_run() -> anyhow::Result<()> {
    let app_config = config::load_config()?;
    info!("Config loaded: OK");

    let config = Arc::new(app_config);

    let notifier = notifier::TelegramNotifier::new(&config.telegram, reqwest::Client::new())?;
    notifier.send_test_message().await?;
    info!("Telegram: OK");

    info!("\nAll checks passed. Ready to run: cargo run -p mts-scraper -- run");

    Ok(())
}
