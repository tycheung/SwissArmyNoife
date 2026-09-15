//! Playwright Node process sidecar (`BROWSER_BACKEND=playwright`).

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use types::ErrorCode;

use crate::driver::{BrowserBackend, BrowserBackendKind};

/// Long-lived Node + Playwright child speaking JSON-lines on stdio.
pub struct PlaywrightBackend {
    script: PathBuf,
    child: Mutex<Option<Sidecar>>,
}

struct Sidecar {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl PlaywrightBackend {
    /// Resolve sidecar script next to the crate (dev) or `BROWSER_SIDECAR` env.
    ///
    /// # Errors
    /// [`ErrorCode::ProviderUnreachable`] when the script file is missing.
    pub fn from_env() -> Result<Self, ErrorCode> {
        let script = if let Ok(p) = std::env::var("BROWSER_SIDECAR") {
            PathBuf::from(p)
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecar/browser_sidecar.mjs")
        };
        if !script.is_file() {
            return Err(ErrorCode::ProviderUnreachable);
        }
        Ok(Self {
            script,
            child: Mutex::new(None),
        })
    }

    async fn roundtrip(&self, req: Value) -> Result<Value, ErrorCode> {
        let mut guard = self.child.lock().await;
        if guard.is_none() {
            let mut child = Command::new("node")
                .arg(&self.script)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| ErrorCode::ProviderUnreachable)?;
            let stdin = child.stdin.take().ok_or(ErrorCode::ProviderUnreachable)?;
            let stdout = child.stdout.take().ok_or(ErrorCode::ProviderUnreachable)?;
            *guard = Some(Sidecar {
                _child: child,
                stdin,
                stdout: BufReader::new(stdout),
            });
        }
        let side = guard.as_mut().ok_or(ErrorCode::ProviderUnreachable)?;
        let line = format!("{req}\n");
        side.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        side.stdin
            .flush()
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        let mut buf = String::new();
        let n = side
            .stdout
            .read_line(&mut buf)
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        if n == 0 {
            *guard = None;
            return Err(ErrorCode::ProviderUnreachable);
        }
        let resp: Value = serde_json::from_str(buf.trim()).map_err(|_| ErrorCode::SchemaInvalid)?;
        if resp.get("ok").and_then(Value::as_bool) != Some(true) {
            let err = resp
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("sidecar error");
            let code = resp.get("code").and_then(Value::as_str).unwrap_or("");
            tracing::warn!(error = err, code, "browser sidecar error");
            if code.contains("policy") || err.contains("policy.denied") {
                return Err(ErrorCode::PolicyDenied);
            }
            if err.contains("unknown ref") || err.contains("schema") {
                return Err(ErrorCode::SchemaInvalid);
            }
            return Err(ErrorCode::ProviderUnreachable);
        }
        Ok(resp)
    }
}

impl BrowserBackend for PlaywrightBackend {
    fn kind(&self) -> BrowserBackendKind {
        BrowserBackendKind::Playwright
    }

    async fn call(&self, profile_dir: &Path, op: &str, params: Value) -> Result<Value, ErrorCode> {
        std::fs::create_dir_all(profile_dir).map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut req = params;
        if let Some(obj) = req.as_object_mut() {
            obj.insert("op".into(), json!(op));
            obj.insert("profile".into(), json!(profile_dir.display().to_string()));
        } else {
            req = json!({
                "op": op,
                "profile": profile_dir.display().to_string(),
            });
        }
        let mut resp = self.roundtrip(req).await?;
        if let Some(obj) = resp.as_object_mut() {
            obj.insert(
                "backend".into(),
                json!(BrowserBackendKind::Playwright.as_str()),
            );
        }
        if op == "close" {
            let _ = std::fs::remove_dir_all(profile_dir);
        }
        Ok(resp)
    }
}
