//! Catalog admin endpoints (`sak572-c`).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::state::AppState;

async fn list_catalog(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    #[cfg(feature = "postgres")]
    if let Some(pg) = &state.pg_catalog {
        let rows = pg
            .list_offers()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let offers: Vec<_> = rows
            .into_iter()
            .map(|o| {
                json!({
                    "id": o.offer_id,
                    "version": o.version,
                    "origin": o.origin,
                })
            })
            .collect();
        return Ok(Json(json!({ "offers": offers, "backend": "postgres" })));
    }

    let catalog = state
        .catalog
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let offers: Vec<_> = catalog
        .list()
        .into_iter()
        .map(|e| {
            json!({
                "id": e.id.as_str(),
                "version": e.version,
            })
        })
        .collect();
    Ok(Json(json!({ "offers": offers, "backend": "memory" })))
}

async fn get_catalog(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    #[cfg(feature = "postgres")]
    if let Some(pg) = &state.pg_catalog {
        let row = pg
            .get_offer(&id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let Some(o) = row else {
            return Err(StatusCode::NOT_FOUND);
        };
        return Ok(Json(json!({
            "id": o.offer_id,
            "version": o.version,
            "origin": o.origin,
            "backend": "postgres",
        })));
    }

    let offer_id = types::OfferId::new(id).map_err(|_| StatusCode::NOT_FOUND)?;
    let catalog = state
        .catalog
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let entry = catalog.get(&offer_id).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(json!({
        "id": entry.id.as_str(),
        "version": entry.version,
        "backend": "memory",
    })))
}

pub fn catalog_router() -> Router<AppState> {
    Router::new()
        .route("/v1/sak/catalog", get(list_catalog))
        .route("/v1/sak/catalog/{id}", get(get_catalog))
}
