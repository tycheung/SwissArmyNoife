//! HTTP admin bearer auth integration (`sak541-c`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_admin::{app_with_state, AppState};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn chat_completions_401_without_bearer_when_token_set() {
    let state = AppState::new().with_http_token("secret-tok");
    let binding = state.bind_llm_chat_for_test(60);
    let app = app_with_state(state);
    let body = json!({
        "binding_id": binding.to_string(),
        "model": "echo",
        "messages": [{ "role": "user", "content": "x" }]
    })
    .to_string();

    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .expect("denied");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let allowed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret-tok")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("allowed");
    let status = allowed.status();
    let bytes = axum::body::to_bytes(allowed.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    assert_eq!(
        status,
        StatusCode::OK,
        "expected chat ok with bearer, body={}",
        String::from_utf8_lossy(&bytes)
    );
}

#[tokio::test]
async fn health_401_without_bearer_when_token_set() {
    let app = app_with_state(AppState::new().with_http_token("t"));
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("health");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
