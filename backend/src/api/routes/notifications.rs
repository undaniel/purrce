use axum::{Router, routing::{get, post, put}};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/notifications/config", get(handlers::notifications::get_config))
        .route("/api/notifications/config", put(handlers::notifications::update_config))
        .route("/api/notifications/test/telegram", post(handlers::notifications::test_telegram))
        .route("/api/notifications/test/ntfy", post(handlers::notifications::test_ntfy))
        .route("/api/notifications/test/email", post(handlers::notifications::test_email))
}
