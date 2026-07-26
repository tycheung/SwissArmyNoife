//! HTTP admin routes smoke (`sak363`).

use std::fs;
use std::sync::Mutex;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_admin::app;
use module_registry::install_and_pin;
use tower::ServiceExt;

static LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
#[allow(clippy::await_holding_lock, clippy::too_many_lines)]
async fn health_and_modules() {
    let _g = LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("CONFIG_DIR", tmp.path().join("cfg"));

    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/community.echo");
    install_and_pin(&src, "path").expect("install");

    let app = app();

    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/sak/modules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(listed.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("community.echo"), "{text}");

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/sak/modules/community.echo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(detail.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("\"version\""), "{text}");

    let cap = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/sak/capacity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cap.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(cap.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        text.contains("sysinfo") || text.contains("total_ram_mb"),
        "{text}"
    );

    let work = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/sak/compute/work")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(work.status(), StatusCode::OK);

    let nodes = app
        .oneshot(
            Request::builder()
                .uri("/v1/sak/compute/nodes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(nodes.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(nodes.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("\"nodes\""), "{text}");

    let _ = fs::remove_dir_all(tmp.path());
    std::env::remove_var("CONFIG_DIR");
}

#[tokio::test]
#[allow(clippy::await_holding_lock, clippy::too_many_lines)]
async fn compute_work_and_nodes_post_roundtrip() {
    let _g = LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("CONFIG_DIR", tmp.path().join("cfg"));

    let app = app();

    let enqueue = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"action":"enqueue","kind":"echo","payload":{"n":1}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enqueue.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(enqueue.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let enq: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(enq.get("work").is_some(), "{enq}");
    let work_id = enq["work"]["id"].as_str().expect("work.id");

    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/nodes")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"action":"register","label":"http-test","caps":["t"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(register.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let reg: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let node_id = reg["node"]["id"].as_str().expect("node.id");

    let claim_body = format!(r#"{{"action":"claim","node_id":"{node_id}"}}"#);
    let claim = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(claim_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(claim.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(claim.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let claimed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(claimed["work"]["id"], work_id, "{claimed}");

    let complete_body = format!(
        r#"{{"action":"complete","work_id":"{work_id}","node_id":"{node_id}","result":{{"ok":true}}}}"#
    );
    let complete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(complete_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(complete.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(complete.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let done: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(done.get("work").is_some(), "{done}");

    let list_body =
        r#"{"action":"list","run_id":"r-filter","stage_name":"echo","limit":50}"#.to_string();
    // enqueue a second unit with run_id in payload for list filter
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"action":"enqueue","kind":"echo","payload":{"run_id":"r-filter","stage_name":"echo"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(list_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(listed.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let list_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !list_json["work"].as_array().unwrap().is_empty(),
        "{list_json}"
    );

    let heartbeat_body = format!(r#"{{"action":"heartbeat","node_id":"{node_id}"}}"#);
    let beat = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/nodes")
                .header("content-type", "application/json")
                .body(Body::from(heartbeat_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(beat.status(), StatusCode::OK);

    let _ = fs::remove_dir_all(tmp.path());
    std::env::remove_var("CONFIG_DIR");
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn compute_work_enqueue_claim_requeue_roundtrip() {
    // sak430-c: AppState ComputePlane requeue after claim
    let _g = LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("CONFIG_DIR", tmp.path().join("cfg"));

    let app = app();

    let enqueue = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"action":"enqueue","kind":"echo","payload":{"n":430}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enqueue.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(enqueue.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let enq: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let work_id = enq["work"]["id"].as_str().expect("work.id").to_owned();

    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/nodes")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"action":"register","label":"requeue-test","caps":["t"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(register.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let reg: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let node_id = reg["node"]["id"].as_str().expect("node.id");

    let claim_body = format!(r#"{{"action":"claim","node_id":"{node_id}"}}"#);
    let claim = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(claim_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(claim.status(), StatusCode::OK);

    let requeue_body = format!(r#"{{"action":"requeue","work_id":"{work_id}"}}"#);
    let requeue = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sak/compute/work")
                .header("content-type", "application/json")
                .body(Body::from(requeue_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(requeue.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(requeue.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let rq: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(rq["action"], "requeue", "{rq}");
    assert_eq!(rq["work"]["id"], work_id, "{rq}");
    assert_eq!(rq["via"], "app_state_compute_plane", "{rq}");
    let status = rq["work"]["status"].as_str().unwrap_or("");
    assert!(
        status == "queued" || status == "pending",
        "expected queued after requeue, got {rq}"
    );

    let _ = fs::remove_dir_all(tmp.path());
    std::env::remove_var("CONFIG_DIR");
}
