//! HTTP `OpenAI` chat completions facade (`sak540-b` / `sak540-c`).

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
async fn chat_completions_stream_returns_sse() {
    let state = AppState::new();
    let binding_id = state.bind_llm_chat_for_test(300);
    let app = app_with_state(state);

    let stream = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "binding_id": binding_id.to_string(),
                        "model": "echo",
                        "stream": true,
                        "messages": [{ "role": "user", "content": "ping" }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("stream");
    assert_eq!(stream.status(), StatusCode::OK);
    let ctype = stream
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ctype.starts_with("text/event-stream"),
        "content-type={ctype}"
    );
    let cache = stream
        .headers()
        .get("cache-control")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(cache, "no-cache");
    let body = axum::body::to_bytes(stream.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(text.contains("chat.completion.chunk"), "{text}");
    assert!(text.contains("ping"), "{text}");
    assert!(text.contains("data: [DONE]"), "{text}");
}

#[tokio::test]
async fn chat_completions_stream_offer_error_is_sse() {
    let state = AppState::new();
    let binding_id = state.bind_llm_chat_for_test(300);
    let app = app_with_state(state);

    let stream = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "binding_id": binding_id.to_string(),
                        "model": "echo",
                        "stream": true,
                        "messages": [{ "role": "bogus", "content": "x" }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("stream");
    assert_eq!(stream.status(), StatusCode::OK);
    let body = axum::body::to_bytes(stream.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(text.contains("\"error\""), "{text}");
    assert!(
        text.contains("schema.invalid") || text.contains("unknown message role"),
        "{text}"
    );
    assert!(!text.contains("sk-"), "{text}");
    assert!(text.contains("data: [DONE]"), "{text}");
}

#[tokio::test]
async fn chat_completions_stream_binding_miss_is_json() {
    let app = app_with_state(AppState::new());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "binding_id": "00000000-0000-0000-0000-000000000001",
                        "model": "echo",
                        "stream": true,
                        "messages": [{ "role": "user", "content": "x" }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("miss");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert!(v["error"]["code"].as_str().is_some(), "{v}");
}

#[tokio::test]
async fn chat_completions_requires_binding() {
    let app = app_with_state(AppState::new());
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

#[tokio::test]
async fn chat_completions_tools_reject_stream() {
    let state = AppState::new();
    let tools_binding = state.bind_tools_loop_for_test(300);
    let app = app_with_state(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tools_binding_id": tools_binding.to_string(),
                        "stream": true,
                        "messages": [{
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_1",
                                "function": {
                                    "name": "tools.echo",
                                    "arguments": "{\"message\":\"hi\"}"
                                }
                            }]
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("post");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["error"]["code"], "stream_not_supported");
}

#[tokio::test]
async fn chat_completions_rate_limit_is_429() {
    let state = AppState::new().with_rate_limit_per_min(1.0);
    let binding_id = state.bind_llm_chat_for_test(300);
    let app = app_with_state(state);

    let body = json!({
        "binding_id": binding_id.to_string(),
        "model": "echo",
        "messages": [{ "role": "user", "content": "a" }]
    })
    .to_string();

    let ok = app
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
        .expect("first");
    assert_eq!(ok.status(), StatusCode::OK);

    let limited = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("second");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let bytes = axum::body::to_bytes(limited.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(v["error"]["type"], "rate_limit_error");
    assert_eq!(v["error"]["code"], "budget.exhausted");
}

#[tokio::test]
async fn chat_completions_refuses_multimodal_content() {
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
                        "messages": [{
                            "role": "user",
                            "content": [
                                { "type": "text", "text": "hi" },
                                { "type": "image_url", "image_url": { "url": "https://x" } }
                            ]
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("post");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["error"]["code"], "schema.invalid");
}

#[tokio::test]
async fn chat_completions_tools_round_trip() {
    let state = AppState::new();
    let tools_binding = state.bind_tools_loop_for_test(300);
    let app = app_with_state(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tools_binding_id": tools_binding.to_string(),
                        "messages": [{
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "tools.echo",
                                    "arguments": "{\"message\":\"hi\"}"
                                }
                            }]
                        }]
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
    assert_eq!(v["choices"][0]["finish_reason"], "tool_calls");
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .expect("content");
    assert!(content.contains("hi"), "{content}");
    assert!(
        content.contains("\"ok\":true") || content.contains("\"ok\": true"),
        "{content}"
    );
}

#[tokio::test]
async fn chat_completions_tools_require_binding() {
    let app = app_with_state(AppState::new());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "messages": [{
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_1",
                                "function": {
                                    "name": "tools.echo",
                                    "arguments": "{}"
                                }
                            }]
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("post");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["error"]["code"], "tools_binding_required");
}
