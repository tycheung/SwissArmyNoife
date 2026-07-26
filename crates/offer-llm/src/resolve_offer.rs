//! `llm.resolve` offer — read-only ADR 006 resolve (no secrets).

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::resolve::{resolve, ConnectionRef, ResolveHint};

/// First-party `llm.resolve` offer (metadata catalog only).
pub struct LlmResolveOffer {
    entry: CatalogEntry,
    connections: Vec<ConnectionRef>,
}

impl LlmResolveOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new(connections: Vec<ConnectionRef>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("llm.resolve", "0.1.0")?,
            connections,
        })
    }
}

impl Offer for LlmResolveOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-llm.resolve".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_resolve(&self.connections, &req.args) {
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
struct ResolveArgs {
    #[serde(default)]
    connection_id: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

fn run_resolve(catalog: &[ConnectionRef], args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: ResolveArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("resolve args: {e}")))?;
    let hint = ResolveHint {
        connection_id: parsed.connection_id,
        provider: parsed.provider,
        model: parsed.model,
    };
    let resolved = resolve(&hint, catalog).map_err(|e| (e.to_error_code(), e.to_string()))?;
    serde_json::to_value(&resolved)
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("resolve encode: {e}")))
        .map(|v| {
            json!({
                "resolved": v,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn resolve_offer_local_ollama_model() {
        let offer = LlmResolveOffer::new(vec![]).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "model": "tiny" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["resolved"]["provider"], "ollama");
                assert_eq!(result["resolved"]["model"], "tiny");
                assert_eq!(result["resolved"]["binding_source"], "local_ollama");
                assert!(result.get("api_key").is_none());
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
