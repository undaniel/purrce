use axum::{Router, routing::{get, patch, post}};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/alerts", get(handlers::alerts::list_alerts))
        .route("/api/alerts", post(handlers::alerts::create_alert))
        .route("/api/alerts/{id}", patch(handlers::alerts::update_alert))
}
