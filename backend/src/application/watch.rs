use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

use crate::api::AppEvent;
use crate::domain::store::ProductOffer;
use crate::domain::watch::Watch;
use crate::infrastructure::database::repository::Repository;
use crate::infrastructure::notifications::{self, AlertPayload};
use crate::infrastructure::scraper::{self, ExtractionError};

#[derive(Clone)]
pub struct WatchService {
    repo: Repository,
    default_currency: String,
    events: broadcast::Sender<AppEvent>,
}

impl WatchService {
    pub fn new(repo: Repository, default_currency: String, events: broadcast::Sender<AppEvent>) -> Self {
        Self { repo, default_currency, events }
    }

    pub async fn create(&self, offer_id: Uuid, interval: i32) -> Result<Watch, sqlx::Error> {
        self.repo.create_watch(offer_id, interval).await
    }

    pub async fn update(&self, id: Uuid, enabled: Option<bool>, interval: Option<i32>) -> Result<Watch, sqlx::Error> {
        self.repo.update_watch(id, enabled, interval).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.delete_watch(id).await
    }

    /// Manual "check now". Reuses the exact same logic as the scheduler so a
    /// manual check also records history, fires alerts and reschedules.
    pub async fn check(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let watch = self.repo.find_watch(id).await?
            .ok_or(sqlx::Error::RowNotFound)?;
        let offer = self.repo.find_offer(watch.offer_id).await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let parsed = url::Url::parse(&offer.url)
            .map_err(|e| sqlx::Error::Protocol(format!("Invalid URL: {}", e)))?;

        let (db_needs_js, db_needs_stealth) = self.repo.find_store_by_domain(
            parsed.host_str().unwrap_or("")
        ).await.ok().flatten().map(|s| (s.needs_js, s.needs_stealth)).unwrap_or((false, false));

        check_watch(&self.repo, watch, offer, db_needs_js, db_needs_stealth, &self.default_currency, &self.events).await
    }
}

/// Scrapes one offer, persists the result, records history and dispatches alerts.
/// Shared by the scheduler tick and the manual check endpoint.
pub async fn check_watch(
    repo: &Repository,
    watch: Watch,
    offer: ProductOffer,
    db_needs_js: bool,
    db_needs_stealth: bool,
    default_currency: &str,
    events: &broadcast::Sender<AppEvent>,
) -> Result<(), sqlx::Error> {
    if let Err(e) = repo.set_watch_checking(watch.id).await {
        warn!("Failed to set watch {} checking: {}", watch.id, e);
        return Ok(());
    }

    let parsed = match url::Url::parse(&offer.url) {
        Ok(u) => u,
        Err(_) => {
            warn!("Invalid URL for offer {}: {}", offer.id, offer.url);
            let _ = repo.increment_watch_failure(watch.id).await;
            let _ = repo.set_watch_status(watch.id, "ERROR").await;
            return Ok(());
        }
    };

    let extracted = scraper::extract_product(&parsed, default_currency, db_needs_js, db_needs_stealth).await;

    match extracted {
        Ok((data, learned_js, learned_stealth)) => {
            if (learned_js && !db_needs_js) || (learned_stealth && !db_needs_stealth) {
                if let Some(domain) = parsed.host_str() {
                    let _ = repo.update_store_scraper_state(
                        domain,
                        db_needs_js || learned_js,
                        db_needs_stealth || learned_stealth,
                    ).await;
                }
            }

            let new_price = data.price;
            let new_availability = data.availability;
            let old_price = offer.current_price;

            // A currency flip would look like a 99% price drop — skip
            if new_price > 0.0
                && !offer.currency.is_empty()
                && !data.currency.is_empty()
                && data.currency != offer.currency
            {
                warn!(
                    "Currency mismatch on offer {}: stored {} vs scraped {}, skipping update",
                    offer.id, offer.currency, data.currency
                );
                let _ = repo.set_watch_status(watch.id, "ACTIVE").await;
                let _ = repo.update_watch_next_check(watch.id, next_check_at(watch.interval_seconds)).await;
                return Ok(());
            }

            if new_price > 0.0 {
                if let Err(e) = repo.update_offer_price(offer.id, new_price, new_availability).await {
                    warn!("Failed to update offer price {}: {}", offer.id, e);
                    return Ok(());
                }
                let _ = events.send(AppEvent::PriceUpdated {
                    offer_id: offer.id,
                    new_price,
                    currency: offer.currency.clone(),
                });
            }

            // Only record history when something actually changed — identical rows
            // inflate the table and make sparklines flat with redundant points.
            let price_changed = (new_price - old_price).abs() > f64::EPSILON;
            let avail_changed = new_availability != offer.availability;
            if (new_price > 0.0 && price_changed) || avail_changed {
                if let Err(e) = repo.insert_price_history(offer.id, new_price, &offer.currency, new_availability).await {
                    warn!("Failed to insert price history for {}: {}", offer.id, e);
                }
            }

            let change = detect_price_change(old_price, new_price, offer.availability, new_availability);
            if change != PriceChangeType::Unchanged {
                info!("Price change on offer {}: {:?} ({} -> {})", offer.id, change, old_price, new_price);

                let notif_cfg = repo.get_notification_config().await
                    .ok().flatten()
                    .map(crate::api::handlers::notifications::NotificationConfig::from);

                let product_info = repo.get_product_info_for_offer(offer.id).await.ok().flatten();

                if let Ok(alerts) = repo.find_alerts_for_offer(offer.id).await {
                    for alert in alerts {
                        let is_historical_low = if matches!(alert.alert_type.as_str(), "HISTORICAL_LOW") {
                            repo.is_historical_low(offer.id, new_price).await.unwrap_or(false)
                        } else {
                            false
                        };

                        let already_triggered = alert.triggered_at.is_some();
                        let should_trigger = match alert.alert_type.as_str() {
                            "PRICE_DECREASE" => change == PriceChangeType::Decrease,
                            "PRICE_INCREASE" => change == PriceChangeType::Increase,
                            "BACK_IN_STOCK" => change == PriceChangeType::BackInStock,
                            "OUT_OF_STOCK" => change == PriceChangeType::OutOfStock,
                            "BELOW_PRICE" => !already_triggered && alert.threshold_price.is_some_and(|t| new_price <= t),
                            "HISTORICAL_LOW" => !already_triggered && is_historical_low,
                            _ => false,
                        };

                        if should_trigger {
                            info!("Triggering alert {} for offer {}", alert.id, offer.id);
                            let _ = repo.trigger_alert(alert.id).await;
                            let _ = events.send(AppEvent::AlertTriggered {
                                alert_id: alert.id,
                                offer_id: offer.id,
                                alert_type: alert.alert_type.clone(),
                            });

                            if let (Some(cfg), Some((product_name, _, store_name, offer_url))) =
                                (&notif_cfg, &product_info)
                            {
                                let payload = AlertPayload {
                                    product_name: product_name.clone(),
                                    store_name: store_name.clone(),
                                    old_price,
                                    new_price,
                                    currency: offer.currency.clone(),
                                    alert_type: alert.alert_type.clone(),
                                    offer_url: offer_url.clone(),
                                };
                                notifications::dispatch(cfg, &payload).await;
                            }
                        }
                    }
                }
            }

            // When price rises above a threshold, reset triggered_at so the alert
            // can fire again the next time the price drops below.
            if new_price > 0.0 && change == PriceChangeType::Increase {
                let _ = repo.reset_below_price_alerts(offer.id, new_price).await;
            }

            if !data.name.is_empty() && data.name != "Pending" && data.name != "Unknown" {
                let _ = repo.update_product_name(offer.product_id, &data.name).await;
            }

            let _ = repo.set_watch_status(watch.id, "ACTIVE").await;
            // Adaptive interval: after a price change check again at 50% of the
            // normal interval (prices often move in clusters), then settle back.
            let next_interval = if change != PriceChangeType::Unchanged {
                watch.interval_seconds / 2
            } else {
                watch.interval_seconds
            };
            let _ = repo.update_watch_next_check(watch.id, next_check_at(next_interval)).await;
            let _ = events.send(AppEvent::WatchStatusChanged { watch_id: watch.id, status: "ACTIVE".into() });
        }
        Err(ExtractionError::CaptchaRequired) => {
            warn!("CAPTCHA required for offer {}", offer.id);
            let _ = repo.set_watch_status(watch.id, "CAPTCHA_REQUIRED").await;
            let _ = events.send(AppEvent::WatchStatusChanged { watch_id: watch.id, status: "CAPTCHA_REQUIRED".into() });
        }
        Err(ExtractionError::Blocked) => {
            warn!("Blocked for offer {}", offer.id);
            let _ = repo.set_watch_status(watch.id, "BLOCKED").await;
            let _ = events.send(AppEvent::WatchStatusChanged { watch_id: watch.id, status: "BLOCKED".into() });
        }
        Err(e) => {
            warn!("Extraction failed for offer {}: {}", offer.id, e);
            let _ = repo.increment_watch_failure(watch.id).await;
            let _ = repo.set_watch_status(watch.id, "ERROR").await;
            let _ = events.send(AppEvent::WatchStatusChanged { watch_id: watch.id, status: "ERROR".into() });
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
enum PriceChangeType {
    Decrease,
    Increase,
    Unchanged,
    BackInStock,
    OutOfStock,
}

fn detect_price_change(old_price: f64, new_price: f64, old_avail: bool, new_avail: bool) -> PriceChangeType {
    if !old_avail && new_avail {
        return PriceChangeType::BackInStock;
    }
    if old_avail && !new_avail {
        return PriceChangeType::OutOfStock;
    }
    if old_price <= 0.0 || new_price <= 0.0 {
        return PriceChangeType::Unchanged;
    }
    // 0.5% threshold — scraper rounding errors are smaller than this
    if new_price < old_price * 0.995 {
        return PriceChangeType::Decrease;
    }
    if new_price > old_price * 1.005 {
        return PriceChangeType::Increase;
    }
    PriceChangeType::Unchanged
}

fn next_check_at(interval_seconds: i32) -> DateTime<Utc> {
    let jitter_ratio = rand::rng().random_range(-0.1..0.1);
    let jittered = (interval_seconds as f64) * (1.0 + jitter_ratio);
    Utc::now() + Duration::seconds(jittered.max(60.0) as i64)
}
