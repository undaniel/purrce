use axum::{Router, routing::{get, post, patch, delete}};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/watches", get(handlers::watches::list_watches))
        .route("/api/watches", post(handlers::watches::create_watch))
        .route("/api/watches/{id}", patch(handlers::watches::update_watch))
        .route("/api/watches/{id}", delete(handlers::watches::delete_watch))
        .route("/api/watches/{id}/check", post(handlers::watches::check_watch))
        .route("/api/watches/{id}/resume", post(handlers::watches::resume_watch))
}
