use axum::{extract::{State, Path, Query}, Json};
use uuid::Uuid;
use serde::Deserialize;
use crate::api::dto::{WatchResponse, ErrorResponse, PaginationParams, PaginatedResponse};
use crate::api::handlers::products::{clamp_interval, MAX_INTERVAL, MIN_INTERVAL};
use crate::api::AppState;

#[derive(Deserialize)]
pub struct CreateWatchRequest {
    pub offer_id: Uuid,
    pub interval_seconds: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateWatchRequest {
    pub enabled: Option<bool>,
    pub interval_seconds: Option<i32>,
}

pub async fn list_watches(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<WatchResponse>>, ErrorResponse> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let (watches, total) = state.repo.list_watches_with_info_paginated(limit, offset)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let data: Vec<WatchResponse> = watches
        .into_iter()
        .map(|w| {
            let (product_id, product_name, store_name) = (w.product_id, w.product_name.clone(), w.store_name.clone());
            let (offer_url, current_price, currency, availability) =
                (w.offer_url.clone(), w.current_price, w.currency.clone(), w.availability);
            WatchResponse::from_domain(w.into_watch()).with_offer_info(
                product_id, product_name, store_name, offer_url, current_price, currency, availability,
            )
        })
        .collect();

    Ok(Json(PaginatedResponse::new(data, total, pagination.page(), limit)))
}

pub async fn create_watch(
    State(state): State<AppState>,
    Json(req): Json<CreateWatchRequest>,
) -> Result<Json<WatchResponse>, ErrorResponse> {
    let watch = state.watch_svc.create(req.offer_id, clamp_interval(req.interval_seconds))
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(WatchResponse::from_domain(watch)))
}

pub async fn update_watch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWatchRequest>,
) -> Result<Json<WatchResponse>, ErrorResponse> {
    let interval = req.interval_seconds.map(|v| v.clamp(MIN_INTERVAL, MAX_INTERVAL));
    let watch = state.watch_svc.update(id, req.enabled, interval)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(WatchResponse::from_domain(watch)))
}

pub async fn delete_watch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.watch_svc.delete(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn check_watch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.watch_svc.check(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "status": "check_started", "watch_id": id })))
}

/// Resets a CAPTCHA_REQUIRED or BLOCKED watch back to ACTIVE so the scheduler
/// retries it. The user calls this after manually solving the CAPTCHA / unblocking.
pub async fn resume_watch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WatchResponse>, ErrorResponse> {
    let watch = state.repo.find_watch(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found("Watch not found"))?;

    if !matches!(watch.status.as_str(), "CAPTCHA_REQUIRED" | "BLOCKED" | "ERROR") {
        return Err(ErrorResponse::bad_request("Watch is not in a resumable state"));
    }

    state.repo.set_watch_status(id, "ACTIVE").await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    // Reset failure count so it gets a fair chance
    sqlx::query("UPDATE watches SET failure_count = 0, next_check_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.repo.pool)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let updated = state.repo.find_watch(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found("Watch not found"))?;

    Ok(Json(WatchResponse::from_domain(updated)))
}
