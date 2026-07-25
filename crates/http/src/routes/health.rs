//! `GET /health` and `GET /v1/sak/health` (`sak363-a`, `sak060-b`).

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

use crate::state::AppState;

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "service": "swissarmynoife-http" }))
}

async fn sak_health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "offers": 0,
        "bindings": 0,
        "policy": "ambient",
    }))
}

#[must_use]
pub fn health_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/v1/sak/health", get(sak_health))
}
