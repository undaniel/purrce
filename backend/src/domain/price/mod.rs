use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow)]
pub struct PriceHistory {
    pub id: Uuid,
    pub offer_id: Uuid,
    pub price: f64,
    pub currency: String,
    pub availability: bool,
    pub observed_at: DateTime<Utc>,
}
