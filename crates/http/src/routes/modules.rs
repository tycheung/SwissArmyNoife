//! Installed module admin endpoints (`sak363-b` / `sak363-c`).

use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use module_registry::{get_installed, list_installed};
use serde::Serialize;
use serde_json::{json, Value};

use crate::state::AppState;

#[derive(Serialize)]
struct ModuleSummary {
    id: String,
    version: String,
    origin: String,
    runtime: String,
    root: String,
}

async fn list_modules() -> Result<Json<Value>, StatusCode> {
    let items = list_installed().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let modules: Vec<ModuleSummary> = items
        .into_iter()
        .map(|m| ModuleSummary {
            id: m.manifest.id,
            version: m.manifest.version,
            origin: m.manifest.origin.as_str().to_owned(),
            runtime: m.manifest.runtime.as_str().to_owned(),
            root: m.root.display().to_string(),
        })
        .collect();
    Ok(Json(json!({ "modules": modules })))
}

async fn get_module(Path(id): Path<String>) -> Result<Json<Value>, StatusCode> {
    let m = get_installed(&id, None).map_err(|e| match e {
        types::ErrorCode::OfferNotFound => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    Ok(Json(json!({
        "id": m.manifest.id,
        "version": m.manifest.version,
        "origin": m.manifest.origin.as_str(),
        "runtime": m.manifest.runtime.as_str(),
        "api_version": m.manifest.api_version,
        "payload": m.manifest.payload,
        "root": m.root.display().to_string(),
    })))
}

pub fn modules_router() -> Router<AppState> {
    Router::new()
        .route("/v1/sak/modules", get(list_modules))
        .route("/v1/sak/modules/{id}", get(get_module))
}
