//! HTTP `OpenAI` chat completions facade (`sak540-b`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_admin::{app_with_state, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn chat_completions_maps_to_llm_chat() {
    let state = AppState::new();
    let binding_id = state.bind_llm_chat_for_test(300);
    let app = app_with_state(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "binding_id": binding_id.to_string(),
                        "model": "echo",
                        "messages": [{ "role": "user", "content": "ping" }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("post");
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["object"], "chat.completion");
    assert_eq!(v["choices"][0]["message"]["role"], "assistant");
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .expect("content");
    assert!(content.contains("ping"), "{content}");
}

#[tokio::test]
async fn chat_completions_rejects_stream_and_missing_binding() {
    let state = AppState::new();
    let binding_id = state.bind_llm_chat_for_test(300);
    let app = app_with_state(state);

    let stream = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "binding_id": binding_id.to_string(),
                        "stream": true,
                        "messages": [{ "role": "user", "content": "x" }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("stream");
    assert_eq!(stream.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(stream.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["error"]["code"], "stream_not_supported");

    let missing = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "messages": [{ "role": "user", "content": "x" }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("missing");
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(missing.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["error"]["code"], "binding_required");
}
