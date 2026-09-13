use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

impl PaginationParams {
    pub fn page(&self) -> i32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i32 {
        self.limit.unwrap_or(10).clamp(1, 100)
    }

    pub fn offset(&self) -> i32 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Deserialize)]
pub struct ProductQueryParams {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub search: Option<String>,
    pub tag: Option<String>,
}

impl ProductQueryParams {
    pub fn page(&self) -> i32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i32 {
        self.limit.unwrap_or(10).clamp(1, 100)
    }

    pub fn offset(&self) -> i32 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub total_pages: i32,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i32, limit: i32) -> Self {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        Self {
            data,
            total,
            page,
            limit,
            total_pages: total_pages.max(1),
        }
    }
}

#[derive(Deserialize)]
pub struct ImportRequest {
    pub url: String,
    pub watch: Option<bool>,
    pub interval_seconds: Option<i32>,
    pub tags: Option<Vec<String>>,
    // Manual override — when provided, skips scraping
    pub name: Option<String>,
    pub price: Option<f64>,
    pub image_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TagResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

impl TagResponse {
    pub fn from_domain(tag: crate::domain::tag::Tag) -> Self {
        Self {
            id: tag.id,
            name: tag.name,
            color: tag.color,
        }
    }
}

#[derive(Serialize)]
pub struct TagUsageResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub product_count: i64,
}

#[derive(Serialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub offers: Vec<OfferResponse>,
    pub tags: Vec<TagResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProductResponse {
    pub fn from_domain(product: crate::domain::product::Product, offers: Vec<crate::domain::store::ProductOffer>, tags: Vec<crate::domain::tag::Tag>) -> Self {
        Self {
            id: product.id,
            name: product.name,
            description: product.description,
            image_url: product.image_url,
            offers: offers.into_iter().map(OfferResponse::from_domain).collect(),
            tags: tags.into_iter().map(TagResponse::from_domain).collect(),
            created_at: product.created_at,
            updated_at: product.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct OfferResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub store_id: Uuid,
    pub store_name: String,
    pub url: String,
    pub currency: String,
    pub current_price: f64,
    pub availability: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OfferResponse {
    pub fn from_domain(offer: crate::domain::store::ProductOffer) -> Self {
        Self {
            id: offer.id,
            product_id: offer.product_id,
            store_id: offer.store_id,
            store_name: offer.store_name.unwrap_or_default(),
            url: offer.url,
            currency: offer.currency,
            current_price: offer.current_price,
            availability: offer.availability,
            last_checked_at: offer.last_checked_at,
            created_at: offer.created_at,
            updated_at: offer.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct PriceHistoryResponse {
    pub id: Uuid,
    pub offer_id: Uuid,
    pub price: f64,
    pub currency: String,
    pub availability: bool,
    pub observed_at: DateTime<Utc>,
}

impl PriceHistoryResponse {
    pub fn from_domain(history: crate::domain::price::PriceHistory) -> Self {
        Self {
            id: history.id,
            offer_id: history.offer_id,
            price: history.price,
            currency: history.currency,
            availability: history.availability,
            observed_at: history.observed_at,
        }
    }
}

#[derive(Serialize)]
pub struct WatchResponse {
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
    // Joined product/store/offer context (present on list, absent on single mutations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<bool>,
}

impl WatchResponse {
    pub fn from_domain(watch: crate::domain::watch::Watch) -> Self {
        Self {
            id: watch.id,
            offer_id: watch.offer_id,
            enabled: watch.enabled,
            interval_seconds: watch.interval_seconds,
            last_check_at: watch.last_check_at,
            next_check_at: watch.next_check_at,
            status: watch.status,
            failure_count: watch.failure_count,
            created_at: watch.created_at,
            updated_at: watch.updated_at,
            product_id: None,
            product_name: None,
            store_name: None,
            offer_url: None,
            current_price: None,
            currency: None,
            availability: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_offer_info(
        mut self,
        product_id: Uuid,
        product_name: String,
        store_name: String,
        offer_url: String,
        current_price: f64,
        currency: String,
        availability: bool,
    ) -> Self {
        self.product_id = Some(product_id);
        self.product_name = Some(product_name);
        self.store_name = Some(store_name);
        self.offer_url = Some(offer_url);
        self.current_price = Some(current_price);
        self.currency = Some(currency);
        self.availability = Some(availability);
        self
    }
}

#[derive(Serialize)]
pub struct AlertResponse {
    pub id: Uuid,
    pub offer_id: Uuid,
    pub alert_type: String,
    pub threshold_price: Option<f64>,
    pub enabled: bool,
    pub triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub product_name: Option<String>,
    pub product_image: Option<String>,
    pub store_name: Option<String>,
    pub offer_url: Option<String>,
}

impl AlertResponse {
    pub fn from_domain(alert: crate::domain::change::Alert) -> Self {
        Self {
            id: alert.id,
            offer_id: alert.offer_id,
            alert_type: alert.alert_type,
            threshold_price: alert.threshold_price,
            enabled: alert.enabled,
            triggered_at: alert.triggered_at,
            created_at: alert.created_at,
            product_name: None,
            product_image: None,
            store_name: None,
            offer_url: None,
        }
    }

    pub fn with_product_info(mut self, product_name: Option<String>, product_image: Option<String>, store_name: Option<String>, offer_url: Option<String>) -> Self {
        self.product_name = product_name;
        self.product_image = product_image;
        self.store_name = store_name;
        self.offer_url = offer_url;
        self
    }
}

#[derive(Serialize)]
pub struct ComparisonResponse {
    pub product: ProductResponse,
    pub offers: Vec<OfferResponse>,
    pub best_offer: Option<OfferResponse>,
    pub min_price: f64,
    pub max_price: f64,
    pub savings: f64,
}

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
    #[serde(skip)]
    pub status: StatusCode,
}

impl ErrorResponse {
    pub fn bad_request(message: &str) -> Self {
        Self {
            error: ErrorBody {
                code: "BAD_REQUEST".to_string(),
                message: message.to_string(),
            },
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self {
            error: ErrorBody {
                code: "NOT_FOUND".to_string(),
                message: message.to_string(),
            },
            status: StatusCode::NOT_FOUND,
        }
    }

    /// Logs the real cause and returns a generic message, so DB/schema details
    /// never reach the client.
    pub fn internal(message: String) -> Self {
        tracing::error!(error = %message, "internal server error");
        Self {
            error: ErrorBody {
                code: "INTERNAL_ERROR".to_string(),
                message: "Error interno del servidor".to_string(),
            },
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// For failures of user-configured external services (Telegram, SMTP, ntfy),
    /// where the upstream detail is actionable and contains no server internals.
    pub fn bad_gateway(message: String) -> Self {
        tracing::warn!(error = %message, "upstream service error");
        Self {
            error: ErrorBody {
                code: "UPSTREAM_ERROR".to_string(),
                message,
            },
            status: StatusCode::BAD_GATEWAY,
        }
    }

    pub fn unprocessable_code(code: &str, message: &str) -> Self {
        Self {
            error: ErrorBody {
                code: code.to_string(),
                message: message.to_string(),
            },
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        (self.status, axum::Json(self.error)).into_response()
    }
}
