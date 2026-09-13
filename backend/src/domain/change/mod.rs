use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow)]
pub struct Alert {
    pub id: Uuid,
    pub offer_id: Uuid,
    pub alert_type: String,
    pub threshold_price: Option<f64>,
    pub enabled: bool,
    pub triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

