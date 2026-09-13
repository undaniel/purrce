use std::collections::HashMap;
use std::sync::Arc;

use rand::Rng;
use sqlx::PgPool;
use tokio::sync::{Semaphore, broadcast};
use tokio::task::JoinSet;
use tracing::{error, info, warn};

use crate::api::AppEvent;
use crate::application::watch::check_watch;
use crate::infrastructure::database::repository::{PendingWatch, Repository};

pub async fn run(pool: PgPool, interval_secs: u64, default_currency: String, events: broadcast::Sender<AppEvent>) {
    info!("Scheduler started (interval={}s)", interval_secs);
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        if let Err(e) = check_watches(&pool, &default_currency, &events).await {
            error!("Scheduler tick failed: {}", e);
        }
    }
}

/// How many stores are scraped at the same time. Watches on the SAME store stay
/// sequential so we never hammer one retailer. The cap scales with `PROFILE`.
async fn check_watches(pool: &PgPool, default_currency: &str, events: &broadcast::Sender<AppEvent>) -> Result<(), sqlx::Error> {
    let repo = Repository::new(pool.clone());
    let pending = repo.find_pending_watches_with_offers().await?;

    if pending.is_empty() {
        return Ok(());
    }

    info!("Found {} pending watches", pending.len());

    // Group by domain so we never hammer one retailer in parallel.
    let mut by_domain: HashMap<String, Vec<PendingWatch>> = HashMap::new();
    for row in pending {
        let key = url::Url::parse(&row.url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| format!("invalid:{}", row.watch_id));
        by_domain.entry(key).or_default().push(row);
    }

    let max_domains = crate::infrastructure::scraper::profile().max_concurrent_domains();
    let sem = Arc::new(Semaphore::new(max_domains));
    let mut tasks = JoinSet::new();
    for (_domain, rows) in by_domain {
        let pool = pool.clone();
        let currency = default_currency.to_string();
        let sem = sem.clone();
        let events = events.clone();
        tasks.spawn(async move {
            let _permit = sem.acquire().await;
            let repo = Repository::new(pool);
            for (i, row) in rows.into_iter().enumerate() {
                // Spacing between consecutive hits to the same domain:
                // random 1.5-3s so we never look like a tight loop to anti-bot systems.
                if i > 0 {
                    let delay = rand::rng().random_range(1_500u64..3_000);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
                let (watch, offer, db_needs_js, db_needs_stealth) = row.split();
                if let Err(e) = check_watch(&repo, watch, offer, db_needs_js, db_needs_stealth, &currency, &events).await {
                    warn!("Watch check failed: {}", e);
                }
            }
        });
    }
    while tasks.join_next().await.is_some() {}

    Ok(())
}
