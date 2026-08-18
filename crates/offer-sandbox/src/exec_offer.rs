//! `sandbox.exec` offer: risk charge → backend exec → JSON result.

use std::path::PathBuf;
use std::sync::Mutex;

use control::{CatalogEntry, Offer, RiskLedger};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::program_policy::is_shell_wrapper;
use crate::{ExecRequest, ProgramAllowlist, SandboxBackend, SandboxError, WorkspaceMountPolicy};

/// First-party `sandbox.exec` offer backed by a [`SandboxBackend`].
pub struct SandboxExecOffer<B> {
    entry: CatalogEntry,
    backend: B,
    risk: Mutex<RiskLedger>,
    mounts: Mutex<WorkspaceMountPolicy>,
    programs: Mutex<ProgramAllowlist>,
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
            mounts: Mutex::new(WorkspaceMountPolicy::default()),
            programs: Mutex::new(ProgramAllowlist::unrestricted()),
        })
    }

    /// Convenience: parse risk caps and program allowlist from a policy JSON object.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the offer id is empty.
    pub fn with_policy(backend: B, policy: &Value) -> Result<Self, ErrorCode> {
        let offer = Self::new(backend, RiskLedger::from_policy(policy))?;
        *offer
            .programs
            .lock()
            .map_err(|_| ErrorCode::SchemaInvalid)? = ProgramAllowlist::from_policy(policy);
        Ok(offer)
    }

    /// Last bind-time mount policy (empty until bind).
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] if the mounts lock is poisoned.
    pub fn mount_policy(&self) -> Result<WorkspaceMountPolicy, ErrorCode> {
        self.mounts
            .lock()
            .map(|g| g.clone())
            .map_err(|_| ErrorCode::SchemaInvalid)
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
        let parsed =
            WorkspaceMountPolicy::from_bind_params(&params).map_err(|e| e.to_error_code())?;
        let mut risk = self.risk.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *risk = RiskLedger::from_policy(&params);
        drop(risk);
        let mut mounts = self.mounts.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *mounts = parsed;
        drop(mounts);
        let mut programs = self.programs.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *programs = ProgramAllowlist::from_policy(&params);
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_exec(
            &self.backend,
            &self.risk,
            &self.mounts,
            &self.programs,
            &req.args,
        ) {
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
    mounts: &Mutex<WorkspaceMountPolicy>,
    programs: &Mutex<ProgramAllowlist>,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: ExecArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("exec args: {e}")))?;
    if parsed.argv.is_empty() || parsed.argv[0].is_empty() {
        return Err((ErrorCode::SchemaInvalid, "argv must be non-empty".into()));
    }
    {
        let allow = programs
            .lock()
            .map_err(|_| (ErrorCode::SchemaInvalid, "programs lock poisoned".into()))?;
        allow.check_exec(&parsed.argv).map_err(|code| {
            let kind = if is_shell_wrapper(&parsed.argv) {
                "sandbox.shell"
            } else {
                "sandbox.programs"
            };
            (code, format!("policy.denied: {kind}: {}", parsed.argv[0]))
        })?;
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
    let policy = mounts
        .lock()
        .map_err(|_| (ErrorCode::SchemaInvalid, "mounts lock poisoned".into()))?
        .clone();
    let out = backend
        .exec_with_mounts(
            &ExecRequest {
                argv: parsed.argv,
                cwd: PathBuf::from(parsed.cwd),
            },
            &policy,
        )
        .map_err(|e| map_sandbox(&e))?;
    let caps = {
        let ledger = risk
            .lock()
            .map_err(|_| (ErrorCode::SchemaInvalid, "risk lock poisoned".into()))?;
        ledger.caps().clone()
    };
    let (stdout, stdout_truncated) =
        truncate_capture(&out.stdout, capture_cap(caps.max_stdout_bytes));
    let (stderr, stderr_truncated) =
        truncate_capture(&out.stderr, capture_cap(caps.max_stderr_bytes));
    Ok(json!({
        "exit_code": out.exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "stdout_truncated": stdout_truncated,
        "stderr_truncated": stderr_truncated,
    }))
}

const DEFAULT_CAPTURE_BYTES: usize = 1_048_576;

fn capture_cap(policy: Option<u64>) -> usize {
    policy
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(DEFAULT_CAPTURE_BYTES)
}

fn truncate_capture(raw: &str, cap: usize) -> (String, bool) {
    let bytes = raw.as_bytes();
    if bytes.len() <= cap {
        return (raw.to_string(), false);
    }
    let mut end = cap;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    (raw[..end].to_string(), true)
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

    #[tokio::test]
    async fn bind_parses_mounts_policy() {
        let offer = stub_offer(&json!({}));
        offer
            .bind(
                BindingId::new(),
                json!({
                    "mounts": [{
                        "host": "/data/project",
                        "guest": "workspace/src",
                        "read_only": true
                    }]
                }),
            )
            .await
            .expect("bind");
        let policy = offer.mount_policy().expect("policy");
        assert_eq!(policy.mounts.len(), 1);
        assert!(policy.mounts[0].read_only);
    }

    #[tokio::test]
    async fn bind_rejects_escaping_guest() {
        let offer = stub_offer(&json!({}));
        let err = offer
            .bind(
                BindingId::new(),
                json!({
                    "mounts": [{ "host": "/data", "guest": "../escape" }]
                }),
            )
            .await
            .expect_err("escape");
        assert_eq!(err, ErrorCode::SandboxViolation);
    }

    #[tokio::test]
    async fn unknown_program_is_policy_denied() {
        let offer = stub_offer(&json!({
            "sandbox": { "programs": ["git", "cargo"] }
        }));
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["python"]}),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error {
                code: ErrorCode::PolicyDenied,
                message,
                ..
            } => {
                assert!(
                    message.contains("sandbox.programs"),
                    "expected programs deny, got {message}"
                );
            }
            other => panic!("expected PolicyDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn git_fixture_allowed() {
        let offer = stub_offer(&json!({
            "sandbox": { "programs": ["git", "cargo", "echo"] }
        }));
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["echo", "ok"]}),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { .. } => {}
            InvokeResp::Error { code, message, .. } => {
                panic!("git/cargo fixture should allow echo: {code}: {message}")
            }
        }
    }

    #[tokio::test]
    async fn stdout_over_cap_truncates_with_metadata() {
        let offer = stub_offer(&json!({
            "risk_caps": { "max_stdout_bytes": 8, "max_stderr_bytes": 0 }
        }));
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"argv": ["echo", "abcdefghijklmnop"]}),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                let stdout = result["stdout"].as_str().expect("stdout");
                assert!(stdout.len() <= 8, "stdout={stdout:?}");
                assert_eq!(result["stdout_truncated"], true);
                assert_eq!(result["stderr_truncated"], false);
            }
            InvokeResp::Error { code, message, .. } => {
                panic!("unexpected error {code}: {message}")
            }
        }
    }
}
