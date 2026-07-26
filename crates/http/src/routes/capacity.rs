//! `GET /v1/sak/capacity` local probe (`refactor` + sak274 surface).

use axum::{routing::get, Json, Router};
use offer_capacity::{HardwareProbe, LocalSysProbe};
use serde_json::{json, Value};

use crate::state::AppState;

async fn get_capacity() -> Json<Value> {
    let snap = LocalSysProbe.probe().expect("local snapshot");
    Json(json!({
        "snapshot": snap,
    }))
}

pub fn capacity_router() -> Router<AppState> {
    Router::new().route("/v1/sak/capacity", get(get_capacity))
}
