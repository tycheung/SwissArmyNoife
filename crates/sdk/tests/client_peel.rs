//! Peel/assert helpers + `SakClient` HTTP smoke (moved from `client.rs` for LOC gate).

use sdk::*;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn normalize_claim_empty_vs_miss() {
    let empty = normalize_claim_work_response(&json!({"work": null, "error": "queue empty"}))
        .expect("empty");
    assert_eq!(empty["via"], "broker");
    assert!(empty["work"].is_null());
    assert!(normalize_claim_work_response(&json!({"work": null, "error": "down"})).is_err());
    // sak488-i: via=broker_miss never soft-polls, even with empty-queue error text
    assert!(normalize_claim_work_response(
        &json!({"via": "broker_miss", "work": null, "error": "queue empty"})
    )
    .is_err());
}

#[test]
fn assert_record_ok_write_path_empty_vs_miss() {
    // sak492-h: enqueue/complete/get/requeue — record ok; null/missing/broker_miss hard miss
    assert!(assert_record_ok(&json!({"work": {"id": "w1"}}), "work").is_ok());
    assert!(assert_record_ok(&json!({"id": "w1"}), "work").is_ok());
    assert!(assert_record_ok(&json!({"work": null}), "work").is_err());
    assert!(assert_record_ok(&json!({"work": null, "error": "queue empty"}), "work").is_err());
    assert!(assert_record_ok(&json!({}), "work").is_err());
    assert!(assert_record_ok(
        &json!({"via": "broker_miss", "status": "degraded", "feature": "enqueue"}),
        "work"
    )
    .is_err());
    assert!(assert_record_ok(
        &json!({"via": "broker_miss", "status": "degraded", "feature": "complete"}),
        "work"
    )
    .is_err());
}

#[test]
fn assert_record_ok_get_requeue_miss_vs_record() {
    // sak490-i: get/requeue — record ok; null/missing/broker_miss are hard misses (no soft empty)
    assert!(assert_record_ok(&json!({"work": {"id": "w1"}}), "work").is_ok());
    assert!(assert_record_ok(&json!({"id": "w1"}), "work").is_ok());
    assert!(assert_record_ok(&json!({"work": null}), "work").is_err());
    assert!(assert_record_ok(&json!({}), "work").is_err());
    assert!(assert_record_ok(
        &json!({"via": "broker_miss", "status": "degraded", "feature": "get"}),
        "work"
    )
    .is_err());
    assert!(assert_record_ok(
        &json!({"via": "broker_miss", "status": "degraded", "feature": "requeue"}),
        "work"
    )
    .is_err());
}

#[test]
fn assert_list_ok_empty_vs_null() {
    assert!(assert_list_ok(&json!({"nodes": []}), "nodes").is_ok());
    assert!(assert_list_ok(&json!({"work": []}), "work").is_ok());
    assert!(assert_list_ok(&json!({"nodes": null}), "nodes").is_err());
    assert!(assert_list_ok(&json!({"error": "x", "nodes": []}), "nodes").is_err());
    assert!(assert_list_ok(
        // sak488-i / sak490-i
        &json!({"via": "broker_miss", "work": [], "status": "degraded"}),
        "work"
    )
    .is_err());
    assert!(assert_list_ok(
        // sak491-j
        &json!({"via": "broker_miss", "nodes": [], "status": "degraded"}),
        "nodes"
    )
    .is_err());
}

#[test]
fn assert_record_ok_nested_and_top_level() {
    assert!(assert_record_ok(&json!({"work": {"id": "w1"}}), "work").is_ok());
    assert!(assert_record_ok(&json!({"id": "w1"}), "work").is_ok());
    assert!(assert_record_ok(&json!({"error": "down"}), "work").is_err());
    assert!(assert_record_ok(
        // sak487-i
        &json!({"via": "broker_miss", "status": "degraded", "feature": "enqueue"}),
        "work"
    )
    .is_err());
    assert!(assert_record_ok(&json!({"node": {"id": "n1"}}), "node").is_ok());
    assert!(assert_record_ok(&json!({"nodes": []}), "node").is_err());
    assert!(assert_record_ok(
        // sak484-i / sak491-j
        &json!({"via": "broker_miss", "status": "degraded", "feature": "register"}),
        "node"
    )
    .is_err());
    assert!(assert_record_ok(
        // sak491-j
        &json!({"via": "broker_miss", "status": "degraded", "feature": "heartbeat"}),
        "node"
    )
    .is_err());
}

#[test]
fn assert_list_ok_rejects_module_miss() {
    assert!(assert_list_ok(
        // sak484-i
        &json!({"via": "broker_miss", "status": "degraded", "feature": "list_modules"}),
        "modules"
    )
    .is_err());
}

#[test]
fn assert_capacity_ok_empty_vs_miss() {
    // sak493-h: health/capacity — empty {} ok; error / via=broker_miss hard miss
    assert!(assert_capacity_ok(&json!({})).is_ok());
    assert!(assert_capacity_ok(&json!({"ok": true})).is_ok());
    assert!(assert_capacity_ok(&json!({"snapshot": {"total_ram_mb": 1}})).is_ok());
    assert!(assert_capacity_ok(&json!({"error": "down"})).is_err());
    assert!(assert_capacity_ok(
        // sak485-i / sak493-h
        &json!({"via": "broker_miss", "status": "degraded", "feature": "health"}),
    )
    .is_err());
    assert!(assert_capacity_ok(
        // sak493-h
        &json!({"via": "broker_miss", "status": "degraded", "feature": "capacity"}),
    )
    .is_err());
}

#[test]
fn assert_list_ok_modules_empty_vs_miss() {
    // sak493-h: list_modules — [] ok; null/missing key / broker_miss hard miss
    assert!(assert_list_ok(&json!({"modules": []}), "modules").is_ok());
    assert!(assert_list_ok(&json!({"modules": null}), "modules").is_err());
    assert!(assert_list_ok(&json!({}), "modules").is_err());
    assert!(assert_list_ok(
        // sak484-i / sak493-h
        &json!({"via": "broker_miss", "status": "degraded", "feature": "list_modules"}),
        "modules",
    )
    .is_err());
    assert!(assert_list_ok(
        &json!({"via": "broker_miss", "modules": [], "status": "degraded"}),
        "modules",
    )
    .is_err());
}

#[test]
fn assert_record_ok_module_empty_vs_miss() {
    // sak493-h: get_module — record ok; null/missing / broker_miss hard miss
    assert!(assert_record_ok(&json!({"module": {"id": "m1"}}), "module").is_ok());
    assert!(assert_record_ok(&json!({"id": "m1"}), "module").is_ok());
    assert!(assert_record_ok(&json!({"module": null}), "module").is_err());
    assert!(assert_record_ok(&json!({}), "module").is_err());
    assert!(assert_record_ok(
        &json!({"via": "broker_miss", "status": "degraded", "feature": "get_module"}),
        "module",
    )
    .is_err());
}

#[test]
fn queue_depth_for_session_payload_first() {
    let items = vec![
        json!({"payload": {"session_id": "s1"}}),
        json!({"session_id": "s2"}),
        json!({"payload": {"session_id": "s2"}, "session_id": "s1"}),
    ];
    assert_eq!(queue_depth_for_session(&items, None), 3);
    assert_eq!(queue_depth_for_session(&items, Some("s1")), 1);
    assert_eq!(queue_depth_for_session(&items, Some("s2")), 2);
}

#[test]
fn node_id_from_broker_record_prefers_node_id() {
    assert_eq!(
        node_id_from_broker_record(&json!({"node_id": "n1", "id": "n2"})),
        "n1"
    );
    assert_eq!(node_id_from_broker_record(&json!({"id": "n2"})), "n2");
}

#[test]
fn is_compute_miss_detects_via_and_degraded() {
    assert!(is_compute_miss(&json!({"via": "broker_miss"})));
    assert!(is_compute_miss(&json!({"status": "degraded"})));
    assert!(is_compute_miss(&json!({"error": "down"})));
    assert!(!is_compute_miss(&json!({"work": {"id": "w1"}})));
}

#[test]
fn is_memory_miss_detects_memory_feature_and_broker_only() {
    assert!(is_memory_miss(&json!({"code": "broker_memory_only"})));
    assert!(is_memory_miss(
        &json!({"via": "broker_miss", "status": "degraded", "feature": "fleet_memory_search", "hits": []}),
    ));
    assert!(is_memory_miss(
        &json!({"feature": "fleet_memory_search", "error": "down", "hits": []}),
    ));
    assert!(!is_memory_miss(&json!({"hits": [], "via": "broker"})));
}

#[test]
fn domain_miss_detectors_sak496_i() {
    assert!(is_sandbox_miss(&json!({"code": "broker_sandbox_only"})));
    assert!(is_sandbox_miss(
        &json!({"via": "broker_miss", "feature": "sandbox_exec", "error": "down"}),
    ));
    assert!(!is_sandbox_miss(&json!({"stdout": "ok", "via": "broker"})));

    assert!(is_tools_miss(&json!({"code": "broker_tools_only"})));
    assert!(is_tools_miss(
        &json!({"via": "broker_miss", "feature": "shell", "error": "down"}),
    ));
    assert!(!is_tools_miss(&json!({"stdout": "ok", "via": "broker"})));

    assert!(is_research_miss(&json!({"code": "broker_research_only"})));
    assert!(is_research_miss(
        &json!({"via": "broker_miss", "feature": "research_fetch", "error": "down"}),
    ));

    assert!(is_egress_miss(&json!({"code": "broker_egress_only"})));
    assert!(is_egress_miss(
        &json!({"via": "broker_miss", "feature": "egress_audit", "error": "down"}),
    ));

    assert!(is_llm_miss(&json!({"code": "broker_llm_unavailable"})));
    assert!(is_llm_miss(
        &json!({"via": "broker_miss", "feature": "llm", "error": "down"}),
    ));
    assert!(!is_llm_miss(&json!({"content": "hi", "via": "broker"})));
}

#[test]
fn domain_assert_ok_sak496_i() {
    assert!(assert_sandbox_ok(&json!({"stdout": "ok"})).is_ok());
    assert!(assert_tools_ok(&json!({"stdout": "ok"})).is_ok());
    assert!(assert_research_ok(&json!({"body": "html"})).is_ok());
    assert!(assert_egress_ok(&json!({"allowed": true})).is_ok());
    assert!(assert_llm_ok(&json!({"content": "hi"})).is_ok());
    assert!(assert_sandbox_ok(&json!({"via": "broker_miss", "feature": "sandbox_exec"})).is_err());
    assert!(assert_llm_ok(&json!({"code": "broker_llm_unavailable"})).is_err());
}

#[test]
fn assert_memory_ok_empty_vs_miss() {
    assert!(assert_memory_ok(&json!({"hits": []}), "hits").is_ok());
    assert!(assert_memory_ok(&json!({"hits": [{"id": "m1"}], "via": "broker"}), "hits").is_ok());
    assert!(assert_memory_ok(&json!({"hits": null}), "hits").is_err());
    assert!(assert_memory_ok(&json!({}), "hits").is_err());
    assert!(assert_memory_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "fleet_memory_search", "hits": []}),
            "hits",
        )
        .is_err());
    assert!(assert_memory_ok(
        &json!({"error": "down", "feature": "fleet_memory_search", "hits": []}),
        "hits",
    )
    .is_err());
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn node_path_empty_vs_miss() {
    // sak491-j: list_nodes / list_nodes_filtered / register / heartbeat HTTP matrix
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"nodes": []})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"nodes": null})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "list",
            "nodes": []
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"nodes": [], "action": "list"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"nodes": null, "action": "list"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "list",
            "nodes": []
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"node": {"id": "n1", "label": "w1"}})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"node": null})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "register"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "node": {"id": "n1", "label": "w1"},
            "action": "heartbeat"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "heartbeat"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    assert_eq!(client.list_nodes().await.unwrap()["nodes"], json!([]));
    assert!(client.list_nodes().await.is_err());
    assert!(client.list_nodes().await.is_err());

    assert_eq!(
        client
            .list_nodes_filtered(json!({"session_id": "s1"}))
            .await
            .unwrap()["nodes"],
        json!([])
    );
    assert!(client
        .list_nodes_filtered(json!({"session_id": "s1"}))
        .await
        .is_err());
    assert!(client
        .list_nodes_filtered(json!({"session_id": "s1"}))
        .await
        .is_err());

    assert_eq!(
        client
            .register_node(json!({"action": "register", "label": "w1"}))
            .await
            .unwrap()["node"]["id"],
        "n1"
    );
    assert!(client
        .register_node(json!({"action": "register", "label": "w1"}))
        .await
        .is_err());
    assert!(client
        .register_node(json!({"action": "register", "label": "w1"}))
        .await
        .is_err());

    assert_eq!(
        client
            .heartbeat_node(json!({"action": "heartbeat", "node_id": "n1"}))
            .await
            .unwrap()["action"],
        "heartbeat"
    );
    assert!(client
        .heartbeat_node(json!({"action": "heartbeat", "node_id": "n1"}))
        .await
        .is_err());
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn work_write_empty_vs_miss() {
    // sak492-h: enqueue/complete/get/requeue/terminate_restart HTTP matrix; claim empty poll unchanged
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work": {"id": "w1", "status": "queued"},
            "action": "enqueue"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"work": null, "action": "enqueue"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "enqueue"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work": {"id": "w1", "status": "done"},
            "action": "complete"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"work": null, "action": "complete"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "complete"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work": {"id": "w1", "status": "queued"}
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"work": null})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "get"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work": {"id": "w1", "status": "queued"},
            "action": "requeue"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "requeue"
        })))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work": null,
            "error": "queue empty"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "work": null,
            "error": "queue empty"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());

    assert_eq!(
        client.enqueue_work("echo", json!({})).await.unwrap()["work"]["id"],
        "w1"
    );
    assert!(client.enqueue_work("echo", json!({})).await.is_err());
    assert!(client.enqueue_work("echo", json!({})).await.is_err());

    assert_eq!(
        client.complete_work("w1", "n1", json!({})).await.unwrap()["action"],
        "complete"
    );
    assert!(client.complete_work("w1", "n1", json!({})).await.is_err());
    assert!(client.complete_work("w1", "n1", json!({})).await.is_err());

    assert_eq!(client.get_work("w1").await.unwrap()["work"]["id"], "w1");
    assert!(client.get_work("w1").await.is_err());
    assert!(client.get_work("w1").await.is_err());

    assert_eq!(
        client.requeue_work("w1").await.unwrap()["action"],
        "requeue"
    );
    assert!(client.requeue_work("w1").await.is_err());
    assert!(client.terminate_restart_work("w1").await.is_err());

    let empty = client.claim_work("n1").await.unwrap();
    assert_eq!(empty["via"], "broker");
    assert!(empty["work"].is_null());
    assert!(client.claim_work("n1").await.is_err());
}

#[tokio::test]
async fn readiness_empty_vs_miss() {
    // sak493-h: health/capacity empty {} ok; list_modules [] ok; get_module record ok; null/missing/broker_miss hard miss
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "health"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"modules": []})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"modules": null})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "list_modules",
            "modules": []
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules/demo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"module": {"id": "demo"}})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules/demo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"module": null})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules/demo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "get_module"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/capacity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/capacity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "via": "broker_miss",
            "status": "degraded",
            "feature": "capacity"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    assert!(client.health().await.unwrap().is_object());
    assert!(client.health().await.is_err());

    assert_eq!(client.list_modules().await.unwrap()["modules"], json!([]));
    assert!(client.list_modules().await.is_err());
    assert!(client.list_modules().await.is_err());

    assert_eq!(
        client.get_module("demo").await.unwrap()["module"]["id"],
        "demo"
    );
    assert!(client.get_module("demo").await.is_err());
    assert!(client.get_module("demo").await.is_err());

    assert!(client.capacity().await.unwrap().is_object());
    assert!(client.capacity().await.is_err());
}

#[test]
fn broker_session_queue_miss_strips_feature_prefix() {
    let nodes = vec![json!({"node_id": "n1", "via": "broker"})];
    let exc = SdkError::Schema("broker_miss: list_work_filtered: work down".into());
    let out = broker_session_queue_miss(&exc, nodes.clone(), Some("s1"), Some("fleet_mesh"));
    assert_eq!(out["via"], "broker_miss");
    assert_eq!(out["status"], "degraded");
    assert_eq!(out["queue_depth"], 0);
    assert_eq!(out["nodes"], json!(nodes));
    assert_eq!(out["error"], "work down");
    assert_eq!(out["session_id"], "s1");
    assert_eq!(out["feature"], "fleet_mesh");
}

#[tokio::test]
async fn session_compute_status_nodes_ok_queue_fail_degraded() {
    let server = MockServer::start().await;
    let nid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "nodes": [{"id": nid, "label": "n1", "caps": []}],
            "action": "list"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "work down",
            "work": []
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    let out = client
        .session_compute_status(Some("s1"), Some("fleet_mesh"))
        .await
        .expect("degraded ok");
    assert_eq!(out["via"], "broker_miss");
    assert_eq!(out["status"], "degraded");
    assert_eq!(out["queue_depth"], 0);
    assert_eq!(out["nodes"].as_array().map(std::vec::Vec::len), Some(1));
    assert_eq!(out["nodes"][0]["node_id"], nid);
    assert!(out["error"].as_str().unwrap_or("").contains("work down"));
}

#[tokio::test]
async fn list_work_filtered_session_id_empty_vs_miss() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work": [],
            "action": "list"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "work down",
            "work": []
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    let empty = client
        .list_work_filtered(json!({"status": "queued", "session_id": "s1"}))
        .await
        .expect("empty ok");
    assert_eq!(empty["work"], json!([]));
    assert!(client
        .list_work_filtered(json!({"status": "queued", "session_id": "s1"}))
        .await
        .is_err());
}

#[tokio::test]
async fn health_and_modules() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/modules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"modules": []})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/capacity"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"snapshot": {"total_ram_mb": 1}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"work": []})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"work": [], "action": "list"})),
        )
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    assert_eq!(client.health().await.unwrap()["ok"], true);
    assert!(client.list_modules().await.unwrap()["modules"].is_array());
    assert!(client.capacity().await.unwrap()["snapshot"].is_object());
    assert!(client.list_work().await.unwrap()["work"].is_array());
    let listed = client
        .list_work_filtered(json!({"action": "list", "run_id": "r1"}))
        .await
        .unwrap();
    assert_eq!(listed["action"], "list");
}
