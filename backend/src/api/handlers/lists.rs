use axum::{extract::{State, Path}, Json};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::api::AppState;
use crate::api::dto::ErrorResponse;
use crate::infrastructure::database::repository::{ProductList, ListItemRow};

#[derive(Deserialize)]
pub struct CreateListRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct RenameListRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct AddItemRequest {
    pub product_id: Uuid,
    pub quantity: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateQuantityRequest {
    pub quantity: i32,
}

#[derive(Serialize)]
pub struct ListWithTotal {
    pub list: ProductList,
    pub items: Vec<ListItemRow>,
    pub total_cost: f64,
}

pub async fn list_lists(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProductList>>, ErrorResponse> {
    let lists = state.repo.list_lists().await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(lists))
}

pub async fn create_list(
    State(state): State<AppState>,
    Json(req): Json<CreateListRequest>,
) -> Result<Json<ProductList>, ErrorResponse> {
    let list = state.repo.create_list(&req.name).await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(list))
}

pub async fn rename_list(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameListRequest>,
) -> Result<Json<ProductList>, ErrorResponse> {
    let list = state.repo.rename_list(id, &req.name).await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(list))
}

pub async fn delete_list(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.repo.delete_list(id).await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({"deleted": true})))
}

pub async fn get_list(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListWithTotal>, ErrorResponse> {
    let list = state.repo.find_list(id).await.map_err(|e| ErrorResponse::internal(e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found("Lista no encontrada"))?;
    let items = state.repo.list_items(id).await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let total_cost = items.iter()
        .map(|i| i.best_price.unwrap_or(0.0) * i.quantity as f64)
        .sum();
    Ok(Json(ListWithTotal { list, items, total_cost }))
}

pub async fn add_item(
    State(state): State<AppState>,
    Path(list_id): Path<Uuid>,
    Json(req): Json<AddItemRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.repo.add_list_item(list_id, req.product_id, req.quantity.unwrap_or(1))
        .await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn update_item_quantity(
    State(state): State<AppState>,
    Path((list_id, product_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateQuantityRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    if req.quantity < 1 {
        return Err(ErrorResponse::bad_request("Cantidad debe ser al menos 1"));
    }
    state.repo.update_list_item_quantity(list_id, product_id, req.quantity)
        .await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn remove_item(
    State(state): State<AppState>,
    Path((list_id, product_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.repo.remove_list_item(list_id, product_id)
        .await.map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
