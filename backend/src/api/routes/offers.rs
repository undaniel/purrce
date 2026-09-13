use axum::{Router, routing::get};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/offers/{id}", get(handlers::offers::get_offer))
        .route("/api/offers/{id}/history", get(handlers::offers::get_offer_history))
        .route("/api/price-history", get(handlers::offers::get_offers_history_bulk))
}
