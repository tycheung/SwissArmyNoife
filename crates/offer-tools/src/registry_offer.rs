//! `tools.registry` — list / get allowlisted tool specs.

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::allowlist::ToolAllowlist;
use crate::registry::{ToolRegistry, ToolSpec};

/// First-party `tools.registry` offer.
pub struct ToolsRegistryOffer {
    entry: CatalogEntry,
    registry: ToolRegistry,
    allowlist: Mutex<ToolAllowlist>,
}

impl ToolsRegistryOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new(registry: ToolRegistry) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("tools.registry", "0.1.0")?,
            registry,
            allowlist: Mutex::new(ToolAllowlist::unrestricted()),
        })
    }

    /// Seed with echo + ping specs (dev / live default pack).
    ///
    /// # Errors
    /// Propagates catalog or register errors.
    pub fn with_defaults() -> Result<Self, ErrorCode> {
        let mut reg = ToolRegistry::new();
        reg.register(ToolSpec::new(
            "tools.echo",
            "Echo a message",
            json!({
                "type": "object",
                "properties": { "message": { "type": "string" } },
                "required": ["message"]
            }),
        )?)?;
        reg.register(ToolSpec::new(
            "tools.ping",
            "Ping",
            json!({ "type": "object", "properties": {} }),
        )?)?;
        Self::new(reg)
    }
}

impl Offer for ToolsRegistryOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-tools.registry".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let mut g = self
            .allowlist
            .lock()
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        *g = ToolAllowlist::from_policy(&params);
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        let allow = self
            .allowlist
            .lock()
            .map_or_else(|_| ToolAllowlist::unrestricted(), |g| g.clone());
        match run_registry(&self.registry, &allow, &req.args) {
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
struct RegistryArgs {
    #[serde(default = "default_op")]
    op: String,
    #[serde(default)]
    id: Option<String>,
}

fn default_op() -> String {
    "list".into()
}

fn spec_json(spec: &ToolSpec) -> Value {
    json!({
        "id": spec.id,
        "description": spec.description,
        "input_schema": spec.input_schema,
    })
}

fn run_registry(
    registry: &ToolRegistry,
    allow: &ToolAllowlist,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: RegistryArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("registry args: {e}")))?;
    match parsed.op.as_str() {
        "list" => {
            let tools: Vec<_> = registry
                .list()
                .into_iter()
                .filter(|s| allow.permits(&s.id).is_ok())
                .map(spec_json)
                .collect();
            Ok(json!({
                "tools": tools,
                "restricted": allow.is_restricted(),
            }))
        }
        "get" => {
            let id = parsed
                .id
                .as_deref()
                .ok_or((ErrorCode::SchemaInvalid, "id required for get".into()))?;
            let spec = registry
                .get_allowed(allow, id)
                .map_err(|c| (c, format!("{c}: tool {id}")))?;
            Ok(spec_json(spec))
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("unknown op: {other} (expected list|get)"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn list_defaults() {
        let offer = ToolsRegistryOffer::with_defaults().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({ "op": "list" }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["tools"].as_array().expect("arr").len(), 2);
                assert_eq!(result["restricted"], false);
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn allowlist_filters_list_and_denies_get() {
        let offer = ToolsRegistryOffer::with_defaults().expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({ "tools": { "allow": ["tools.echo"] } }),
            )
            .await
            .expect("bind");
        let listed = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({ "op": "list" }),
                offer: None,
            })
            .await;
        match listed {
            InvokeResp::Ok { result, .. } => {
                let tools = result["tools"].as_array().expect("arr");
                assert_eq!(tools.len(), 1);
                assert_eq!(tools[0]["id"], "tools.echo");
                assert_eq!(result["restricted"], true);
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
        let deny = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({ "op": "get", "id": "tools.ping" }),
                offer: None,
            })
            .await;
        match deny {
            InvokeResp::Error {
                code: ErrorCode::PolicyDenied,
                ..
            } => {}
            other => panic!("expected policy.denied, got {other:?}"),
        }
    }
}
