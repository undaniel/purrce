mod config;
mod api;
mod domain;
mod infrastructure;
mod application;
mod scheduler;

use config::AppConfig;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "purrce_backend=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let profile = infrastructure::scraper::ScrapeProfile::from_env();
    infrastructure::scraper::set_profile(profile);
    tracing::info!("Scraper profile: {:?}", profile);

    // Restore cookies (e.g. a previously solved cf_clearance) and keep them in sync.
    if let Err(e) = infrastructure::scraper::cookies::load(&pool).await {
        tracing::warn!("Failed to load persisted cookies: {}", e);
    }
    let cookie_pool = pool.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tick.tick().await;
            if let Err(e) = infrastructure::scraper::cookies::flush(&cookie_pool).await {
                tracing::warn!("Failed to persist cookies: {}", e);
            }
        }
    });

    let (app, events_tx) = api::create_router(pool.clone(), config.default_currency.clone());

    let scheduler_pool = pool.clone();
    let scheduler_interval = std::env::var("SCHEDULER_INTERVAL")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .unwrap_or(30);
    let scheduler_currency = config.default_currency.clone();
    tokio::spawn(async move {
        scheduler::run(scheduler_pool, scheduler_interval, scheduler_currency, events_tx).await;
    });

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.bind_addr, config.port)).await?;
    tracing::info!("Server running on {}:{}", config.bind_addr, config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
