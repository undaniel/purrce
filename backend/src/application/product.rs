use std::sync::{Arc, OnceLock};

use tokio::sync::Semaphore;
use uuid::Uuid;
use tracing::{debug, info};

use crate::domain::product::Product;
use crate::domain::store::ProductOffer;
use crate::infrastructure::database::repository::Repository;
use crate::infrastructure::scraper;

/// Tracking/session params that should never be stored — they change per-visit
/// and would make the same product look like different URLs.
const TRACKING_PARAMS: &[&str] = &[
    "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
    "fbclid", "gclid", "msclkid", "yclid", "_ga", "_gid",
    "mc_eid", "mc_cid", "ref", "source", "affiliate",
    // MercadoLibre session tokens
    "pdp_filters", "noIndex",
    // Amazon
    "tag", "linkCode", "linkId", "ref_",
];

/// Caps concurrent scrapes triggered by HTTP imports (the scheduler has its own
/// per-domain cap). The headless browser is the expensive resource.
static IMPORT_SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn import_sem() -> &'static Arc<Semaphore> {
    IMPORT_SEM.get_or_init(|| Arc::new(Semaphore::new(4)))
}

async fn acquire_scrape_permit() -> Result<tokio::sync::SemaphorePermit<'static>, sqlx::Error> {
    import_sem()
        .acquire()
        .await
        .map_err(|e| sqlx::Error::Protocol(format!("scrape semaphore closed: {}", e)))
}

/// Strips tracking params and the fragment so the same product URL is always
/// stored the same way, regardless of where the user copied it from.
fn canonicalize_url(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else { return url.to_string() };

    let keep: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !TRACKING_PARAMS.iter().any(|t| k.starts_with(t)))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    parsed.set_fragment(None);
    {
        // Re-encode through the URL API so values containing &, = or spaces stay valid.
        let mut qp = parsed.query_pairs_mut();
        qp.clear();
        for (k, v) in &keep {
            qp.append_pair(k, v);
        }
    }
    if keep.is_empty() {
        parsed.set_query(None);
    }

    parsed.to_string()
}

/// Extracts a store name from a domain.
/// - Strips "www." prefix
/// - Takes the first meaningful segment
/// - Capitalizes first letter
fn store_name_from_domain(domain: &str) -> String {
    let stripped = domain.strip_prefix("www.").unwrap_or(domain);
    let name = stripped.split('.').next().unwrap_or(stripped);

    let mut chars = name.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => name.to_string(),
    }
}

/// Checks if a store name looks like a domain fragment that should be cleaned up.
fn is_bad_store_name(name: &str) -> bool {
    let bad = ["www", "ww2", "ww3", "m", "mobile", "shop", "store", "web", "app"];
    bad.iter().any(|&b| name.eq_ignore_ascii_case(b))
}

#[derive(Clone)]
pub struct ProductService {
    repo: Repository,
    default_currency: String,
}

impl ProductService {
    pub fn new(repo: Repository, default_currency: String) -> Self {
        Self { repo, default_currency }
    }

    pub async fn get_product(&self, id: Uuid) -> Result<Option<(Product, Vec<ProductOffer>)>, sqlx::Error> {
        let product = self.repo.find_product(id).await?;
        match product {
            Some(p) => {
                let offers = self.repo.find_offers_by_product(p.id).await?;
                Ok(Some((p, offers)))
            }
            None => Ok(None),
        }
    }

    /// Resolves (or creates) the store for a domain, auto-correcting bad names.
    async fn resolve_store(&self, domain: &str) -> Result<crate::domain::store::Store, sqlx::Error> {
        match self.repo.find_store_by_domain(domain).await? {
            Some(s) if is_bad_store_name(&s.name) => {
                let corrected = store_name_from_domain(domain);
                info!(store_id = %s.id, old_name = %s.name, new_name = %corrected, "correcting bad store name");
                let _ = self.repo.update_store_name(s.id, &corrected).await;
                Ok(crate::domain::store::Store { name: corrected, ..s })
            }
            Some(s) => Ok(s),
            None => {
                let name = store_name_from_domain(domain);
                debug!(name, domain, "creating new store");
                self.repo.create_store(&name, domain, &format!("https://{}", domain)).await
            }
        }
    }

    pub async fn import_from_url(&self, url: &str, watch: bool, interval: i32, manual_name: Option<&str>, manual_price: Option<f64>, manual_image: Option<&str>) -> Result<(Product, ProductOffer), sqlx::Error> {
        let url = &canonicalize_url(url);
        let parsed = url::Url::parse(url).map_err(|_| sqlx::Error::Protocol("Invalid URL".into()))?;
        let domain = parsed.domain().ok_or_else(|| sqlx::Error::Protocol("No domain".into()))?;

        debug!(domain, "resolving store");
        let store = self.resolve_store(domain).await?;

        // Idempotency: the same store+URL is already tracked.
        if let Some(offer) = self.repo.find_offer_by_store_and_url(store.id, url).await? {
            if let Some(product) = self.repo.find_product(offer.product_id).await? {
                info!(url, offer_id = %offer.id, "offer already tracked, returning existing");
                return Ok((product, offer));
            }
        }

        let (name, price, currency, image_url, availability, external_id) = if let Some(name) = manual_name {
            info!(url, name, "using manual product data, skipping scraper");
            (name.to_string(), manual_price.unwrap_or(0.0), self.default_currency.clone(), manual_image.map(|s| s.to_string()), true, None)
        } else {
            info!(url, "scraping product (this may take up to 90s)");
            let scrape_start = std::time::Instant::now();
            let permit = acquire_scrape_permit().await?;
            let extracted = scraper::extract_product(&parsed, &self.default_currency, store.needs_js, store.needs_stealth).await;
            drop(permit);
            info!(url, elapsed_ms = scrape_start.elapsed().as_millis(), ok = extracted.is_ok(), "scrape finished");

            match extracted {
                Ok((data, learned_js, learned_stealth)) if data.name != "Pending" && data.price > 0.0 => {
                    if (learned_js && !store.needs_js) || (learned_stealth && !store.needs_stealth) {
                        let _ = self.repo.update_store_scraper_state(
                            domain,
                            store.needs_js || learned_js,
                            store.needs_stealth || learned_stealth,
                        ).await;
                    }
                    (data.name, data.price, data.currency, data.image_url, data.availability, data.external_product_id)
                }
                Err(scraper::ExtractionError::CaptchaRequired) => {
                    return Err(sqlx::Error::Protocol("BLOCKED_CAPTCHA".into()));
                }
                Err(scraper::ExtractionError::Blocked) => {
                    return Err(sqlx::Error::Protocol("BLOCKED_ACCESS".into()));
                }
                Ok(_) | Err(_) => {
                    return Err(sqlx::Error::Protocol("EXTRACTION_FAILED".into()));
                }
            }
        };

        info!(name, price, currency, availability, "scraped product data");
        let (product, offer) = self.repo.create_product_with_offer(
            &name,
            image_url.as_deref(),
            store.id,
            url,
            &currency,
            price,
            availability,
            external_id.as_deref(),
            watch,
            interval,
        ).await?;
        debug!(product_id = %product.id, offer_id = %offer.id, "product and offer created");

        Ok((product, offer))
    }

    pub async fn delete_product(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.repo.delete_product(id).await
    }

    pub async fn add_offer_to_product(&self, product_id: Uuid, url: &str, watch: bool, interval: i32) -> Result<ProductOffer, sqlx::Error> {
        self.repo.find_product(product_id).await?
            .ok_or_else(|| sqlx::Error::Protocol("Product not found".into()))?;

        let url = &canonicalize_url(url);
        let parsed = url::Url::parse(url).map_err(|_| sqlx::Error::Protocol("Invalid URL".into()))?;
        let domain = parsed.domain().ok_or_else(|| sqlx::Error::Protocol("No domain".into()))?;

        let store = self.resolve_store(domain).await?;

        // Idempotency: this URL is already tracked somewhere.
        if let Some(existing) = self.repo.find_offer_by_store_and_url(store.id, url).await? {
            info!(url, offer_id = %existing.id, "offer already tracked, returning existing");
            return Ok(existing);
        }

        let permit = acquire_scrape_permit().await?;
        let extracted = scraper::extract_product(&parsed, &self.default_currency, store.needs_js, store.needs_stealth).await;
        drop(permit);

        let (price, currency, availability, external_id) = match &extracted {
            Ok((data, learned_js, learned_stealth)) => {
                if (*learned_js && !store.needs_js) || (*learned_stealth && !store.needs_stealth) {
                    let _ = self.repo.update_store_scraper_state(
                        domain,
                        store.needs_js || *learned_js,
                        store.needs_stealth || *learned_stealth,
                    ).await;
                }
                (data.price, data.currency.clone(), data.availability, data.external_product_id.clone())
            }
            Err(_) => (0.0, self.default_currency.clone(), true, None),
        };

        self.repo.create_offer_with_history(
            product_id,
            store.id,
            url,
            &currency,
            price,
            availability,
            external_id.as_deref(),
            watch,
            interval,
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::canonicalize_url;

    #[test]
    fn strips_tracking_params_and_fragment() {
        let out = canonicalize_url("https://tienda.cl/p/1?utm_source=x&sku=ab&fbclid=y#top");
        assert_eq!(out, "https://tienda.cl/p/1?sku=ab");
    }

    #[test]
    fn encodes_values_with_reserved_chars() {
        let out = canonicalize_url("https://tienda.cl/p?a=b%26c&d=e%3Df");
        let parsed = url::Url::parse(&out).expect("valid url");
        let pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert!(pairs.contains(&("a".to_string(), "b&c".to_string())));
        assert!(pairs.contains(&("d".to_string(), "e=f".to_string())));
    }

    #[test]
    fn invalid_url_is_returned_as_is() {
        assert_eq!(canonicalize_url("not a url"), "not a url");
    }
}
