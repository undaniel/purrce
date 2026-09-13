use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow)]
#[allow(dead_code)]
pub struct Store {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub base_url: String,
    pub enabled: bool,
    pub needs_js: bool,
    pub needs_stealth: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow, Clone)]
pub struct ProductOffer {
    pub id: Uuid,
    pub product_id: Uuid,
    pub store_id: Uuid,
    pub url: String,
    #[allow(dead_code)]
    pub external_product_id: Option<String>,
    pub currency: String,
    pub current_price: f64,
    pub availability: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_name: Option<String>,
}
