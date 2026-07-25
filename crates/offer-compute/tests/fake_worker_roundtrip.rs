//! Conformance: fake worker register → enqueue → claim → complete (`sak299-a`).

use std::sync::Arc;

use offer_compute::{ComputePlane, IdentityMerge, WorkStatus};
use serde_json::json;

#[test]
fn fake_worker_round_trip() {
    let plane = Arc::new(ComputePlane::with_merge(Arc::new(IdentityMerge)));
    let node = plane
        .nodes
        .register("fake", vec!["echo".into()], None)
        .expect("register");
    let unit = plane
        .queue
        .enqueue("echo", json!({ "n": 7, "password": "nope" }))
        .expect("enqueue");
    assert_eq!(unit.payload["password"], "[REDACTED]");

    let claimed = plane.queue.claim(node.id).expect("claim");
    assert_eq!(claimed.id, unit.id);
    assert_eq!(claimed.status, WorkStatus::Claimed);

    let done = plane
        .queue
        .complete(
            claimed.id,
            node.id,
            json!({ "sum": 7 }),
            plane.merge.as_ref(),
        )
        .expect("complete");
    assert_eq!(done.status, WorkStatus::Completed);
    assert_eq!(done.result.unwrap()["sum"], 7);

    assert!(plane.queue.claim(node.id).is_err());
}

#[test]
fn sqlite_queue_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("broker.db");
    let plane = ComputePlane::with_sqlite_queue(&path).expect("sqlite plane");
    let node = plane.nodes.register("sql", vec![], None).unwrap();
    let u = plane.queue.enqueue("echo", json!({"x": 1})).unwrap();
    let c = plane.queue.claim(node.id).unwrap();
    assert_eq!(c.id, u.id);
    let d = plane
        .queue
        .complete(c.id, node.id, json!({"y": 2}), plane.merge.as_ref())
        .unwrap();
    assert_eq!(d.status, WorkStatus::Completed);
}
