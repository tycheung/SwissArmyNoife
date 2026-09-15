//! `browser.session` — full browser automation with binding-frozen egress.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use offer_egress::EgressPolicy;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::driver::BrowserBackend;
use crate::playwright::PlaywrightBackend;
use crate::stub::StubBackend;

/// Env: `stub` (CI default) | `playwright` (product).
pub const BROWSER_BACKEND: &str = "BROWSER_BACKEND";

enum Driver {
    Stub(StubBackend),
    Playwright(Box<PlaywrightBackend>),
}

struct SessionState {
    policy: EgressPolicy,
    principal: String,
    profile_dir: PathBuf,
}

/// First-party browser session offer (ADR 013 / sak596-e+).
pub struct BrowserSessionOffer {
    entry: CatalogEntry,
    root: PathBuf,
    driver: Driver,
    sessions: Mutex<HashMap<BindingId, SessionState>>,
}

impl BrowserSessionOffer {
    /// Build from `BROWSER_BACKEND` and `CONFIG_DIR/browser` root.
    ///
    /// # Errors
    /// Catalog / mkdir / playwright script resolution failures.
    pub fn from_env(root: impl Into<PathBuf>) -> Result<Self, ErrorCode> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|_| ErrorCode::SchemaInvalid)?;
        let raw = std::env::var(BROWSER_BACKEND).unwrap_or_default();
        let driver = if raw.eq_ignore_ascii_case("playwright") {
            tracing::info!(root = %root.display(), "browser backend=playwright");
            Driver::Playwright(Box::new(PlaywrightBackend::from_env()?))
        } else {
            tracing::info!(root = %root.display(), "browser backend=stub");
            Driver::Stub(StubBackend::new())
        };
        Ok(Self {
            entry: CatalogEntry::new("browser.session", "0.2.0")?,
            root,
            driver,
            sessions: Mutex::new(HashMap::new()),
        })
    }

    /// Test helper with stub driver and explicit root.
    ///
    /// # Errors
    /// Catalog id construction.
    pub fn stub_at(root: impl Into<PathBuf>) -> Result<Self, ErrorCode> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(Self {
            entry: CatalogEntry::new("browser.session", "0.2.0")?,
            root,
            driver: Driver::Stub(StubBackend::new()),
            sessions: Mutex::new(HashMap::new()),
        })
    }

    #[must_use]
    pub fn backend_label(&self) -> &'static str {
        match &self.driver {
            Driver::Stub(_) => "stub",
            Driver::Playwright(_) => "playwright",
        }
    }

    fn profile_for(root: &Path, binding_id: BindingId) -> PathBuf {
        root.join(binding_id.to_string())
    }

    async fn driver_call(
        driver: &Driver,
        profile: &Path,
        op: &str,
        params: Value,
    ) -> Result<Value, ErrorCode> {
        match driver {
            Driver::Stub(b) => b.call(profile, op, params).await,
            Driver::Playwright(b) => b.call(profile, op, params).await,
        }
    }
}

impl Offer for BrowserSessionOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok(format!("res-browser.session-{}", self.backend_label()))
    }

    async fn bind(&self, binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let policy = EgressPolicy::from_policy(&params);
        let principal = params
            .get("principal")
            .and_then(Value::as_str)
            .unwrap_or("local")
            .to_owned();
        let profile_dir = Self::profile_for(&self.root, binding_id);
        std::fs::create_dir_all(&profile_dir).map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut sessions = self.sessions.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        sessions.insert(
            binding_id,
            SessionState {
                policy,
                principal,
                profile_dir,
            },
        );
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run(&self.driver, &self.sessions, req.binding_id, &req.args).await {
            Ok(result) => InvokeResp::ok(invoke_id, result),
            Err((code, message)) => InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code,
                message,
            },
        }
    }

    async fn unbind(&self, binding_id: BindingId) -> Result<(), ErrorCode> {
        let profile = {
            let mut sessions = self.sessions.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
            sessions.remove(&binding_id).map(|s| s.profile_dir)
        };
        if let Some(profile_dir) = profile {
            let _ = Self::driver_call(&self.driver, &profile_dir, "close", json!({})).await;
        }
        Ok(())
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        Ok(())
    }
}

async fn run(
    driver: &Driver,
    sessions: &Mutex<HashMap<BindingId, SessionState>>,
    binding_id: BindingId,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let action = args
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            (
                ErrorCode::SchemaInvalid,
                "browser.session requires action".into(),
            )
        })?
        .to_owned();

    let (policy, principal, profile_dir) = {
        let sessions = sessions
            .lock()
            .map_err(|_| (ErrorCode::SchemaInvalid, "session lock".into()))?;
        let Some(state) = sessions.get(&binding_id) else {
            return Err((
                ErrorCode::BindingExpired,
                "browser.session: unknown binding".into(),
            ));
        };
        (
            state.policy.clone(),
            state.principal.clone(),
            state.profile_dir.clone(),
        )
    };

    let mut params = args.clone();
    if let Some(obj) = params.as_object_mut() {
        obj.remove("action");
    }

    if action == "navigate" {
        let url = params.get("url").and_then(Value::as_str).ok_or_else(|| {
            (
                ErrorCode::SchemaInvalid,
                "browser.session navigate requires url".into(),
            )
        })?;
        policy
            .check(&principal, url)
            .map_err(|code| (code, format!("{code}: browser navigate denied for {url}")))?;
    }

    if action == "cdp" {
        let method = params.get("method").and_then(Value::as_str).unwrap_or("");
        if method.starts_with("Input.") {
            return Err((
                ErrorCode::PolicyDenied,
                "policy.denied: Input.* CDP methods are forbidden".into(),
            ));
        }
    }

    let op = match action.as_str() {
        "navigate" => "navigate",
        "snapshot" => "snapshot",
        "click" => "click",
        "type" => "type",
        "fill" => "fill",
        "press_key" => "press_key",
        "scroll" => "scroll",
        "select_option" => "select_option",
        "drag" => "drag",
        "mouse_click_xy" => "mouse_click_xy",
        "take_screenshot" => "take_screenshot",
        "highlight" => "highlight",
        "get_bounding_box" => "get_bounding_box",
        "tabs" => "tabs",
        "lock" => "lock",
        "console" => "console",
        "network" => "network",
        "failure_report" => "failure_report",
        "cdp" => "cdp",
        other => {
            return Err((
                ErrorCode::SchemaInvalid,
                format!("browser.session unknown action: {other}"),
            ));
        }
    };

    let mut result = BrowserSessionOffer::driver_call(driver, &profile_dir, op, params)
        .await
        .map_err(|code| (code, format!("{code}: browser {action} failed")))?;
    if let Some(obj) = result.as_object_mut() {
        obj.entry("action".to_string())
            .or_insert_with(|| json!(action));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    async fn bind_stub(tmp: &Path) -> (BrowserSessionOffer, BindingId) {
        let offer = BrowserSessionOffer::stub_at(tmp).expect("offer");
        let binding = BindingId::new();
        offer
            .bind(
                binding,
                json!({
                    "egress": {
                        "allow_hosts": ["example.com"],
                        "allow_principals": ["local"]
                    },
                    "principal": "local"
                }),
            )
            .await
            .expect("bind");
        (offer, binding)
    }

    #[tokio::test]
    async fn navigate_snapshot_refs_and_click() {
        let tmp = tempfile::tempdir().expect("tmp");
        let (offer, binding) = bind_stub(tmp.path()).await;

        let nav = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "navigate", "url": "https://example.com/" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        assert!(matches!(nav, InvokeResp::Ok { .. }));

        let snap = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "snapshot" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        let InvokeResp::Ok { result, .. } = snap else {
            panic!("{snap:?}");
        };
        let text = result["snapshot"].as_str().expect("snapshot");
        assert!(text.contains("[e1]"), "refs missing: {text}");
        assert!(text.contains("[e4]"), "button ref missing: {text}");

        let click = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "click", "ref": "e4" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        assert!(matches!(click, InvokeResp::Ok { .. }), "{click:?}");
    }

    #[tokio::test]
    async fn failure_report_and_cdp_deny() {
        let tmp = tempfile::tempdir().expect("tmp");
        let (offer, binding) = bind_stub(tmp.path()).await;
        let _ = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "navigate", "url": "https://example.com/" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;

        let report = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "failure_report", "step": "checkout" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match report {
            InvokeResp::Ok { result, .. } => {
                assert!(result["screenshot_path"].as_str().is_some());
                assert_eq!(result["step"], "checkout");
                assert!(result["console"].is_array());
            }
            other => panic!("{other:?}"),
        }

        let cdp = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "cdp", "method": "Input.dispatchKeyEvent" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match cdp {
            InvokeResp::Error { code, .. } => assert_eq!(code, ErrorCode::PolicyDenied),
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn navigate_egress_denied() {
        let tmp = tempfile::tempdir().expect("tmp");
        let (offer, binding) = bind_stub(tmp.path()).await;
        let resp = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({ "action": "navigate", "url": "https://evil.example/" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error { code, .. } => assert_eq!(code, ErrorCode::EgressDenied),
            other => panic!("expected deny, got {other:?}"),
        }
    }
}
