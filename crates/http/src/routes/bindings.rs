//! Binding admin endpoints (`sak067-a`).

use std::time::UNIX_EPOCH;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use control::BindingRecord;
use serde::Serialize;
use serde_json::{json, Value};
use types::BindingId;

use crate::state::AppState;

#[derive(Serialize)]
struct BindingSummary {
    binding_id: String,
    offer_id: String,
    principal: String,
    principal_kind: String,
    expires_at: u64,
}

fn summary(record: &BindingRecord) -> BindingSummary {
    BindingSummary {
        binding_id: record.binding_id.to_string(),
        offer_id: record.offer_id.as_str().to_owned(),
        principal: record.principal.id.clone(),
        principal_kind: record.principal.kind.as_str().to_owned(),
        expires_at: record
            .expires_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

async fn list_bindings(State(state): State<AppState>) -> Json<Value> {
    let store = state.bindings.lock().expect("bindings lock");
    let bindings: Vec<_> = store.list().into_iter().map(summary).collect();
    Json(json!({ "bindings": bindings }))
}

async fn get_binding(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let binding_id = uuid::Uuid::parse_str(&id)
        .map(BindingId::from_uuid)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let store = state.bindings.lock().expect("bindings lock");
    let record = store.get(binding_id).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(json!(summary(record))))
}

pub fn bindings_router() -> Router<AppState> {
    Router::new()
        .route("/v1/sak/bindings", get(list_bindings))
        .route("/v1/sak/bindings/{id}", get(get_binding))
}
