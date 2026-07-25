//! `compute.work` offer — enqueue / claim / complete / get (`sak291`–`sak292`).

use std::sync::Arc;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};
use uuid::Uuid;

use crate::node::NodeId;
use crate::plane::ComputePlane;
use crate::queue::WorkId;

/// Work queue surface.
pub struct ComputeWorkOffer {
    entry: CatalogEntry,
    plane: Arc<ComputePlane>,
}

impl ComputeWorkOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(plane: Arc<ComputePlane>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("compute.work", "0.1.0")?,
            plane,
        })
    }
}

impl Offer for ComputeWorkOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-compute.work".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run(&self.plane, &req.args) {
            Ok(v) => InvokeResp::ok(invoke_id, v),
            Err((code, message)) => InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code,
                message,
            },
        }
    }

    async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct WorkArgs {
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

fn payload_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str()).or_else(|| {
        payload
            .get("payload")
            .and_then(|inner| inner.get(key))
            .and_then(|v| v.as_str())
    })
}

fn work_matches(
    unit: &crate::queue::WorkUnit,
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

fn run(plane: &ComputePlane, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: WorkArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("work args: {e}")))?;
    match parsed.action.as_str() {
        "enqueue" => {
            let kind = parsed
                .kind
                .filter(|s| !s.is_empty())
                .ok_or((ErrorCode::SchemaInvalid, "kind required".into()))?;
            let payload = parsed.payload.unwrap_or_else(|| json!({}));
            let unit = plane
                .queue
                .enqueue(&kind, payload)
                .map_err(|c| (c, format!("{c}: enqueue")))?;
            serde_json::to_value(unit).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        "claim" => {
            let node = require_node(parsed.node_id.as_deref())?;
            let unit = plane
                .queue
                .claim(node)
                .map_err(|c| (c, format!("{c}: claim")))?;
            serde_json::to_value(unit).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        "complete" => {
            let node = require_node(parsed.node_id.as_deref())?;
            let work_id = require_work(parsed.work_id.as_deref())?;
            let result = parsed
                .result
                .ok_or((ErrorCode::SchemaInvalid, "result required".into()))?;
            let unit = plane
                .queue
                .complete(work_id, node, result, plane.merge.as_ref())
                .map_err(|c| (c, format!("{c}: complete")))?;
            serde_json::to_value(unit).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        "get" => {
            let work_id = require_work(parsed.work_id.as_deref())?;
            let unit = plane
                .queue
                .get(work_id)
                .map_err(|c| (c, format!("{c}: get")))?;
            serde_json::to_value(unit).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        "list" => {
            let limit = parsed.limit.unwrap_or(100).clamp(1, 500);
            let units = plane
                .queue
                .list(limit)
                .map_err(|c| (c, format!("{c}: list")))?;
            let filtered: Vec<_> = units
                .into_iter()
                .filter(|u| {
                    work_matches(
                        u,
                        parsed.run_id.as_deref(),
                        parsed.stage_name.as_deref(),
                        parsed.status.as_deref(),
                    )
                })
                .collect();
            Ok(json!({ "work": filtered }))
        }
        "requeue" => {
            let work_id = require_work(parsed.work_id.as_deref())?;
            let unit = plane
                .queue
                .requeue(work_id)
                .map_err(|c| (c, format!("{c}: requeue")))?;
            serde_json::to_value(unit).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        other => Err((ErrorCode::SchemaInvalid, format!("unknown action: {other}"))),
    }
}

fn require_node(raw: Option<&str>) -> Result<NodeId, (ErrorCode, String)> {
    let s = raw
        .filter(|s| !s.is_empty())
        .ok_or((ErrorCode::SchemaInvalid, "node_id required".into()))?;
    let u = Uuid::parse_str(s).map_err(|_| (ErrorCode::SchemaInvalid, "bad node_id".into()))?;
    Ok(NodeId::from_uuid(u))
}

fn require_work(raw: Option<&str>) -> Result<WorkId, (ErrorCode, String)> {
    let s = raw
        .filter(|s| !s.is_empty())
        .ok_or((ErrorCode::SchemaInvalid, "work_id required".into()))?;
    let u = Uuid::parse_str(s).map_err(|_| (ErrorCode::SchemaInvalid, "bad work_id".into()))?;
    Ok(WorkId::from_uuid(u))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_offer::ComputeNodeOffer;
    use types::InvokeId;

    #[tokio::test]
    async fn enqueue_claim_complete_roundtrip() {
        let plane = Arc::new(ComputePlane::new());
        let nodes = ComputeNodeOffer::new(Arc::clone(&plane)).unwrap();
        let work = ComputeWorkOffer::new(Arc::clone(&plane)).unwrap();

        let InvokeResp::Ok { result: node, .. } = nodes
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"action": "register", "label": "w"}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await
        else {
            panic!("register");
        };
        let node_id = node["id"].as_str().unwrap().to_owned();

        let InvokeResp::Ok { result: unit, .. } = work
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "action": "enqueue",
                    "kind": "echo",
                    "payload": { "n": 1, "api_key": "secret" }
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await
        else {
            panic!("enqueue");
        };
        assert_eq!(unit["payload"]["api_key"], "[REDACTED]");
        let work_id = unit["id"].as_str().unwrap().to_owned();

        let InvokeResp::Ok {
            result: claimed, ..
        } = work
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"action": "claim", "node_id": node_id}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await
        else {
            panic!("claim");
        };
        assert_eq!(claimed["id"], work_id);

        let InvokeResp::Ok { result: done, .. } = work
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "action": "complete",
                    "node_id": node_id,
                    "work_id": work_id,
                    "result": { "echo": true }
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await
        else {
            panic!("complete");
        };
        assert_eq!(done["status"], "completed");
    }
}
