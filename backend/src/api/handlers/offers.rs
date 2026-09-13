use axum::{extract::{State, Path, Query}, Json};
use serde::Deserialize;
use uuid::Uuid;
use crate::api::dto::{OfferResponse, PriceHistoryResponse, ErrorResponse, PaginationParams, PaginatedResponse};
use crate::api::AppState;

pub async fn get_offer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OfferResponse>, ErrorResponse> {
    let offer = state.repo.find_offer(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found("Offer not found"))?;

    Ok(Json(OfferResponse::from_domain(offer)))
}

pub async fn get_offer_history(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PriceHistoryResponse>>, ErrorResponse> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let (history, total) = state.repo.list_price_history_paginated(id, limit, offset)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(PaginatedResponse::new(
        history.into_iter().map(PriceHistoryResponse::from_domain).collect(),
        total,
        pagination.page(),
        limit,
    )))
}

#[derive(Deserialize)]
pub struct BulkHistoryParams {
    /// Comma-separated offer IDs.
    pub ids: String,
    /// Max samples per offer (default 120, clamped 2..=500).
    pub limit: Option<i64>,
}

/// GET /api/price-history?ids=id1,id2&limit=120
///
/// Bulk price history for many offers in one request. Used by the UI to
/// compute buy signals for product lists without N+1 calls.
pub async fn get_offers_history_bulk(
    State(state): State<AppState>,
    Query(params): Query<BulkHistoryParams>,
) -> Result<Json<Vec<PriceHistoryResponse>>, ErrorResponse> {
    let ids: Vec<Uuid> = params
        .ids
        .split(',')
        .filter_map(|s| Uuid::parse_str(s.trim()).ok())
        .collect();

    if ids.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let limit = params.limit.unwrap_or(120).clamp(2, 500);

    let history = state
        .repo
        .list_price_history_for_offers(&ids, limit)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(
        history
            .into_iter()
            .map(PriceHistoryResponse::from_domain)
            .collect(),
    ))
}
