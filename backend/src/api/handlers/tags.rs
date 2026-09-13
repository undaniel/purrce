use axum::{extract::{State, Path}, Json};
use uuid::Uuid;
use serde::Deserialize;
use crate::api::dto::{TagResponse, TagUsageResponse, ErrorResponse};
use crate::api::AppState;

#[derive(Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Deserialize)]
pub struct MergeTagRequest {
    pub target_id: Uuid,
}

#[derive(Deserialize)]
pub struct UpdateProductTagsRequest {
    pub tags: Vec<String>,
}

pub async fn list_tags(State(state): State<AppState>) -> Result<Json<Vec<TagResponse>>, ErrorResponse> {
    let tags = state.repo.list_tags()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(tags.into_iter().map(TagResponse::from_domain).collect()))
}

pub async fn create_tag(
    State(state): State<AppState>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<TagResponse>, ErrorResponse> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ErrorResponse::bad_request("Tag name cannot be empty"));
    }

    if state.repo.find_tag_by_name(name)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .is_some()
    {
        return Err(ErrorResponse::bad_request("Ya existe un tag con ese nombre"));
    }

    let tag = state.repo.create_tag(name, req.color.as_deref())
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(TagResponse::from_domain(tag)))
}

/// GET /api/tags/usage — tags with product counts, for the tag manager.
pub async fn list_tags_usage(
    State(state): State<AppState>,
) -> Result<Json<Vec<TagUsageResponse>>, ErrorResponse> {
    let tags = state.repo.list_tags_with_usage()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(tags.into_iter().map(|(id, name, color, product_count)| TagUsageResponse {
        id,
        name,
        color,
        product_count,
    }).collect()))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<Json<TagResponse>, ErrorResponse> {
    let name = req.name.as_deref().map(str::trim).filter(|s| !s.is_empty());

    if req.name.is_some() && name.is_none() {
        return Err(ErrorResponse::bad_request("Tag name cannot be empty"));
    }

    if let Some(new_name) = name {
        if let Some(existing) = state.repo.find_tag_by_name(new_name)
            .await
            .map_err(|e| ErrorResponse::internal(e.to_string()))?
        {
            if existing.id != id {
                return Err(ErrorResponse::bad_request(
                    "Ya existe un tag con ese nombre. Usa Fusionar para unirlos.",
                ));
            }
        }
    }

    let tag = state.repo.update_tag(id, name, req.color.as_deref())
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(TagResponse::from_domain(tag)))
}

/// POST /api/tags/{id}/merge — move products from this tag into `target_id`.
pub async fn merge_tag(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<MergeTagRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    if id == req.target_id {
        return Err(ErrorResponse::bad_request("Cannot merge a tag into itself"));
    }

    let target_exists = state.repo.list_tags()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .into_iter()
        .any(|t| t.id == req.target_id);

    if !target_exists {
        return Err(ErrorResponse::not_found("Target tag not found"));
    }

    state.repo.merge_tags(id, req.target_id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "merged": true })))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.repo.delete_tag(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn get_product_tags(
    State(state): State<AppState>,
    Path(product_id): Path<Uuid>,
) -> Result<Json<Vec<TagResponse>>, ErrorResponse> {
    let tags = state.repo.get_product_tags(product_id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(tags.into_iter().map(TagResponse::from_domain).collect()))
}

pub async fn set_product_tags(
    State(state): State<AppState>,
    Path(product_id): Path<Uuid>,
    Json(req): Json<UpdateProductTagsRequest>,
) -> Result<Json<Vec<TagResponse>>, ErrorResponse> {
    let mut tag_ids = Vec::new();
    for name in &req.tags {
        let tag = state.repo.find_or_create_tag(name)
            .await
            .map_err(|e| ErrorResponse::internal(e.to_string()))?;
        tag_ids.push(tag.id);
    }

    state.repo.set_product_tags(product_id, &tag_ids)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let tags = state.repo.get_product_tags(product_id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(tags.into_iter().map(TagResponse::from_domain).collect()))
}
