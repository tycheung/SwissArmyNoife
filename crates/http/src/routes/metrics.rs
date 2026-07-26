//! Meter JSONL + Prometheus text export (`sak066-a` / `sak528-d`).

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

async fn metrics_prometheus(State(state): State<AppState>) -> Response {
    let body = state.meter_snapshot().to_prometheus();
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

pub fn metrics_router() -> Router<AppState> {
    Router::new()
        .route("/v1/sak/metrics", get(metrics))
        .route("/metrics", get(metrics_prometheus))
        .route("/v1/sak/metrics/prometheus", get(metrics_prometheus))
}
