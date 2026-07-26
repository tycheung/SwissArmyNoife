//! `memory.scope` — hash / list scope kinds (repo / user / org).

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::scope::{scope_hash, ScopeKind};

/// First-party `memory.scope` offer.
pub struct MemoryScopeOffer {
    entry: CatalogEntry,
}

impl MemoryScopeOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("memory.scope", "0.1.0")?,
        })
    }
}

impl Offer for MemoryScopeOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-memory.scope".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_scope(&req.args) {
            Ok(result) => InvokeResp::ok(invoke_id, result),
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
struct ScopeArgs {
    #[serde(default = "default_op")]
    op: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

fn default_op() -> String {
    "hash".into()
}

fn parse_kind(raw: &str) -> Result<ScopeKind, (ErrorCode, String)> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "repo" => Ok(ScopeKind::Repo),
        "user" => Ok(ScopeKind::User),
        "org" => Ok(ScopeKind::Org),
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("unknown scope kind: {other}"),
        )),
    }
}

fn run_scope(args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: ScopeArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("scope args: {e}")))?;
    match parsed.op.as_str() {
        "kinds" => Ok(json!({ "kinds": ["repo", "user", "org"] })),
        "hash" | "inspect" => {
            let kind_raw = parsed
                .kind
                .as_deref()
                .ok_or((ErrorCode::SchemaInvalid, "kind required".into()))?;
            let id = parsed
                .id
                .as_deref()
                .ok_or((ErrorCode::SchemaInvalid, "id required".into()))?;
            if id.trim().is_empty() {
                return Err((ErrorCode::SchemaInvalid, "id must be non-empty".into()));
            }
            let kind = parse_kind(kind_raw)?;
            let id_norm = id.trim().to_ascii_lowercase();
            let scope_key = scope_hash(kind, id);
            Ok(json!({
                "scope_key": scope_key,
                "kind": kind.as_str(),
                "id": id_norm,
            }))
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("unknown op: {other} (expected hash|inspect|kinds)"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn hash_repo_stable() {
        let offer = MemoryScopeOffer::new().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({ "op": "hash", "kind": "repo", "id": "Acme/App" }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["kind"], "repo");
                assert_eq!(result["id"], "acme/app");
                assert_eq!(result["scope_key"], scope_hash(ScopeKind::Repo, "Acme/App"));
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn kinds_lists_three() {
        let offer = MemoryScopeOffer::new().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({ "op": "kinds" }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["kinds"].as_array().expect("arr").len(), 3);
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn bad_kind_errors() {
        let offer = MemoryScopeOffer::new().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({ "kind": "galaxy", "id": "x" }),
                offer: None,
            })
            .await;
        assert!(matches!(resp, InvokeResp::Error { .. }));
    }
}
