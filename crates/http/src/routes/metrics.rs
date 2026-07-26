//! Meter JSONL export (`sak066-a`).

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::state::AppState;

async fn metrics(State(state): State<AppState>) -> Response {
    let body = state.meter_snapshot().to_jsonl();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        body,
    )
        .into_response()
}

pub fn metrics_router() -> Router<AppState> {
    Router::new().route("/v1/sak/metrics", get(metrics))
}
