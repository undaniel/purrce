use axum::{Router, routing::{get, post, patch}};
use super::super::handlers;

pub fn router() -> Router<crate::api::AppState> {
    Router::new()
        .route("/api/tags", get(handlers::tags::list_tags))
        .route("/api/tags", post(handlers::tags::create_tag))
        .route("/api/tags/usage", get(handlers::tags::list_tags_usage))
        .route("/api/tags/{id}", patch(handlers::tags::update_tag).delete(handlers::tags::delete_tag))
        .route("/api/tags/{id}/merge", post(handlers::tags::merge_tag))
        .route("/api/products/{id}/tags", get(handlers::tags::get_product_tags))
        .route("/api/products/{id}/tags", post(handlers::tags::set_product_tags))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_do_not_conflict() {
        // matchit panics while building the router if two path patterns overlap.
        let _ = router();
    }
}
