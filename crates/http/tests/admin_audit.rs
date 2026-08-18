//! HTTP audit list/query (`sak528-a`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use control::AuditStatus;
use http_admin::{app_with_state, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;
use types::{BindingId, InvokeId, InvokeResp, OfferId};
use uuid::Uuid;

#[tokio::test]
async fn audit_list_filters_and_keeps_redaction() {
    let state = AppState::new();
    {
        let mut audit = state.audit.lock().expect("lock");
        let id = InvokeId::from_uuid(Uuid::nil());
        let binding = BindingId::from_uuid(Uuid::from_u128(1));
        let offer = OfferId::new("llm.chat").expect("id");
        audit.record_invoke(
            id,
            binding,
            offer,
            &json!({ "api_key": "sk-live-secret", "q": "hi" }),
            &InvokeResp::ok(id, json!({})),
        );
        assert_eq!(audit.list_active().len(), 1);
        assert_eq!(audit.list_active()[0].status, AuditStatus::Ok);
    }
    let app = app_with_state(state);

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sak/audit?offer_id=llm.chat")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list");
    assert_eq!(list.status(), StatusCode::OK);
    let body = axum::body::to_bytes(list.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["events"].as_array().expect("arr").len(), 1);
    assert_eq!(v["events"][0]["detail"]["args"]["api_key"], "[REDACTED]");
    assert!(!v.to_string().contains("sk-live"));

    let miss = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sak/audit?offer_id=other")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("miss");
    let body = axum::body::to_bytes(miss.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert!(v["events"].as_array().expect("arr").is_empty());
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn audit_list_from_pg_audit_store() {
    use persist_postgres::ports::{AuditEventRow, AuditStore, MemoryAuditStore};
    use std::sync::Arc;

    let store = MemoryAuditStore::new();
    store
        .append_event(&AuditEventRow {
            event_id: "00000000-0000-0000-0000-0000000000ae".into(),
            binding_id: "00000000-0000-0000-0000-0000000000ab".into(),
            kind: "llm.embed".into(),
            recorded_at_unix: 1_700_000_000,
        })
        .expect("append");
    let state = AppState::new().with_audit_store(Arc::new(store));
    let app = app_with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/audit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["backend"], "postgres");
    assert_eq!(v["events"][0]["offer_id"], "llm.embed");
}
