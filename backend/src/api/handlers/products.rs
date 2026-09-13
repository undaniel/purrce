use std::collections::HashMap;

use axum::{extract::{State, Path, Query}, Json};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::api::dto::{ProductResponse, OfferResponse, ComparisonResponse, ImportRequest, ErrorResponse, ProductQueryParams, PaginatedResponse};
use crate::api::AppState;
use crate::domain::product::Product;
use crate::domain::store::ProductOffer;
use crate::domain::tag::Tag;
use crate::infrastructure::database::repository::Repository;

/// Minimum and maximum allowed check interval (1 minute .. 30 days).
pub const MIN_INTERVAL: i32 = 60;
pub const MAX_INTERVAL: i32 = 2_592_000;

pub fn clamp_interval(value: Option<i32>) -> i32 {
    value.unwrap_or(3600).clamp(MIN_INTERVAL, MAX_INTERVAL)
}

/// Loads offers and tags for a batch of products with two queries total instead
/// of two per product.
async fn with_relations(repo: &Repository, products: Vec<Product>) -> Vec<ProductResponse> {
    let ids: Vec<Uuid> = products.iter().map(|p| p.id).collect();

    let mut offers_by: HashMap<Uuid, Vec<ProductOffer>> = HashMap::new();
    for offer in repo.find_offers_by_products(&ids).await.unwrap_or_default() {
        offers_by.entry(offer.product_id).or_default().push(offer);
    }

    let mut tags_by: HashMap<Uuid, Vec<Tag>> = HashMap::new();
    for (product_id, tag) in repo.get_tags_for_products(&ids).await.unwrap_or_default() {
        tags_by.entry(product_id).or_default().push(tag);
    }

    products
        .into_iter()
        .map(|product| {
            let offers = offers_by.remove(&product.id).unwrap_or_default();
            let tags = tags_by.remove(&product.id).unwrap_or_default();
            ProductResponse::from_domain(product, offers, tags)
        })
        .collect()
}

#[derive(Serialize)]
pub struct CurrencyValueResponse {
    pub currency: String,
    pub total: f64,
    pub count: i64,
}

#[derive(Serialize)]
pub struct ProductStatsResponse {
    pub total: i64,
    pub down: i64,
    pub up: i64,
    pub unchanged: i64,
    pub at_min: i64,
    pub no_price: i64,
    pub values: Vec<CurrencyValueResponse>,
}

#[derive(Deserialize)]
pub struct AddOfferRequest {
    pub url: String,
    pub watch: Option<bool>,
    pub interval_seconds: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateOfferPriceRequest {
    pub price: f64,
    /// Accepted for backwards compatibility; the offer's stored currency wins.
    #[allow(dead_code)]
    pub currency: Option<String>,
    pub availability: Option<bool>,
}

pub async fn get_product_stats(State(state): State<AppState>) -> Result<Json<ProductStatsResponse>, ErrorResponse> {
    let ((total, down, up, unchanged), (at_min, no_price, values)) = tokio::try_join!(
        state.repo.get_product_stats(),
        state.repo.get_dashboard_extras(),
    )
    .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(ProductStatsResponse {
        total,
        down,
        up,
        unchanged,
        at_min,
        no_price,
        values: values
            .into_iter()
            .map(|(currency, total, count)| CurrencyValueResponse { currency, total, count })
            .collect(),
    }))
}

pub async fn list_products(
    State(state): State<AppState>,
    Query(params): Query<ProductQueryParams>,
) -> Result<Json<PaginatedResponse<ProductResponse>>, ErrorResponse> {
    let limit = params.limit();
    let offset = params.offset();
    let search = params.search.as_deref();
    let tag = params.tag.as_deref();

    let (products, total) = state.repo.search_products_paginated(search, tag, limit, offset)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let data = with_relations(&state.repo, products).await;

    Ok(Json(PaginatedResponse::new(data, total, params.page(), limit)))
}

pub async fn list_all_products(State(state): State<AppState>) -> Result<Json<Vec<ProductResponse>>, ErrorResponse> {
    let products = state.repo.list_products()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(with_relations(&state.repo, products).await))
}

pub async fn import_product(
    State(state): State<AppState>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ProductResponse>, ErrorResponse> {
    if url::Url::parse(&req.url).is_err() {
        return Err(ErrorResponse::bad_request("URL inválida"));
    }

    let interval = clamp_interval(req.interval_seconds);
    let watch = req.watch.unwrap_or(true);

    info!(url = %req.url, watch, interval, "POST /products/import started");
    let t = std::time::Instant::now();

    let (product, offer) = state.product_svc.import_from_url(
        &req.url, watch, interval,
        req.name.as_deref(), req.price, req.image_url.as_deref(),
    )
    .await
    .map_err(|e| {
        let msg = e.to_string();
        info!(url = %req.url, elapsed_ms = t.elapsed().as_millis(), error = %msg, "POST /products/import failed");
        if msg.contains("BLOCKED_CAPTCHA") || msg.contains("BLOCKED_ACCESS") {
            ErrorResponse::unprocessable_code("BLOCKED", "El sitio bloquea el acceso automático (Cloudflare / bot detection). Ingresa los datos manualmente.")
        } else if msg.contains("EXTRACTION_FAILED") {
            ErrorResponse::unprocessable_code("EXTRACTION_FAILED", "No se pudo extraer el precio del producto. Ingresa los datos manualmente.")
        } else {
            ErrorResponse::internal(msg)
        }
    })?;

    info!(url = %req.url, elapsed_ms = t.elapsed().as_millis(), product_id = %product.id, "POST /products/import completed");

    // Add tags if provided
    if let Some(tag_names) = &req.tags {
        for name in tag_names {
            let tag = state.repo.find_or_create_tag(name)
                .await
                .map_err(|e| ErrorResponse::internal(e.to_string()))?;
            let _ = state.repo.add_tag_to_product(product.id, tag.id).await;
        }
    }

    let tags = state.repo.get_product_tags(product.id)
        .await
        .unwrap_or_default();

    Ok(Json(ProductResponse::from_domain(product, vec![offer], tags)))
}

pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProductResponse>, ErrorResponse> {
    let (product, offers) = state.product_svc.get_product(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found("Product not found"))?;

    let tags = state.repo.get_product_tags(id)
        .await
        .unwrap_or_default();

    Ok(Json(ProductResponse::from_domain(product, offers, tags)))
}

pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    state.product_svc.delete_product(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn add_offer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddOfferRequest>,
) -> Result<Json<OfferResponse>, ErrorResponse> {
    if url::Url::parse(&req.url).is_err() {
        return Err(ErrorResponse::bad_request("URL inválida"));
    }

    let interval = clamp_interval(req.interval_seconds);
    let watch = req.watch.unwrap_or(true);

    let offer = state.product_svc.add_offer_to_product(id, &req.url, watch, interval)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Product not found") {
                ErrorResponse::not_found("Producto no encontrado")
            } else if msg.contains("Invalid URL") || msg.contains("No domain") {
                ErrorResponse::bad_request("URL inválida")
            } else {
                ErrorResponse::internal(msg)
            }
        })?;

    Ok(Json(OfferResponse::from_domain(offer)))
}

pub async fn get_comparison(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ComparisonResponse>, ErrorResponse> {
    let (product, offers) = state.product_svc.get_product(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found("Product not found"))?;

    let available_offers: Vec<_> = offers.iter()
        .filter(|o| o.current_price > 0.0 && o.availability)
        .collect();

    let min_price = available_offers.iter()
        .map(|o| o.current_price)
        .fold(f64::INFINITY, f64::min);

    let max_price = available_offers.iter()
        .map(|o| o.current_price)
        .fold(0.0_f64, f64::max);

    let best_offer = available_offers.iter()
        .min_by(|a, b| a.current_price.total_cmp(&b.current_price))
        .map(|o| OfferResponse::from_domain((*o).clone()));

    let savings = if min_price < f64::INFINITY && max_price > 0.0 {
        max_price - min_price
    } else {
        0.0
    };

    let mut sorted_offers = offers;
    sorted_offers.sort_by(|a, b| {
        // Offers with price 0 go to the end. total_cmp is NaN-safe.
        match (a.current_price > 0.0, b.current_price > 0.0) {
            (true, true) => a.current_price.total_cmp(&b.current_price),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => std::cmp::Ordering::Equal,
        }
    });

    let tags = state.repo.get_product_tags(id)
        .await
        .unwrap_or_default();

    Ok(Json(ComparisonResponse {
        product: ProductResponse::from_domain(product, vec![], tags),
        offers: sorted_offers.into_iter().map(OfferResponse::from_domain).collect(),
        best_offer,
        min_price: if min_price == f64::INFINITY { 0.0 } else { min_price },
        max_price,
        savings,
    }))
}

pub async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, ErrorResponse> {
    let product = state.repo.update_product(id, req.name.as_deref(), req.description.as_deref(), req.image_url.as_deref())
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let offers = state.repo.find_offers_by_product(id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let tags = state.repo.get_product_tags(id)
        .await
        .unwrap_or_default();

    Ok(Json(ProductResponse::from_domain(product, offers, tags)))
}

pub async fn update_offer_price(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOfferPriceRequest>,
) -> Result<Json<OfferResponse>, ErrorResponse> {
    let availability = req.availability.unwrap_or(true);

    let offer = state.repo.update_offer_price(id, req.price, availability)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    // Record against the offer's stored currency, not a request default.
    let _ = state.repo.insert_price_history(id, req.price, &offer.currency, availability).await;

    Ok(Json(OfferResponse::from_domain(offer)))
}
