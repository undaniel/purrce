use axum::{extract::{State, Path, Query}, Json};
use uuid::Uuid;
use serde::Deserialize;
use crate::api::dto::{AlertResponse, ErrorResponse, PaginationParams, PaginatedResponse};
use crate::api::AppState;

#[derive(Deserialize)]
pub struct UpdateAlertRequest {
    pub enabled: Option<bool>,
    pub threshold_price: Option<f64>,
}

#[derive(Deserialize)]
pub struct CreateAlertRequest {
    pub offer_id: Uuid,
    pub alert_type: String,
    pub threshold_price: Option<f64>,
}

pub async fn create_alert(
    State(state): State<AppState>,
    Json(req): Json<CreateAlertRequest>,
) -> Result<Json<AlertResponse>, ErrorResponse> {
    let alert = state.repo.create_alert(req.offer_id, &req.alert_type, req.threshold_price)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(AlertResponse::from_domain(alert)))
}

pub async fn list_alerts(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<AlertResponse>>, ErrorResponse> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    // Use optimized JOIN query to avoid N+1
    let (rows, total) = state.repo.list_alerts_with_info_paginated(limit, offset)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let data: Vec<AlertResponse> = rows
        .into_iter()
        .map(|(alert, product_name, product_image, store_name, offer_url)| {
            AlertResponse::from_domain(alert)
                .with_product_info(
                    Some(product_name),
                    product_image,
                    Some(store_name),
                    Some(offer_url),
                )
        })
        .collect();

    Ok(Json(PaginatedResponse::new(data, total, pagination.page(), limit)))
}

pub async fn update_alert(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequest>,
) -> Result<Json<AlertResponse>, ErrorResponse> {
    let alert = state.repo.update_alert(id, req.enabled, req.threshold_price)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(AlertResponse::from_domain(alert)))
}
