pub mod routes;
pub mod handlers;
pub mod dto;

use axum::{Router, routing::get};
use axum::http::{HeaderValue, StatusCode, header};
use sqlx::PgPool;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};
use crate::infrastructure::database::repository::Repository;
use crate::application::product::ProductService;
use crate::application::watch::WatchService;
use crate::api::handlers::notifications::NotificationConfig;

/// Events pushed to SSE subscribers.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    PriceUpdated { offer_id: uuid::Uuid, new_price: f64, currency: String },
    WatchStatusChanged { watch_id: uuid::Uuid, status: String },
    AlertTriggered { alert_id: uuid::Uuid, offer_id: uuid::Uuid, alert_type: String },
}

#[derive(Clone)]
pub struct AppState {
    pub repo: Repository,
    pub product_svc: ProductService,
    pub watch_svc: WatchService,
    pub notification_config: Arc<RwLock<NotificationConfig>>,
    pub events: broadcast::Sender<AppEvent>,
}

pub fn create_router(pool: PgPool, default_currency: String) -> (Router, broadcast::Sender<AppEvent>) {
    // ponytail: 128 slots; old events are dropped when the buffer is full
    let (events_tx, _) = broadcast::channel(128);

    let repo = Repository::new(pool);
    let product_svc = ProductService::new(repo.clone(), default_currency.clone());
    let watch_svc = WatchService::new(repo.clone(), default_currency, events_tx.clone());

    let notification_config = load_notification_config();

    let state = AppState {
        repo,
        product_svc,
        watch_svc,
        notification_config: Arc::new(RwLock::new(notification_config)),
        events: events_tx.clone(),
    };

    let frontend_dir = std::env::var("FRONTEND_DIR")
        .unwrap_or_else(|_| "frontend/build".to_string());

    let serve_frontend = std::path::Path::new(&frontend_dir).exists();

    // SSE connections are long-lived, so they must stay out of the request timeout.
    let events_route = Router::new()
        .route("/api/events", get(routes::events::sse_handler));

    let api_routes = Router::new()
        .route("/api/health", get(routes::health::health_check))
        .merge(routes::products::router())
        .merge(routes::offers::router())
        .merge(routes::watches::router())
        .merge(routes::alerts::router())
        .merge(routes::notifications::router())
        .merge(routes::tags::router())
        .merge(routes::lists::router())
        // Generous ceiling: a cold import can drive a headless browser for a while.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(180),
        ))
        .merge(events_route);

    let router = if serve_frontend {
        let index_path = format!("{}/index.html", frontend_dir);
        let static_service = ServeDir::new(&frontend_dir)
            .not_found_service(ServeFile::new(&index_path));
        Router::new()
            .merge(api_routes)
            .fallback_service(static_service)
    } else {
        Router::new().merge(api_routes)
    };

    // The frontend is served from the same origin (and Vite proxies /api in dev),
    // so no CORS is needed. Cross-origin reads/writes are blocked by default.
    let router = router
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        ))
        .with_state(state);

    (router, events_tx)
}

fn load_notification_config() -> NotificationConfig {
    NotificationConfig::default()
}
