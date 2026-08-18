//! HTTP admin catalog routes (`sak572-c`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_admin::{app_with_state, AppState};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn list_catalog_from_memory() {
    let app = app_with_state(AppState::new());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/catalog")
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
    assert_eq!(v["backend"], "memory");
    let offers = v["offers"].as_array().expect("offers");
    assert!(offers.iter().any(|o| o["id"] == "llm.chat"));
}

#[tokio::test]
async fn get_catalog_memory_found_and_missing() {
    let app = app_with_state(AppState::new());
    let ok = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/sak/catalog/llm.chat")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/catalog/no.such.offer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn list_catalog_from_pg_catalog_port() {
    use persist_postgres::ports::{CatalogStore, MemoryCatalog};
    use std::sync::Arc;

    let store = MemoryCatalog::new();
    store
        .upsert_offer("pg.only", "9.9.9", "postgres")
        .expect("upsert");
    let state = AppState::new().with_catalog_store(Arc::new(store));
    let app = app_with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/catalog")
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
    assert_eq!(v["backend"], "postgres");
    assert_eq!(v["offers"][0]["id"], "pg.only");
    assert_eq!(v["offers"][0]["origin"], "postgres");
}
