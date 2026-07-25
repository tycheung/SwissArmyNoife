//! `sandbox.exec` offer: risk charge → backend exec → JSON result.

use std::path::PathBuf;
use std::sync::Mutex;

use control::{CatalogEntry, Offer, RiskLedger};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::{ExecRequest, SandboxBackend, SandboxError};

/// First-party `sandbox.exec` offer backed by a [`SandboxBackend`].
pub struct SandboxExecOffer<B> {
    entry: CatalogEntry,
    backend: B,
    risk: Mutex<RiskLedger>,
}

impl<B> SandboxExecOffer<B> {
    /// Build with an initial risk ledger (often from bind policy).
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the offer id is empty.
    pub fn new(backend: B, risk: RiskLedger) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("sandbox.exec", "0.1.0")?,
            backend,
            risk: Mutex::new(risk),
        })
    }

    /// Convenience: parse risk caps from a policy JSON object.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the offer id is empty.
    pub fn with_policy(backend: B, policy: &Value) -> Result<Self, ErrorCode> {
        Self::new(backend, RiskLedger::from_policy(policy))
    }
}

impl<B: SandboxBackend + Send + Sync> Offer for SandboxExecOffer<B> {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-sandbox.exec".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let mut risk = self.risk.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *risk = RiskLedger::from_policy(&params);
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_exec(&self.backend, &self.risk, &req.args) {
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
struct ExecArgs {
    argv: Vec<String>,
    #[serde(default = "default_cwd")]
    cwd: String,
}

fn default_cwd() -> String {
    ".".into()
}

fn run_exec<B: SandboxBackend>(
    backend: &B,
    risk: &Mutex<RiskLedger>,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: ExecArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("exec args: {e}")))?;
    if parsed.argv.is_empty() || parsed.argv[0].is_empty() {
        return Err((ErrorCode::SchemaInvalid, "argv must be non-empty".into()));
    }
    {
        let mut ledger = risk
            .lock()
            .map_err(|_| (ErrorCode::SchemaInvalid, "risk lock poisoned".into()))?;
        ledger.charge_shell().map_err(|code| {
            (
                code,
                "sandbox.violation:risk_cap: max_shell_invocations".into(),
            )
        })?;
    }
    let out = backend
        .exec(&ExecRequest {
            argv: parsed.argv,
            cwd: PathBuf::from(parsed.cwd),
        })
        .map_err(|e| map_sandbox(&e))?;
    Ok(json!({
        "exit_code": out.exit_code,
        "stdout": out.stdout,
        "stderr": out.stderr,
    }))
}

fn map_sandbox(err: &SandboxError) -> (ErrorCode, String) {
    (err.to_error_code(), err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StubBackend;
    use serde_json::json;
    use types::InvokeId;

    fn stub_offer(policy: &Value) -> SandboxExecOffer<StubBackend> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backend = StubBackend::with_root(tmp.path()).expect("backend");
        let _root = tmp.keep();
        SandboxExecOffer::with_policy(backend, policy).expect("offer")
    }

    #[tokio::test]
    async fn invoke_stub_returns_stdout() {
        let offer = stub_offer(&json!({}));
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["echo", "hi"]}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["exit_code"], 0);
                assert_eq!(result["stdout"], "stub:echo\u{1f}hi");
            }
            InvokeResp::Error { code, message, .. } => {
                panic!("unexpected error {code}: {message}")
            }
        }
    }

    #[tokio::test]
    async fn shell_cap_exhausts() {
        let offer = stub_offer(&json!({
            "risk_caps": { "max_shell_invocations": 1 }
        }));
        let req = InvokeReq {
            binding_id: BindingId::new(),
            args: json!({"argv": ["true"]}),
            invoke_id: None,
            offer: None,
        };
        match offer.invoke(req.clone()).await {
            InvokeResp::Ok { .. } => {}
            InvokeResp::Error { code, message, .. } => {
                panic!("first should ok: {code}: {message}")
            }
        }
        match offer.invoke(req).await {
            InvokeResp::Error {
                code: ErrorCode::BudgetExhausted,
                ..
            } => {}
            other => panic!("expected BudgetExhausted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cwd_escape_is_sandbox_violation() {
        let offer = stub_offer(&json!({}));
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["true"], "cwd": ".."}),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error {
                code: ErrorCode::SandboxViolation,
                message,
                ..
            } => {
                assert!(
                    message.contains("path_escape"),
                    "expected path_escape subcode, got {message}"
                );
            }
            other => panic!("expected SandboxViolation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn bind_resets_risk_from_policy() {
        let offer = stub_offer(&json!({
            "risk_caps": { "max_shell_invocations": 1 }
        }));
        let _ = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["true"]}),
                invoke_id: None,
                offer: None,
            })
            .await;
        offer
            .bind(
                BindingId::new(),
                json!({"risk_caps": {"max_shell_invocations": 2}}),
            )
            .await
            .expect("bind");
        match offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["true"]}),
                invoke_id: None,
                offer: None,
            })
            .await
        {
            InvokeResp::Ok { .. } => {}
            InvokeResp::Error { code, message, .. } => {
                panic!("after rebind should ok: {code}: {message}")
            }
        }
    }
}
