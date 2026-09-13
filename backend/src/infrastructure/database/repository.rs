use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::product::Product;
use crate::domain::store::{ProductOffer, Store};
use crate::domain::price::PriceHistory;
use crate::domain::watch::Watch;
use crate::domain::change::Alert;
use crate::domain::tag::Tag;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProductList {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ListItemRow {
    pub id: Uuid,
    pub list_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i32,
    pub created_at: DateTime<Utc>,
    pub product_name: String,
    pub image_url: Option<String>,
    pub best_price: Option<f64>,
    pub currency: Option<String>,
}

/// Flat row returned by find_pending_watches_with_offers (avoids N+1 on the scheduler tick).
#[derive(FromRow)]
pub struct PendingWatch {
    pub watch_id: Uuid,
    pub offer_id: Uuid,
    pub enabled: bool,
    pub interval_seconds: i32,
    pub last_check_at: Option<DateTime<Utc>>,
    pub next_check_at: Option<DateTime<Utc>>,
    pub status: String,
    pub failure_count: i32,
    pub watch_created_at: DateTime<Utc>,
    pub watch_updated_at: DateTime<Utc>,
    pub offer_pk: Uuid,
    pub product_id: Uuid,
    pub store_id: Uuid,
    pub url: String,
    pub external_product_id: Option<String>,
    pub currency: String,
    pub current_price: f64,
    pub availability: bool,
    pub offer_last_checked_at: Option<DateTime<Utc>>,
    pub offer_created_at: DateTime<Utc>,
    pub offer_updated_at: DateTime<Utc>,
    pub store_name: Option<String>,
    pub needs_js: bool,
    pub needs_stealth: bool,
}

impl PendingWatch {
    pub fn split(self) -> (Watch, ProductOffer, bool, bool) {
        let watch = Watch {
            id: self.watch_id,
            offer_id: self.offer_id,
            enabled: self.enabled,
            interval_seconds: self.interval_seconds,
            last_check_at: self.last_check_at,
            next_check_at: self.next_check_at,
            status: self.status,
            failure_count: self.failure_count,
            created_at: self.watch_created_at,
            updated_at: self.watch_updated_at,
        };
        let offer = ProductOffer {
            id: self.offer_pk,
            product_id: self.product_id,
            store_id: self.store_id,
            url: self.url,
            external_product_id: self.external_product_id,
            currency: self.currency,
            current_price: self.current_price,
            availability: self.availability,
            last_checked_at: self.offer_last_checked_at,
            created_at: self.offer_created_at,
            updated_at: self.offer_updated_at,
            store_name: self.store_name,
        };
        (watch, offer, self.needs_js, self.needs_stealth)
    }
}

/// Watch row joined with the product/store/offer it monitors, so the UI can
/// show what each monitor actually corresponds to.
#[derive(FromRow)]
pub struct WatchWithInfo {
    pub id: Uuid,
    pub offer_id: Uuid,
    pub enabled: bool,
    pub interval_seconds: i32,
    pub last_check_at: Option<DateTime<Utc>>,
    pub next_check_at: Option<DateTime<Utc>>,
    pub status: String,
    pub failure_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub product_id: Uuid,
    pub product_name: String,
    pub store_name: String,
    pub offer_url: String,
    pub current_price: f64,
    pub currency: String,
    pub availability: bool,
}

impl WatchWithInfo {
    pub fn into_watch(self) -> Watch {
        Watch {
            id: self.id,
            offer_id: self.offer_id,
            enabled: self.enabled,
            interval_seconds: self.interval_seconds,
            last_check_at: self.last_check_at,
            next_check_at: self.next_check_at,
            status: self.status,
            failure_count: self.failure_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct Repository {
    pub pool: PgPool,
}

impl Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── Store ──────────────────────────────────────────────────────

    pub async fn find_store_by_domain(&self, domain: &str) -> Result<Option<Store>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, domain, base_url, enabled, needs_js, needs_stealth, created_at FROM stores WHERE domain = $1",
        )
        .bind(domain)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_store(&self, name: &str, domain: &str, base_url: &str) -> Result<Store, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO stores (name, domain, base_url, enabled) VALUES ($1, $2, $3, true) RETURNING id, name, domain, base_url, enabled, needs_js, needs_stealth, created_at",
        )
        .bind(name)
        .bind(domain)
        .bind(base_url)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_store_scraper_state(&self, domain: &str, needs_js: bool, needs_stealth: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE stores SET needs_js = $2, needs_stealth = $3 WHERE domain = $1")
            .bind(domain)
            .bind(needs_js)
            .bind(needs_stealth)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_store_name(&self, id: Uuid, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE stores SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Product ────────────────────────────────────────────────────

    pub async fn list_products(&self) -> Result<Vec<Product>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, image_url, created_at, updated_at FROM products ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }


    /// Search products by name or store name, with optional tag filter
    pub async fn search_products_paginated(
        &self,
        query: Option<&str>,
        tag: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<Product>, i64), sqlx::Error> {
        let search_pattern = query.map(|q| format!("%{}%", q.to_lowercase()));
        
        let products = if let Some(tag_name) = tag {
            sqlx::query_as::<_, Product>(
                r#"
                SELECT DISTINCT p.id, p.name, p.description, p.image_url, p.created_at, p.updated_at
                FROM products p
                LEFT JOIN product_tags pt ON p.id = pt.product_id
                LEFT JOIN tags t ON pt.tag_id = t.id
                LEFT JOIN product_offers o ON p.id = o.product_id
                LEFT JOIN stores s ON o.store_id = s.id
                WHERE ($1::text IS NULL OR LOWER(p.name) LIKE $1 OR LOWER(s.name) LIKE $1)
                AND ($2::text IS NULL OR t.name = $2)
                ORDER BY p.created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&search_pattern)
            .bind(tag_name)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Product>(
                r#"
                SELECT DISTINCT p.id, p.name, p.description, p.image_url, p.created_at, p.updated_at
                FROM products p
                LEFT JOIN product_offers o ON p.id = o.product_id
                LEFT JOIN stores s ON o.store_id = s.id
                WHERE ($1::text IS NULL OR LOWER(p.name) LIKE $1 OR LOWER(s.name) LIKE $1)
                ORDER BY p.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let total: (i64,) = if let Some(tag_name) = tag {
            sqlx::query_as(
                r#"
                SELECT COUNT(DISTINCT p.id)
                FROM products p
                LEFT JOIN product_tags pt ON p.id = pt.product_id
                LEFT JOIN tags t ON pt.tag_id = t.id
                LEFT JOIN product_offers o ON p.id = o.product_id
                LEFT JOIN stores s ON o.store_id = s.id
                WHERE ($1::text IS NULL OR LOWER(p.name) LIKE $1 OR LOWER(s.name) LIKE $1)
                AND ($2::text IS NULL OR t.name = $2)
                "#,
            )
            .bind(&search_pattern)
            .bind(tag_name)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT COUNT(DISTINCT p.id)
                FROM products p
                LEFT JOIN product_offers o ON p.id = o.product_id
                LEFT JOIN stores s ON o.store_id = s.id
                WHERE ($1::text IS NULL OR LOWER(p.name) LIKE $1 OR LOWER(s.name) LIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&self.pool)
            .await?
        };

        Ok((products, total.0))
    }

    /// Get dashboard stats: total products, products with price changes
    pub async fn get_product_stats(&self) -> Result<(i64, i64, i64, i64), sqlx::Error> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products")
            .fetch_one(&self.pool)
            .await?;

        // Products with price decrease in last 24h
        let down: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT ph1.offer_id)
            FROM price_history ph1
            JOIN price_history ph2 ON ph1.offer_id = ph2.offer_id
            WHERE ph1.observed_at = (
                SELECT MAX(observed_at) FROM price_history WHERE offer_id = ph1.offer_id
            )
            AND ph2.observed_at = (
                SELECT MAX(observed_at) FROM price_history 
                WHERE offer_id = ph2.offer_id AND observed_at < ph1.observed_at
            )
            AND ph1.price < ph2.price
            AND ph1.observed_at > NOW() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        // Products with price increase in last 24h
        let up: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT ph1.offer_id)
            FROM price_history ph1
            JOIN price_history ph2 ON ph1.offer_id = ph2.offer_id
            WHERE ph1.observed_at = (
                SELECT MAX(observed_at) FROM price_history WHERE offer_id = ph1.offer_id
            )
            AND ph2.observed_at = (
                SELECT MAX(observed_at) FROM price_history 
                WHERE offer_id = ph2.offer_id AND observed_at < ph1.observed_at
            )
            AND ph1.price > ph2.price
            AND ph1.observed_at > NOW() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let unchanged = total.0 - down.0 - up.0;

        Ok((total.0, down.0, up.0, unchanged.max(0)))
    }

    /// Dashboard extras: products at historical low, products without price,
    /// and watchlist value grouped by currency (cheapest offer per product).
    pub async fn get_dashboard_extras(
        &self,
    ) -> Result<(i64, i64, Vec<(String, f64, i64)>), sqlx::Error> {
        // Products whose cheapest available offer is at its historical minimum
        // (requires at least two recorded prices).
        let at_min: (i64,) = sqlx::query_as(
            r#"
            WITH best AS (
                SELECT DISTINCT ON (o.product_id) o.id AS offer_id, o.current_price
                FROM product_offers o
                WHERE o.current_price > 0
                ORDER BY o.product_id, o.current_price ASC
            )
            SELECT COUNT(*)
            FROM best b
            WHERE (
                SELECT COUNT(*) FROM price_history ph WHERE ph.offer_id = b.offer_id
            ) >= 2
            AND b.current_price <= (
                SELECT MIN(ph.price) FROM price_history ph
                WHERE ph.offer_id = b.offer_id AND ph.price > 0
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        // Products with no priced offer at all.
        let no_price: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM products p
            WHERE NOT EXISTS (
                SELECT 1 FROM product_offers o
                WHERE o.product_id = p.id AND o.current_price > 0
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        // Total watchlist value per currency (cheapest offer per product).
        let values: Vec<(String, f64, i64)> = sqlx::query_as(
            r#"
            WITH best AS (
                SELECT DISTINCT ON (o.product_id) o.currency, o.current_price
                FROM product_offers o
                WHERE o.current_price > 0
                ORDER BY o.product_id, o.current_price ASC
            )
            SELECT currency, SUM(current_price)::float8, COUNT(*)::int8
            FROM best
            GROUP BY currency
            ORDER BY COUNT(*) DESC, SUM(current_price) DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok((at_min.0, no_price.0, values))
    }

    pub async fn find_product(&self, id: Uuid) -> Result<Option<Product>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, image_url, created_at, updated_at FROM products WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_product(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<Product, sqlx::Error> {
        sqlx::query_as(
            "UPDATE products SET name = COALESCE($2, name), description = COALESCE($3, description), image_url = COALESCE($4, image_url), updated_at = NOW() WHERE id = $1 RETURNING id, name, description, image_url, created_at, updated_at",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(image_url)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_product_name(&self, id: Uuid, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE products SET name = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_product(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── ProductOffer ───────────────────────────────────────────────

    pub async fn find_offers_by_product(&self, product_id: Uuid) -> Result<Vec<ProductOffer>, sqlx::Error> {
        sqlx::query_as(
            "SELECT po.id, po.product_id, po.store_id, po.url, po.external_product_id, po.currency, po.current_price, po.availability, po.last_checked_at, po.created_at, po.updated_at, s.name as store_name FROM product_offers po JOIN stores s ON s.id = po.store_id WHERE po.product_id = $1",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Offers for many products in one round-trip (avoids the per-product N+1).
    pub async fn find_offers_by_products(&self, product_ids: &[Uuid]) -> Result<Vec<ProductOffer>, sqlx::Error> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as(
            "SELECT po.id, po.product_id, po.store_id, po.url, po.external_product_id, po.currency, po.current_price, po.availability, po.last_checked_at, po.created_at, po.updated_at, s.name as store_name \
             FROM product_offers po JOIN stores s ON s.id = po.store_id \
             WHERE po.product_id = ANY($1) \
             ORDER BY po.current_price ASC NULLS LAST, po.created_at ASC",
        )
        .bind(product_ids)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn find_offer(&self, id: Uuid) -> Result<Option<ProductOffer>, sqlx::Error> {
        sqlx::query_as(
            "SELECT po.id, po.product_id, po.store_id, po.url, po.external_product_id, po.currency, po.current_price, po.availability, po.last_checked_at, po.created_at, po.updated_at, s.name as store_name FROM product_offers po JOIN stores s ON s.id = po.store_id WHERE po.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_offer_price(
        &self,
        id: Uuid,
        price: f64,
        availability: bool,
    ) -> Result<ProductOffer, sqlx::Error> {
        sqlx::query_as(
            "UPDATE product_offers SET current_price = $2, availability = $3, last_checked_at = NOW(), updated_at = NOW() \
             WHERE id = $1 \
             RETURNING id, product_id, store_id, url, external_product_id, currency, current_price, availability, last_checked_at, created_at, updated_at, NULL::text as store_name",
        )
        .bind(id)
        .bind(price)
        .bind(availability)
        .fetch_one(&self.pool)
        .await
    }

    /// Existing offer for a canonical store+URL pair, used to avoid duplicates.
    pub async fn find_offer_by_store_and_url(&self, store_id: Uuid, url: &str) -> Result<Option<ProductOffer>, sqlx::Error> {
        sqlx::query_as(
            "SELECT po.id, po.product_id, po.store_id, po.url, po.external_product_id, po.currency, po.current_price, po.availability, po.last_checked_at, po.created_at, po.updated_at, s.name as store_name \
             FROM product_offers po JOIN stores s ON s.id = po.store_id \
             WHERE po.store_id = $1 AND po.url = $2",
        )
        .bind(store_id)
        .bind(url)
        .fetch_optional(&self.pool)
        .await
    }

    /// Creates a product, its first offer, history, watch and default alert in a
    /// single transaction, so a mid-way failure never leaves an orphan product.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_product_with_offer(
        &self,
        name: &str,
        image_url: Option<&str>,
        store_id: Uuid,
        url: &str,
        currency: &str,
        price: f64,
        availability: bool,
        external_id: Option<&str>,
        watch: bool,
        interval: i32,
    ) -> Result<(Product, ProductOffer), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let product: Product = sqlx::query_as(
            "INSERT INTO products (name, description, image_url) VALUES ($1, NULL, $2) \
             RETURNING id, name, description, image_url, created_at, updated_at",
        )
        .bind(name)
        .bind(image_url)
        .fetch_one(&mut *tx)
        .await?;

        let last_checked = (price > 0.0).then(Utc::now);
        let offer: ProductOffer = sqlx::query_as(
            "INSERT INTO product_offers (product_id, store_id, url, external_product_id, currency, current_price, availability, last_checked_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             RETURNING id, product_id, store_id, url, external_product_id, currency, current_price, availability, last_checked_at, created_at, updated_at, NULL::text as store_name",
        )
        .bind(product.id)
        .bind(store_id)
        .bind(url)
        .bind(external_id)
        .bind(currency)
        .bind(price)
        .bind(availability)
        .bind(last_checked)
        .fetch_one(&mut *tx)
        .await?;

        if price > 0.0 {
            sqlx::query("INSERT INTO price_history (offer_id, price, currency, availability) VALUES ($1, $2, $3, $4)")
                .bind(offer.id)
                .bind(price)
                .bind(currency)
                .bind(availability)
                .execute(&mut *tx)
                .await?;
        }

        if watch {
            sqlx::query("INSERT INTO watches (offer_id, enabled, interval_seconds, status) VALUES ($1, true, $2, 'ACTIVE')")
                .bind(offer.id)
                .bind(interval)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("INSERT INTO alerts (offer_id, alert_type, threshold_price, enabled) VALUES ($1, 'PRICE_DECREASE', NULL, true)")
            .bind(offer.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((product, offer))
    }

    /// Adds an offer (with its history, optional watch and default alert) to an
    /// existing product in a single transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_offer_with_history(
        &self,
        product_id: Uuid,
        store_id: Uuid,
        url: &str,
        currency: &str,
        price: f64,
        availability: bool,
        external_id: Option<&str>,
        watch: bool,
        interval: i32,
    ) -> Result<ProductOffer, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let last_checked = (price > 0.0).then(Utc::now);
        let offer: ProductOffer = sqlx::query_as(
            "INSERT INTO product_offers (product_id, store_id, url, external_product_id, currency, current_price, availability, last_checked_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             RETURNING id, product_id, store_id, url, external_product_id, currency, current_price, availability, last_checked_at, created_at, updated_at, NULL::text as store_name",
        )
        .bind(product_id)
        .bind(store_id)
        .bind(url)
        .bind(external_id)
        .bind(currency)
        .bind(price)
        .bind(availability)
        .bind(last_checked)
        .fetch_one(&mut *tx)
        .await?;

        if price > 0.0 {
            sqlx::query("INSERT INTO price_history (offer_id, price, currency, availability) VALUES ($1, $2, $3, $4)")
                .bind(offer.id)
                .bind(price)
                .bind(currency)
                .bind(availability)
                .execute(&mut *tx)
                .await?;
        }

        if watch {
            sqlx::query("INSERT INTO watches (offer_id, enabled, interval_seconds, status) VALUES ($1, true, $2, 'ACTIVE')")
                .bind(offer.id)
                .bind(interval)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("INSERT INTO alerts (offer_id, alert_type, threshold_price, enabled) VALUES ($1, 'PRICE_DECREASE', NULL, true)")
            .bind(offer.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(offer)
    }

    // ── PriceHistory ───────────────────────────────────────────────

    pub async fn insert_price_history(
        &self,
        offer_id: Uuid,
        price: f64,
        currency: &str,
        availability: bool,
    ) -> Result<PriceHistory, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO price_history (offer_id, price, currency, availability) VALUES ($1, $2, $3, $4) RETURNING id, offer_id, price, currency, availability, observed_at",
        )
        .bind(offer_id)
        .bind(price)
        .bind(currency)
        .bind(availability)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_price_history_paginated(&self, offer_id: Uuid, limit: i32, offset: i32) -> Result<(Vec<PriceHistory>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, PriceHistory>(
            "SELECT id, offer_id, price, currency, availability, observed_at FROM price_history WHERE offer_id = $1 ORDER BY observed_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(offer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM price_history WHERE offer_id = $1")
            .bind(offer_id)
            .fetch_one(&self.pool)
            .await?;

        Ok((rows, total.0))
    }

    /// Fetch recent price history for many offers in a single round-trip.
    /// Rows come ordered by offer and time ascending. Avoids the N+1 pattern
    /// when computing price signals for a list of products.
    pub async fn list_price_history_for_offers(
        &self,
        offer_ids: &[Uuid],
        per_offer_limit: i64,
    ) -> Result<Vec<PriceHistory>, sqlx::Error> {
        if offer_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, PriceHistory>(
            r#"
            SELECT id, offer_id, price, currency, availability, observed_at
            FROM (
                SELECT id, offer_id, price, currency, availability, observed_at,
                       ROW_NUMBER() OVER (PARTITION BY offer_id ORDER BY observed_at DESC) AS rn
                FROM price_history
                WHERE offer_id = ANY($1)
            ) ranked
            WHERE rn <= $2
            ORDER BY offer_id, observed_at ASC
            "#,
        )
        .bind(offer_ids)
        .bind(per_offer_limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_min_price(&self, offer_id: Uuid) -> Result<f64, sqlx::Error> {
        let row: (f64,) = sqlx::query_as(
            "SELECT COALESCE(MIN(price), 0) FROM price_history WHERE offer_id = $1 AND price > 0",
        )
        .bind(offer_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn is_historical_low(&self, offer_id: Uuid, price: f64) -> Result<bool, sqlx::Error> {
        if price <= 0.0 {
            return Ok(false);
        }
        let min = self.get_min_price(offer_id).await?;
        Ok(price <= min)
    }

    // ── Watch ──────────────────────────────────────────────────────

    pub async fn list_watches_with_info_paginated(&self, limit: i32, offset: i32) -> Result<(Vec<WatchWithInfo>, i64), sqlx::Error> {
        let watches = sqlx::query_as::<_, WatchWithInfo>(
            r#"
            SELECT
              w.id, w.offer_id, w.enabled, w.interval_seconds, w.last_check_at,
              w.next_check_at, w.status, w.failure_count, w.created_at, w.updated_at,
              p.id   AS product_id,
              p.name AS product_name,
              s.name AS store_name,
              o.url  AS offer_url,
              o.current_price,
              o.currency,
              o.availability
            FROM watches w
            JOIN product_offers o ON o.id = w.offer_id
            JOIN products p ON p.id = o.product_id
            JOIN stores s ON s.id = o.store_id
            ORDER BY w.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watches")
            .fetch_one(&self.pool)
            .await?;

        Ok((watches, total.0))
    }

    pub async fn find_watch(&self, id: Uuid) -> Result<Option<Watch>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, offer_id, enabled, interval_seconds, last_check_at, next_check_at, status, failure_count, created_at, updated_at FROM watches WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_watch(
        &self,
        offer_id: Uuid,
        interval_seconds: i32,
    ) -> Result<Watch, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO watches (offer_id, enabled, interval_seconds, status) VALUES ($1, true, $2, 'ACTIVE') RETURNING id, offer_id, enabled, interval_seconds, last_check_at, next_check_at, status, failure_count, created_at, updated_at",
        )
        .bind(offer_id)
        .bind(interval_seconds)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_watch(
        &self,
        id: Uuid,
        enabled: Option<bool>,
        interval_seconds: Option<i32>,
    ) -> Result<Watch, sqlx::Error> {
        sqlx::query_as(
            "UPDATE watches SET enabled = COALESCE($2, enabled), interval_seconds = COALESCE($3, interval_seconds), updated_at = NOW() WHERE id = $1 RETURNING id, offer_id, enabled, interval_seconds, last_check_at, next_check_at, status, failure_count, created_at, updated_at",
        )
        .bind(id)
        .bind(enabled)
        .bind(interval_seconds)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete_watch(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM watches WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_watch_checking(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE watches SET status = 'CHECKING', last_check_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Single query replacing N+1 find_offer calls in the scheduler tick.
    pub async fn find_pending_watches_with_offers(&self) -> Result<Vec<PendingWatch>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT
              w.id            AS watch_id,
              w.offer_id,
              w.enabled,
              w.interval_seconds,
              w.last_check_at,
              w.next_check_at,
              w.status,
              w.failure_count,
              w.created_at    AS watch_created_at,
              w.updated_at    AS watch_updated_at,
              po.id           AS offer_pk,
              po.product_id,
              po.store_id,
              po.url,
              po.external_product_id,
              po.currency,
              po.current_price,
              po.availability,
              po.last_checked_at AS offer_last_checked_at,
              po.created_at   AS offer_created_at,
              po.updated_at   AS offer_updated_at,
              s.name          AS store_name,
              s.needs_js,
              s.needs_stealth
            FROM watches w
            JOIN product_offers po ON po.id = w.offer_id
            JOIN stores s ON s.id = po.store_id
            WHERE w.enabled = true
              AND w.status = 'ACTIVE'
              AND (w.next_check_at IS NULL OR w.next_check_at <= NOW())
            ORDER BY w.next_check_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Reset BELOW_PRICE alerts whose threshold is now below the current price,
    /// so they can fire again if the price drops back down.
    pub async fn reset_below_price_alerts(&self, offer_id: Uuid, current_price: f64) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE alerts SET triggered_at = NULL \
             WHERE offer_id = $1 AND alert_type = 'BELOW_PRICE' \
               AND threshold_price < $2 AND triggered_at IS NOT NULL",
        )
        .bind(offer_id)
        .bind(current_price)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_watch_next_check(&self, id: Uuid, next_check_at: chrono::DateTime<chrono::Utc>) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE watches SET next_check_at = $2, status = 'ACTIVE' WHERE id = $1")
            .bind(id)
            .bind(next_check_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn increment_watch_failure(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE watches SET failure_count = failure_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_watch_status(&self, id: Uuid, status: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE watches SET status = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Alert ──────────────────────────────────────────────────────

    /// Get alerts with product info in a single query (avoids N+1)
    pub async fn list_alerts_with_info_paginated(&self, limit: i32, offset: i32) -> Result<(Vec<(Alert, String, Option<String>, String, String)>, i64), sqlx::Error> {
        #[derive(FromRow)]
        struct AlertWithInfo {
            id: Uuid,
            offer_id: Uuid,
            alert_type: String,
            threshold_price: Option<f64>,
            enabled: bool,
            triggered_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            product_name: String,
            product_image: Option<String>,
            store_name: String,
            offer_url: String,
        }

        let rows = sqlx::query_as::<_, AlertWithInfo>(
            r#"
            SELECT 
                a.id,
                a.offer_id,
                a.alert_type,
                a.threshold_price,
                a.enabled,
                a.triggered_at,
                a.created_at,
                p.name as product_name,
                p.image_url as product_image,
                s.name as store_name,
                o.url as offer_url
            FROM alerts a
            JOIN product_offers o ON a.offer_id = o.id
            JOIN products p ON o.product_id = p.id
            JOIN stores s ON o.store_id = s.id
            ORDER BY a.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM alerts")
            .fetch_one(&self.pool)
            .await?;

        let result: Vec<(Alert, String, Option<String>, String, String)> = rows
            .into_iter()
            .map(|row| {
                let alert = Alert {
                    id: row.id,
                    offer_id: row.offer_id,
                    alert_type: row.alert_type,
                    threshold_price: row.threshold_price,
                    enabled: row.enabled,
                    triggered_at: row.triggered_at,
                    created_at: row.created_at,
                };
                (alert, row.product_name, row.product_image, row.store_name, row.offer_url)
            })
            .collect();

        Ok((result, total.0))
    }

    pub async fn get_product_info_for_offer(&self, offer_id: Uuid) -> Result<Option<(String, Option<String>, String, String)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT 
                p.name as product_name,
                p.image_url as product_image,
                s.name as store_name,
                o.url as offer_url
            FROM product_offers o
            JOIN products p ON o.product_id = p.id
            JOIN stores s ON o.store_id = s.id
            WHERE o.id = $1
            "#,
        )
        .bind(offer_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_alerts_for_offer(&self, offer_id: Uuid) -> Result<Vec<Alert>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, offer_id, alert_type, threshold_price, enabled, triggered_at, created_at FROM alerts WHERE offer_id = $1 AND enabled = true",
        )
        .bind(offer_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create_alert(
        &self,
        offer_id: Uuid,
        alert_type: &str,
        threshold_price: Option<f64>,
    ) -> Result<Alert, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO alerts (offer_id, alert_type, threshold_price, enabled) VALUES ($1, $2, $3, true) RETURNING id, offer_id, alert_type, threshold_price, enabled, triggered_at, created_at",
        )
        .bind(offer_id)
        .bind(alert_type)
        .bind(threshold_price)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_alert(
        &self,
        id: Uuid,
        enabled: Option<bool>,
        threshold_price: Option<f64>,
    ) -> Result<Alert, sqlx::Error> {
        sqlx::query_as(
            "UPDATE alerts SET enabled = COALESCE($2, enabled), threshold_price = COALESCE($3, threshold_price) WHERE id = $1 RETURNING id, offer_id, alert_type, threshold_price, enabled, triggered_at, created_at",
        )
        .bind(id)
        .bind(enabled)
        .bind(threshold_price)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn trigger_alert(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE alerts SET triggered_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Tags ──────────────────────────────────────────────────────

    pub async fn list_tags(&self) -> Result<Vec<Tag>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, color, created_at FROM tags ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Tags together with how many products use each one (for the tag manager).
    pub async fn list_tags_with_usage(&self) -> Result<Vec<(Uuid, String, String, i64)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT t.id, t.name, t.color, COUNT(pt.product_id)::int8
            FROM tags t
            LEFT JOIN product_tags pt ON pt.tag_id = t.id
            GROUP BY t.id, t.name, t.color
            ORDER BY COUNT(pt.product_id) DESC, t.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_tag(
        &self,
        id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
    ) -> Result<Tag, sqlx::Error> {
        sqlx::query_as(
            "UPDATE tags SET name = COALESCE($2, name), color = COALESCE($3, color) WHERE id = $1 RETURNING id, name, color, created_at",
        )
        .bind(id)
        .bind(name)
        .bind(color)
        .fetch_one(&self.pool)
        .await
    }

    /// Move every product association from `source_id` into `target_id`,
    /// then delete the source tag. Used to clean up duplicates.
    pub async fn merge_tags(&self, source_id: Uuid, target_id: Uuid) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO product_tags (product_id, tag_id) \
             SELECT product_id, $2 FROM product_tags WHERE tag_id = $1 \
             ON CONFLICT DO NOTHING",
        )
        .bind(source_id)
        .bind(target_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(source_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_tag_by_name(&self, name: &str) -> Result<Option<Tag>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, color, created_at FROM tags WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_tag(&self, name: &str, color: Option<&str>) -> Result<Tag, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO tags (name, color) VALUES ($1, COALESCE($2, '#6366f1')) RETURNING id, name, color, created_at",
        )
        .bind(name)
        .bind(color)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete_tag(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_product_tags(&self, product_id: Uuid) -> Result<Vec<Tag>, sqlx::Error> {
        sqlx::query_as(
            "SELECT t.id, t.name, t.color, t.created_at FROM tags t JOIN product_tags pt ON pt.tag_id = t.id WHERE pt.product_id = $1 ORDER BY t.name ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Tags for many products in one round-trip, as (product_id, tag) pairs.
    pub async fn get_tags_for_products(&self, product_ids: &[Uuid]) -> Result<Vec<(Uuid, Tag)>, sqlx::Error> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(FromRow)]
        struct ProductTagRow {
            product_id: Uuid,
            id: Uuid,
            name: String,
            color: String,
            created_at: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, ProductTagRow>(
            "SELECT pt.product_id, t.id, t.name, t.color, t.created_at \
             FROM product_tags pt JOIN tags t ON t.id = pt.tag_id \
             WHERE pt.product_id = ANY($1) \
             ORDER BY t.name ASC",
        )
        .bind(product_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.product_id,
                    Tag { id: r.id, name: r.name, color: r.color, created_at: r.created_at },
                )
            })
            .collect())
    }

    pub async fn add_tag_to_product(&self, product_id: Uuid, tag_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO product_tags (product_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(product_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_product_tags(&self, product_id: Uuid, tag_ids: &[Uuid]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM product_tags WHERE product_id = $1")
            .bind(product_id)
            .execute(&mut *tx)
            .await?;

        // One round-trip for the whole set instead of one INSERT per tag.
        sqlx::query(
            "INSERT INTO product_tags (product_id, tag_id) \
             SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING",
        )
        .bind(product_id)
        .bind(tag_ids)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_or_create_tag(&self, name: &str) -> Result<Tag, sqlx::Error> {
        if let Some(tag) = self.find_tag_by_name(name).await? {
            Ok(tag)
        } else {
            self.create_tag(name, None).await
        }
    }

    // ── Notification Config ──────────────────────────────────────

    pub async fn get_notification_config(&self) -> Result<Option<NotificationConfigRow>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, telegram_enabled, telegram_bot_token, telegram_chat_id, \
             email_enabled, email_smtp_host, email_smtp_port, email_from, email_to, \
             webhook_enabled, webhook_url, ntfy_enabled, ntfy_url, ntfy_topic, ntfy_token, \
             email_smtp_password, created_at, updated_at \
             FROM notification_config LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_notification_config(
        &self,
        telegram_enabled: bool,
        telegram_bot_token: Option<&str>,
        telegram_chat_id: Option<&str>,
        email_enabled: bool,
        email_smtp_host: Option<&str>,
        email_smtp_port: Option<i32>,
        email_from: Option<&str>,
        email_to: Option<&str>,
        email_smtp_password: Option<&str>,
        webhook_enabled: bool,
        webhook_url: Option<&str>,
        ntfy_enabled: bool,
        ntfy_url: Option<&str>,
        ntfy_topic: Option<&str>,
        ntfy_token: Option<&str>,
    ) -> Result<NotificationConfigRow, sqlx::Error> {
        let existing = self.get_notification_config().await?;

        if existing.is_some() {
            sqlx::query_as(
                "UPDATE notification_config SET \
                 telegram_enabled = $1, telegram_bot_token = $2, telegram_chat_id = $3, \
                 email_enabled = $4, email_smtp_host = $5, email_smtp_port = $6, \
                 email_from = $7, email_to = $8, email_smtp_password = $9, \
                 webhook_enabled = $10, webhook_url = $11, \
                 ntfy_enabled = $12, ntfy_url = $13, ntfy_topic = $14, ntfy_token = $15, \
                 updated_at = NOW() \
                 WHERE id = (SELECT id FROM notification_config LIMIT 1) \
                 RETURNING id, telegram_enabled, telegram_bot_token, telegram_chat_id, \
                 email_enabled, email_smtp_host, email_smtp_port, email_from, email_to, \
                 webhook_enabled, webhook_url, ntfy_enabled, ntfy_url, ntfy_topic, ntfy_token, \
                 email_smtp_password, created_at, updated_at",
            )
            .bind(telegram_enabled)
            .bind(telegram_bot_token)
            .bind(telegram_chat_id)
            .bind(email_enabled)
            .bind(email_smtp_host)
            .bind(email_smtp_port)
            .bind(email_from)
            .bind(email_to)
            .bind(email_smtp_password)
            .bind(webhook_enabled)
            .bind(webhook_url)
            .bind(ntfy_enabled)
            .bind(ntfy_url)
            .bind(ntfy_topic)
            .bind(ntfy_token)
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "INSERT INTO notification_config \
                 (telegram_enabled, telegram_bot_token, telegram_chat_id, \
                  email_enabled, email_smtp_host, email_smtp_port, email_from, email_to, email_smtp_password, \
                  webhook_enabled, webhook_url, ntfy_enabled, ntfy_url, ntfy_topic, ntfy_token) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) \
                 RETURNING id, telegram_enabled, telegram_bot_token, telegram_chat_id, \
                 email_enabled, email_smtp_host, email_smtp_port, email_from, email_to, \
                 webhook_enabled, webhook_url, ntfy_enabled, ntfy_url, ntfy_topic, ntfy_token, \
                 email_smtp_password, created_at, updated_at",
            )
            .bind(telegram_enabled)
            .bind(telegram_bot_token)
            .bind(telegram_chat_id)
            .bind(email_enabled)
            .bind(email_smtp_host)
            .bind(email_smtp_port)
            .bind(email_from)
            .bind(email_to)
            .bind(email_smtp_password)
            .bind(webhook_enabled)
            .bind(webhook_url)
            .bind(ntfy_enabled)
            .bind(ntfy_url)
            .bind(ntfy_topic)
            .bind(ntfy_token)
            .fetch_one(&self.pool)
            .await
        }
    }

    // --- Lists ---

    pub async fn list_lists(&self) -> Result<Vec<ProductList>, sqlx::Error> {
        sqlx::query_as::<_, ProductList>(
            "SELECT id, name, created_at, updated_at FROM lists ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create_list(&self, name: &str) -> Result<ProductList, sqlx::Error> {
        sqlx::query_as::<_, ProductList>(
            "INSERT INTO lists (name) VALUES ($1) RETURNING id, name, created_at, updated_at"
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete_list(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM lists WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    pub async fn list_items(&self, list_id: Uuid) -> Result<Vec<ListItemRow>, sqlx::Error> {
        sqlx::query_as::<_, ListItemRow>(
            "SELECT li.id, li.list_id, li.product_id, li.quantity, li.created_at,
                    p.name AS product_name, p.image_url,
                    best.current_price AS best_price, best.currency AS currency
             FROM list_items li
             JOIN products p ON p.id = li.product_id
             LEFT JOIN LATERAL (
                 SELECT po.current_price, po.currency
                 FROM product_offers po
                 WHERE po.product_id = p.id AND po.current_price > 0
                 ORDER BY po.current_price ASC
                 LIMIT 1
             ) best ON true
             WHERE li.list_id = $1
             ORDER BY li.created_at ASC"
        )
        .bind(list_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn add_list_item(&self, list_id: Uuid, product_id: Uuid, quantity: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO list_items (list_id, product_id, quantity) VALUES ($1, $2, $3)
             ON CONFLICT (list_id, product_id) DO UPDATE SET quantity = EXCLUDED.quantity"
        )
        .bind(list_id)
        .bind(product_id)
        .bind(quantity)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_list(&self, id: Uuid) -> Result<Option<ProductList>, sqlx::Error> {
        sqlx::query_as::<_, ProductList>(
            "SELECT id, name, created_at, updated_at FROM lists WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn rename_list(&self, id: Uuid, name: &str) -> Result<ProductList, sqlx::Error> {
        sqlx::query_as::<_, ProductList>(
            "UPDATE lists SET name = $1, updated_at = NOW() WHERE id = $2 \
             RETURNING id, name, created_at, updated_at"
        )
        .bind(name)
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_list_item_quantity(&self, list_id: Uuid, product_id: Uuid, quantity: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE list_items SET quantity = $1 WHERE list_id = $2 AND product_id = $3")
            .bind(quantity)
            .bind(list_id)
            .bind(product_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_list_item(&self, list_id: Uuid, product_id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM list_items WHERE list_id = $1 AND product_id = $2")
            .bind(list_id)
            .bind(product_id)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct NotificationConfigRow {
    pub id: uuid::Uuid,
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub email_enabled: bool,
    pub email_smtp_host: Option<String>,
    pub email_smtp_port: Option<i32>,
    pub email_from: Option<String>,
    pub email_to: Option<String>,
    pub webhook_enabled: bool,
    pub webhook_url: Option<String>,
    pub ntfy_enabled: bool,
    pub ntfy_url: Option<String>,
    pub ntfy_topic: Option<String>,
    pub ntfy_token: Option<String>,
    pub email_smtp_password: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
