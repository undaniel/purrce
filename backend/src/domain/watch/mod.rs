use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(FromRow)]
pub struct Watch {
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
}
