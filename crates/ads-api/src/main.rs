use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mts_ads_api=info".parse().unwrap())
                .add_directive("mts_common=info".parse().unwrap()),
        )
        .init();

    info!("mts-ads-api: not yet implemented");
    info!("This crate will use Amazon Ads API instead of web scraping.");
    eprintln!("TODO: implement Amazon Ads API client");
    std::process::exit(1);
}
