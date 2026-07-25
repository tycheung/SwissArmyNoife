//! `GET /v1/sak/health` smoke (`sak060-b`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_admin::app;
use tower::ServiceExt;

#[tokio::test]
async fn sak_health_returns_broker_snapshot() {
    let app = app();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["offers"], 0);
    assert_eq!(v["bindings"], 0);
    assert_eq!(v["policy"], "ambient");
}
