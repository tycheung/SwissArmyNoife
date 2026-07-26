//! `eval.*` generic check-runner host (not Nimbusware standards UI).

mod runner;

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde_json::Value;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use runner::run_checks;

/// First-party `eval.run` offer — generic check runner.
pub struct EvalRunOffer {
    entry: CatalogEntry,
    /// `None` = all assert kinds allowed (frozen at bind).
    allowed_asserts: Mutex<Option<Vec<String>>>,
}

impl EvalRunOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("eval.run", "0.1.0")?,
            allowed_asserts: Mutex::new(None),
        })
    }
}

impl Offer for EvalRunOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-eval.run".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let allowed = parse_allowed_asserts(&params)?;
        let mut g = self
            .allowed_asserts
            .lock()
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        *g = allowed;
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        let allowed = self.allowed_asserts.lock().map_or(None, |g| g.clone());
        match run_checks(&req.args, allowed.as_deref()) {
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

fn parse_allowed_asserts(params: &Value) -> Result<Option<Vec<String>>, ErrorCode> {
    let Some(arr) = params
        .pointer("/eval/allowed_asserts")
        .or_else(|| params.get("allowed_asserts"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    if arr.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let s = v.as_str().ok_or(ErrorCode::SchemaInvalid)?;
        out.push(s.to_owned());
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use types::InvokeId;

    #[tokio::test]
    async fn catalog_is_eval_run() {
        let offer = EvalRunOffer::new().expect("offer");
        assert_eq!(offer.catalog_entry().id.as_str(), "eval.run");
        assert_eq!(offer.catalog_entry().version, "0.1.0");
    }

    #[tokio::test]
    async fn invoke_fixture_pass() {
        let offer = EvalRunOffer::new().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({
                    "checks": [
                        { "id": "n", "assert": "eq", "actual": 42, "expected": 42 }
                    ]
                }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => assert_eq!(result["passed"], true),
            InvokeResp::Error { message, .. } => panic!("unexpected error: {message}"),
        }
    }

    #[tokio::test]
    async fn invoke_fixture_fail() {
        let offer = EvalRunOffer::new().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({
                    "checks": [
                        { "id": "n", "assert": "eq", "actual": 1, "expected": 2 }
                    ]
                }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => assert_eq!(result["passed"], false),
            InvokeResp::Error { message, .. } => panic!("unexpected error: {message}"),
        }
    }

    #[tokio::test]
    async fn bind_deny_disallowed_assert() {
        let offer = EvalRunOffer::new().expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({ "eval": { "allowed_asserts": ["eq"] } }),
            )
            .await
            .expect("bind");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({
                    "checks": [
                        { "id": "c", "assert": "contains", "actual": "hi", "expected": "h" }
                    ]
                }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error { code, .. } => assert_eq!(code, ErrorCode::PolicyDenied),
            InvokeResp::Ok { .. } => panic!("expected policy deny"),
        }
    }
}
