//! `compute.node` offer — register / heartbeat / list (`sak290`).

use std::sync::Arc;
use std::time::Duration;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};
use uuid::Uuid;

use crate::node::NodeId;
use crate::plane::ComputePlane;

/// Node registry surface.
pub struct ComputeNodeOffer {
    entry: CatalogEntry,
    plane: Arc<ComputePlane>,
}

impl ComputeNodeOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(plane: Arc<ComputePlane>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("compute.node", "0.1.0")?,
            plane,
        })
    }
}

impl Offer for ComputeNodeOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-compute.node".into())
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
struct NodeArgs {
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

fn run(plane: &ComputePlane, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: NodeArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("node args: {e}")))?;
    match parsed.action.as_str() {
        "register" => {
            let label = parsed
                .label
                .filter(|s| !s.is_empty())
                .ok_or((ErrorCode::SchemaInvalid, "label required".into()))?;
            let id = parse_node_id(parsed.node_id.as_deref())?;
            let rec = plane
                .nodes
                .register_scoped(
                    &label,
                    parsed.caps.unwrap_or_default(),
                    id,
                    parsed.session_id.filter(|s| !s.is_empty()),
                )
                .map_err(|c| (c, format!("{c}: register")))?;
            serde_json::to_value(rec).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        "heartbeat" => {
            let id = parse_node_id(parsed.node_id.as_deref())?
                .ok_or((ErrorCode::SchemaInvalid, "node_id required".into()))?;
            let rec = plane
                .nodes
                .heartbeat(id)
                .map_err(|c| (c, format!("{c}: heartbeat")))?;
            serde_json::to_value(rec).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))
        }
        "list" => {
            let stale = parsed.stale_secs.map(Duration::from_secs);
            let session = parsed.session_id.filter(|s| !s.is_empty());
            let list = plane
                .nodes
                .list_filtered(stale, session.as_deref())
                .map_err(|c| (c, format!("{c}: list")))?;
            Ok(json!({ "nodes": list }))
        }
        other => Err((ErrorCode::SchemaInvalid, format!("unknown action: {other}"))),
    }
}

fn parse_node_id(raw: Option<&str>) -> Result<Option<NodeId>, (ErrorCode, String)> {
    match raw {
        None | Some("") => Ok(None),
        Some(s) => {
            let u =
                Uuid::parse_str(s).map_err(|_| (ErrorCode::SchemaInvalid, "bad node_id".into()))?;
            Ok(Some(NodeId::from_uuid(u)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn register_and_list() {
        let offer = ComputeNodeOffer::new(Arc::new(ComputePlane::new())).unwrap();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"action": "register", "label": "w1", "caps": ["echo"]}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => assert_eq!(result["label"], "w1"),
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
