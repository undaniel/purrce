use axum::{Router, routing::{get, post, put, patch, delete}};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/products", get(handlers::products::list_products))
        .route("/api/products/all", get(handlers::products::list_all_products))
        .route("/api/products/stats", get(handlers::products::get_product_stats))
        .route("/api/products/import", post(handlers::products::import_product))
        .route("/api/products/{id}", get(handlers::products::get_product))
        .route("/api/products/{id}", put(handlers::products::update_product))
        .route("/api/products/{id}", delete(handlers::products::delete_product))
        .route("/api/products/{id}/offers", post(handlers::products::add_offer))
        .route("/api/products/{id}/comparison", get(handlers::products::get_comparison))
        .route("/api/offers/{id}/price", patch(handlers::products::update_offer_price))
}
