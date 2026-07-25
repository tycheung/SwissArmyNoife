//! `network.egress.check` offer: principal + host allowlist gate (no HTTP fetch).

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::policy::EgressPolicy;

/// First-party egress check offer.
pub struct EgressCheckOffer {
    entry: CatalogEntry,
    policy: Mutex<EgressPolicy>,
    /// Principal frozen at bind time (from binding store is preferred; fallback arg).
    principal: Mutex<String>,
}

impl EgressCheckOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when catalog id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("network.egress.check", "0.1.0")?,
            policy: Mutex::new(EgressPolicy::default()),
            principal: Mutex::new("local".into()),
        })
    }
}

impl Offer for EgressCheckOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-network.egress.check".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let mut policy = self.policy.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *policy = EgressPolicy::from_policy(&params);
        if let Some(p) = params.get("principal").and_then(Value::as_str) {
            let mut principal = self
                .principal
                .lock()
                .map_err(|_| ErrorCode::SchemaInvalid)?;
            p.clone_into(&mut *principal);
        }
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_check(&self.policy, &self.principal, &req.args) {
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
struct CheckArgs {
    url: String,
    #[serde(default)]
    principal: Option<String>,
}

fn run_check(
    policy: &Mutex<EgressPolicy>,
    frozen_principal: &Mutex<String>,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: CheckArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("check args: {e}")))?;
    let policy = policy
        .lock()
        .map_err(|_| (ErrorCode::SchemaInvalid, "policy lock".into()))?;
    let default_principal = frozen_principal
        .lock()
        .map_err(|_| (ErrorCode::SchemaInvalid, "principal lock".into()))?;
    let principal = parsed
        .principal
        .as_deref()
        .unwrap_or(default_principal.as_str());
    let host = policy
        .check(principal, &parsed.url)
        .map_err(|code| (code, format!("{code}: egress check failed")))?;
    Ok(json!({
        "allowed": true,
        "host": host,
        "url": parsed.url,
        "principal": principal,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn check_allow_and_deny() {
        let offer = EgressCheckOffer::new().expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({
                    "egress": {
                        "allow_hosts": ["api.example.com"],
                        "allow_principals": ["local"]
                    }
                }),
            )
            .await
            .expect("bind");

        let ok = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "url": "https://api.example.com/v1" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match ok {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["allowed"], true);
                assert_eq!(result["host"], "api.example.com");
            }
            other @ InvokeResp::Error { .. } => panic!("expected ok, got {other:?}"),
        }

        let denied = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "url": "https://evil.com/" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match denied {
            InvokeResp::Error {
                code: ErrorCode::EgressDenied,
                ..
            } => {}
            other => panic!("expected egress.denied, got {other:?}"),
        }
    }
}
