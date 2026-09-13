use axum::{Router, routing::{get, post, delete, patch}};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/lists", get(handlers::lists::list_lists))
        .route("/api/lists", post(handlers::lists::create_list))
        .route("/api/lists/{id}", get(handlers::lists::get_list))
        .route("/api/lists/{id}", patch(handlers::lists::rename_list))
        .route("/api/lists/{id}", delete(handlers::lists::delete_list))
        .route("/api/lists/{id}/items", post(handlers::lists::add_item))
        .route("/api/lists/{id}/items/{product_id}", patch(handlers::lists::update_item_quantity))
        .route("/api/lists/{id}/items/{product_id}", delete(handlers::lists::remove_item))
}
