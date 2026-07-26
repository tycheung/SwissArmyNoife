//! HTTP vault connections admin (`sak527-a` / `sak527-b`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_admin::{app_with_state, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

fn boot_app() -> (axum::Router, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("tmp");
    std::env::set_var(persist_sqlite::CONFIG_DIR, tmp.path());
    std::env::set_var(
        vault::VAULT_KEY,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    let state = AppState::from_env();
    assert!(state.vault.is_some(), "vault should open under CONFIG_DIR");
    (app_with_state(state), tmp)
}

async fn post_conn(app: axum::Router, id: &str) -> Value {
    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/connections")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "connection_id": id,
                        "provider": "openai",
                        "label": "prod",
                        "secret": "sk-live-secret-value"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("post");
    assert_eq!(create.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(create.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    serde_json::from_slice(&body).expect("json")
}

#[tokio::test]
async fn post_and_list_connections_never_echo_secret() {
    let (app, _tmp) = boot_app();
    let created = post_conn(app.clone(), "c1").await;
    assert_eq!(created["connection_id"], "c1");
    assert!(!created.to_string().contains("sk-live"));
    assert!(created.get("secret").is_none());

    let list = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sak/connections")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list");
    assert_eq!(list.status(), StatusCode::OK);
    let body = axum::body::to_bytes(list.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let listed: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(listed["connections"].as_array().expect("arr").len(), 1);
    assert!(!listed.to_string().contains("sk-live"));
}

#[tokio::test]
async fn get_and_delete_connection_by_id() {
    let (app, _tmp) = boot_app();
    let _ = post_conn(app.clone(), "c1").await;

    let got = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sak/connections/c1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get");
    assert_eq!(got.status(), StatusCode::OK);
    let body = axum::body::to_bytes(got.into_body(), 64 * 1024)
        .await
        .expect("bytes");
    let one: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(one["connection_id"], "c1");
    assert!(!one.to_string().contains("sk-live"));

    let miss = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sak/connections/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("miss");
    assert_eq!(miss.status(), StatusCode::NOT_FOUND);

    let del = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/sak/connections/c1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("del");
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    let gone = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/sak/connections/c1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("gone");
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
}
