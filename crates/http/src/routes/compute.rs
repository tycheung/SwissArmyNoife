//! Compute work + nodes (`sak290-d` … `sak429-d` `AppState` `ComputePlane`).

use std::time::Duration;

use axum::extract::State;
use axum::{routing::get, Json, Router};
use offer_compute::{NodeId, WorkId};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::state::AppState;

async fn list_work(State(state): State<AppState>) -> Json<Value> {
    match state.compute() {
        Ok(plane) => match plane.queue.list(50) {
            Ok(units) => Json(json!({
                "work": units,
                "backend": "sqlite",
                "via": "app_state_compute_plane"
            })),
            Err(code) => Json(json!({ "error": code.as_str(), "work": [] })),
        },
        Err(message) => Json(json!({
            "error": message,
            "work": [],
            "hint": "open broker.db failed; set CONFIG_DIR/DB_PATH"
        })),
    }
}

#[derive(Debug, Deserialize)]
struct WorkActionBody {
    action: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    payload: Option<Value>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    work_id: Option<String>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    stage_name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

fn parse_node(raw: Option<&str>) -> Result<NodeId, String> {
    let s = raw
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "node_id required".to_owned())?;
    let u = Uuid::parse_str(s).map_err(|_| "bad node_id".to_owned())?;
    Ok(NodeId::from_uuid(u))
}

fn parse_work(raw: Option<&str>) -> Result<WorkId, String> {
    let s = raw
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "work_id required".to_owned())?;
    let u = Uuid::parse_str(s).map_err(|_| "bad work_id".to_owned())?;
    Ok(WorkId::from_uuid(u))
}

fn payload_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str()).or_else(|| {
        payload
            .get("payload")
            .and_then(|inner| inner.get(key))
            .and_then(|v| v.as_str())
    })
}

fn work_matches(
    unit: &offer_compute::WorkUnit,
    run_id: Option<&str>,
    stage_name: Option<&str>,
    status: Option<&str>,
) -> bool {
    if let Some(want) = status.filter(|s| !s.is_empty()) {
        if unit.status.as_str() != want {
            return false;
        }
    }
    if let Some(stage) = stage_name.filter(|s| !s.is_empty()) {
        let kind_ok = unit.kind == stage;
        let payload_ok = payload_str(&unit.payload, "stage_name") == Some(stage);
        if !kind_ok && !payload_ok {
            return false;
        }
    }
    if let Some(rid) = run_id.filter(|s| !s.is_empty()) {
        if payload_str(&unit.payload, "run_id") != Some(rid) {
            return false;
        }
    }
    true
}

#[allow(clippy::too_many_lines)]
async fn work_action(
    State(state): State<AppState>,
    Json(body): Json<WorkActionBody>,
) -> Json<Value> {
    let plane = match state.compute() {
        Ok(p) => p,
        Err(message) => {
            return Json(json!({
                "error": message,
                "hint": "open broker.db failed; set CONFIG_DIR/DB_PATH"
            }));
        }
    };
    let q = plane.queue.as_ref();
    let out = match body.action.as_str() {
        "enqueue" => {
            let kind = body
                .kind
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "kind required".to_owned());
            match kind {
                Ok(kind) => {
                    let payload = body.payload.unwrap_or_else(|| json!({}));
                    q.enqueue(&kind, payload)
                        .map(|u| {
                            json!({
                                "work": u,
                                "action": "enqueue",
                                "backend": "sqlite",
                                "via": "app_state_compute_plane"
                            })
                        })
                        .map_err(|c| c.as_str().to_owned())
                }
                Err(e) => Err(e),
            }
        }
        "claim" => match parse_node(body.node_id.as_deref()) {
            Ok(node) => q
                .claim(node)
                .map(|u| {
                    json!({
                        "work": u,
                        "action": "claim",
                        "backend": "sqlite",
                        "via": "app_state_compute_plane"
                    })
                })
                .map_err(|c| c.as_str().to_owned()),
            Err(e) => Err(e),
        },
        "complete" => {
            let node = parse_node(body.node_id.as_deref());
            let work_id = parse_work(body.work_id.as_deref());
            let result = body.result.ok_or_else(|| "result required".to_owned());
            match (node, work_id, result) {
                (Ok(node), Ok(work_id), Ok(result)) => q
                    .complete(work_id, node, result, plane.merge.as_ref())
                    .map(|u| {
                        json!({
                            "work": u,
                            "action": "complete",
                            "backend": "sqlite",
                            "via": "app_state_compute_plane"
                        })
                    })
                    .map_err(|c| c.as_str().to_owned()),
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
            }
        }
        "get" => match parse_work(body.work_id.as_deref()) {
            Ok(work_id) => q
                .get(work_id)
                .map(|u| {
                    json!({
                        "work": u,
                        "action": "get",
                        "backend": "sqlite",
                        "via": "app_state_compute_plane"
                    })
                })
                .map_err(|c| c.as_str().to_owned()),
            Err(e) => Err(e),
        },
        "requeue" => match parse_work(body.work_id.as_deref()) {
            Ok(work_id) => q
                .requeue(work_id)
                .map(|u| {
                    json!({
                        "work": u,
                        "action": "requeue",
                        "backend": "sqlite",
                        "via": "app_state_compute_plane"
                    })
                })
                .map_err(|c| c.as_str().to_owned()),
            Err(e) => Err(e),
        },
        "list" => {
            let limit = body.limit.unwrap_or(100).clamp(1, 500);
            match q.list(limit) {
                Ok(units) => {
                    let filtered: Vec<_> = units
                        .into_iter()
                        .filter(|u| {
                            work_matches(
                                u,
                                body.run_id.as_deref(),
                                body.stage_name.as_deref(),
                                body.status.as_deref(),
                            )
                        })
                        .collect();
                    Ok(json!({
                        "work": filtered,
                        "action": "list",
                        "backend": "sqlite",
                        "via": "app_state_compute_plane"
                    }))
                }
                Err(c) => Err(c.as_str().to_owned()),
            }
        }
        other => Err(format!("unknown action: {other}")),
    };
    match out {
        Ok(v) => Json(v),
        Err(message) => Json(json!({ "error": message, "action": body.action })),
    }
}

#[derive(Debug, Deserialize)]
struct NodeActionBody {
    action: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    caps: Option<Vec<String>>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    stale_secs: Option<u64>,
    #[serde(default)]
    session_id: Option<String>,
}

fn parse_optional_node(raw: Option<&str>) -> Result<Option<NodeId>, String> {
    match raw {
        None | Some("") => Ok(None),
        Some(s) => {
            let u = Uuid::parse_str(s).map_err(|_| "bad node_id".to_owned())?;
            Ok(Some(NodeId::from_uuid(u)))
        }
    }
}

async fn list_nodes(State(state): State<AppState>) -> Json<Value> {
    match state.compute() {
        Ok(plane) => match plane.nodes.list(None) {
            Ok(nodes) => Json(json!({
                "nodes": nodes,
                "backend": "sqlite",
                "via": "app_state_compute_plane",
                "note": "AppState ComputePlane SQLite (sak429-d; default COMPUTE_QUEUE=sqlite)"
            })),
            Err(code) => Json(json!({ "error": code.as_str(), "nodes": [] })),
        },
        Err(message) => Json(json!({
            "error": message,
            "nodes": [],
            "hint": "open broker.db failed; set CONFIG_DIR/DB_PATH"
        })),
    }
}

async fn node_action(
    State(state): State<AppState>,
    Json(body): Json<NodeActionBody>,
) -> Json<Value> {
    let plane = match state.compute() {
        Ok(p) => p,
        Err(message) => {
            return Json(json!({
                "error": message,
                "hint": "open broker.db failed; set CONFIG_DIR/DB_PATH"
            }));
        }
    };
    let reg = plane.nodes.as_ref();
    let out = match body.action.as_str() {
        "register" => {
            let label = body
                .label
                .clone()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "label required".to_owned());
            match (label, parse_optional_node(body.node_id.as_deref())) {
                (Ok(label), Ok(id)) => reg
                    .register_scoped(
                        &label,
                        body.caps.clone().unwrap_or_default(),
                        id,
                        body.session_id.clone().filter(|s| !s.is_empty()),
                    )
                    .map(|n| {
                        json!({
                            "node": n,
                            "action": "register",
                            "backend": "sqlite",
                            "via": "app_state_compute_plane"
                        })
                    })
                    .map_err(|c| c.as_str().to_owned()),
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }
        "heartbeat" => match parse_node(body.node_id.as_deref()) {
            Ok(id) => reg
                .heartbeat(id)
                .map(|n| {
                    json!({
                        "node": n,
                        "action": "heartbeat",
                        "backend": "sqlite",
                        "via": "app_state_compute_plane"
                    })
                })
                .map_err(|c| c.as_str().to_owned()),
            Err(e) => Err(e),
        },
        "list" => {
            let stale_after = body.stale_secs.map(Duration::from_secs);
            let session = body.session_id.clone().filter(|s| !s.is_empty());
            reg.list_filtered(stale_after, session.as_deref())
                .map(|nodes| {
                    json!({
                        "nodes": nodes,
                        "action": "list",
                        "backend": "sqlite",
                        "via": "app_state_compute_plane"
                    })
                })
                .map_err(|c| c.as_str().to_owned())
        }
        other => Err(format!("unknown action: {other}")),
    };
    match out {
        Ok(v) => Json(v),
        Err(message) => Json(json!({ "error": message, "action": body.action })),
    }
}

pub fn compute_router() -> Router<AppState> {
    Router::new()
        .route("/v1/sak/compute/work", get(list_work).post(work_action))
        .route("/v1/sak/compute/nodes", get(list_nodes).post(node_action))
}
