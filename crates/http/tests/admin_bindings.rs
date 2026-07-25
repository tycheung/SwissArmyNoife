//! HTTP admin binding routes (`sak067-a`).

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use control::{BindRequest, CatalogEntry};
use http_admin::{app_with_state, AppState};
use serde_json::Value;
use tower::ServiceExt;
use types::OfferId;

fn state_with_binding() -> AppState {
    let state = AppState::new();
    state
        .catalog
        .lock()
        .expect("catalog")
        .register(CatalogEntry::new("llm.chat", "0.1.0").expect("id"));
    let binding_id = {
        let mut store = state.bindings.lock().expect("lock");
        store
            .bind(BindRequest {
                offer_id: OfferId::new("llm.chat").expect("valid"),
                principal: control::Principal::local(),
                policy_json: serde_json::json!({}),
                ttl: Duration::from_secs(3600),
            })
            .binding_id
    };
    let _ = binding_id;
    state
}

#[tokio::test]
async fn list_bindings_empty_and_populated() {
    let empty = app_with_state(AppState::new());
    let resp = empty
        .oneshot(
            Request::builder()
                .uri("/v1/sak/bindings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["bindings"].as_array().map(|a| a.len()), Some(0));

    let populated = app_with_state(state_with_binding());
    let resp = populated
        .oneshot(
            Request::builder()
                .uri("/v1/sak/bindings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let row = &v["bindings"][0];
    assert_eq!(row["offer_id"], "llm.chat");
    assert_eq!(row["principal"], "local");
    assert_eq!(row["principal_kind"], "local");
    assert!(row.get("policy_json").is_none());
}

#[tokio::test]
async fn get_binding_found_and_missing() {
    let state = state_with_binding();
    let id = {
        let store = state.bindings.lock().expect("lock");
        store.list()[0].binding_id.to_string()
    };
    let app = app_with_state(state);
    let ok = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/sak/bindings/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/bindings/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn metrics_jsonl_export() {
    let state = AppState::new();
    *state.invoke_count.lock().expect("lock") = 7;
    let app = app_with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("\"invokes_total\""));
    assert!(text.contains("\"value\":7"));
}
