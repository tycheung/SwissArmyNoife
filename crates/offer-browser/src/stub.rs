//! Deterministic full-protocol stub (`BROWSER_BACKEND=stub`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use types::ErrorCode;

use crate::driver::{BrowserBackend, BrowserBackendKind};

/// 1x1 PNG (transparent).
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TabState {
    url: String,
    title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RefMeta {
    role: String,
    name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StubState {
    tabs: Vec<TabState>,
    active: usize,
    locked: bool,
    refs: BTreeMap<String, RefMeta>,
    console: Vec<Value>,
    network: Vec<Value>,
    next_ref: u32,
}

impl Default for StubState {
    fn default() -> Self {
        Self {
            tabs: vec![TabState {
                url: "about:blank".into(),
                title: "Stub page".into(),
            }],
            active: 0,
            locked: false,
            refs: BTreeMap::new(),
            console: Vec::new(),
            network: Vec::new(),
            next_ref: 1,
        }
    }
}

/// Stub driver: full protocol fidelity without Chromium.
#[derive(Debug, Default)]
pub struct StubBackend {
    lock: Mutex<()>,
}

impl StubBackend {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn state_path(profile: &Path) -> PathBuf {
        profile.join("stub_state.json")
    }

    fn load(profile: &Path) -> Result<StubState, ErrorCode> {
        let path = Self::state_path(profile);
        if !path.is_file() {
            return Ok(StubState::default());
        }
        let raw = std::fs::read_to_string(&path).map_err(|_| ErrorCode::SchemaInvalid)?;
        serde_json::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)
    }

    fn save(profile: &Path, state: &StubState) -> Result<(), ErrorCode> {
        std::fs::create_dir_all(profile).map_err(|_| ErrorCode::SchemaInvalid)?;
        let raw = serde_json::to_string_pretty(state).map_err(|_| ErrorCode::SchemaInvalid)?;
        std::fs::write(Self::state_path(profile), raw).map_err(|_| ErrorCode::SchemaInvalid)
    }

    fn active_tab(state: &StubState) -> &TabState {
        state.tabs.get(state.active).unwrap_or(&state.tabs[0])
    }

    fn rebuild_refs(state: &mut StubState) {
        state.refs.clear();
        state.next_ref = 1;
        let title = Self::active_tab(state).title.clone();
        let url = Self::active_tab(state).url.clone();
        for (role, name) in [
            ("document", title.as_str()),
            ("heading", title.as_str()),
            ("link", url.as_str()),
            ("button", "Stub Action"),
            ("textbox", "Stub Input"),
            ("combobox", "Stub Select"),
        ] {
            let id = format!("e{}", state.next_ref);
            state.next_ref += 1;
            state.refs.insert(
                id,
                RefMeta {
                    role: role.into(),
                    name: name.into(),
                },
            );
        }
    }

    fn snapshot_text(state: &StubState) -> String {
        let mut out = String::new();
        for (id, meta) in &state.refs {
            let _ = writeln!(out, "- {} \"{}\" [{id}]", meta.role, meta.name);
        }
        out
    }

    fn require_unlocked(state: &StubState) -> Result<(), ErrorCode> {
        if state.locked {
            Err(ErrorCode::PolicyDenied)
        } else {
            Ok(())
        }
    }

    fn require_ref<'a>(state: &'a StubState, r: &str) -> Result<&'a RefMeta, ErrorCode> {
        state.refs.get(r).ok_or(ErrorCode::SchemaInvalid)
    }

    fn usize_param(params: &Value, key: &str) -> Result<Option<usize>, ErrorCode> {
        match params.get(key).and_then(Value::as_u64) {
            None => Ok(None),
            Some(v) => usize::try_from(v)
                .map(Some)
                .map_err(|_| ErrorCode::SchemaInvalid),
        }
    }

    fn limit_param(params: &Value) -> usize {
        params
            .get("limit")
            .and_then(Value::as_u64)
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(50)
    }

    fn write_shot(profile: &Path, name: &str) -> Result<PathBuf, ErrorCode> {
        let shots = profile.join("shots");
        std::fs::create_dir_all(&shots).map_err(|_| ErrorCode::SchemaInvalid)?;
        let path = shots.join(name);
        std::fs::write(&path, TINY_PNG).map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(path)
    }

    fn op_navigate(
        state: &mut StubState,
        params: &Value,
        backend: &str,
    ) -> Result<Value, ErrorCode> {
        Self::require_unlocked(state)?;
        let url = params
            .get("url")
            .and_then(Value::as_str)
            .ok_or(ErrorCode::SchemaInvalid)?
            .to_owned();
        if let Some(tab) = state.tabs.get_mut(state.active) {
            tab.url.clone_from(&url);
            tab.title = "Stub page".into();
        }
        state.console.push(json!({
            "type": "log",
            "text": format!("navigated {url}"),
            "ts": 0
        }));
        Self::rebuild_refs(state);
        let tab = Self::active_tab(state);
        Ok(json!({
            "ok": true,
            "url": tab.url,
            "title": tab.title,
            "backend": backend
        }))
    }

    fn op_snapshot(state: &mut StubState, backend: &str) -> Value {
        if state.refs.is_empty() {
            Self::rebuild_refs(state);
        }
        let tab = Self::active_tab(state);
        json!({
            "ok": true,
            "url": tab.url,
            "title": tab.title,
            "snapshot": Self::snapshot_text(state),
            "backend": backend
        })
    }

    fn op_act(
        state: &mut StubState,
        op: &str,
        params: &Value,
        backend: &str,
    ) -> Result<Value, ErrorCode> {
        Self::require_unlocked(state)?;
        if let Some(r) = params.get("ref").and_then(Value::as_str) {
            let _ = Self::require_ref(state, r)?;
        }
        if op == "type" || op == "fill" {
            let text = params.get("text").and_then(Value::as_str).unwrap_or("");
            state.console.push(json!({
                "type": "log",
                "text": format!("{op}:{text}"),
                "ts": 0
            }));
        }
        Ok(json!({ "ok": true, "op": op, "backend": backend }))
    }

    fn op_tabs(state: &mut StubState, params: &Value, backend: &str) -> Result<Value, ErrorCode> {
        let action = params
            .get("tabs_action")
            .or_else(|| params.get("action"))
            .and_then(Value::as_str)
            .unwrap_or("list");
        match action {
            "list" => Ok(json!({
                "ok": true,
                "tabs": state.tabs,
                "active": state.active,
                "backend": backend
            })),
            "new" => {
                Self::require_unlocked(state)?;
                state.tabs.push(TabState {
                    url: "about:blank".into(),
                    title: "New Tab".into(),
                });
                state.active = state.tabs.len() - 1;
                Self::rebuild_refs(state);
                Ok(json!({
                    "ok": true,
                    "active": state.active,
                    "tabs": state.tabs,
                    "backend": backend
                }))
            }
            "select" => {
                let idx = Self::usize_param(params, "index")?.ok_or(ErrorCode::SchemaInvalid)?;
                if idx >= state.tabs.len() {
                    return Err(ErrorCode::SchemaInvalid);
                }
                state.active = idx;
                Self::rebuild_refs(state);
                Ok(json!({ "ok": true, "active": state.active, "backend": backend }))
            }
            "close" => {
                Self::require_unlocked(state)?;
                if state.tabs.len() <= 1 {
                    return Err(ErrorCode::SchemaInvalid);
                }
                let idx = Self::usize_param(params, "index")?.unwrap_or(state.active);
                if idx >= state.tabs.len() {
                    return Err(ErrorCode::SchemaInvalid);
                }
                state.tabs.remove(idx);
                if state.active >= state.tabs.len() {
                    state.active = state.tabs.len() - 1;
                }
                Self::rebuild_refs(state);
                Ok(json!({
                    "ok": true,
                    "active": state.active,
                    "tabs": state.tabs,
                    "backend": backend
                }))
            }
            _ => Err(ErrorCode::SchemaInvalid),
        }
    }

    fn op_failure_report(
        state: &mut StubState,
        profile: &Path,
        params: &Value,
        backend: &str,
    ) -> Result<Value, ErrorCode> {
        let path = Self::write_shot(profile, "failure.png")?;
        if state.refs.is_empty() {
            Self::rebuild_refs(state);
        }
        let tab = Self::active_tab(state);
        let excerpt: String = Self::snapshot_text(state).chars().take(2000).collect();
        Ok(json!({
            "ok": true,
            "url": tab.url,
            "title": tab.title,
            "step": params.get("step").cloned().unwrap_or(Value::Null),
            "screenshot_path": path.display().to_string(),
            "snapshot_excerpt": excerpt,
            "console": state.console.iter().rev().take(20).cloned().collect::<Vec<_>>(),
            "network": state.network.iter().rev().take(20).cloned().collect::<Vec<_>>(),
            "ts": 0,
            "backend": backend
        }))
    }
}

impl BrowserBackend for StubBackend {
    fn kind(&self) -> BrowserBackendKind {
        BrowserBackendKind::Stub
    }

    async fn call(&self, profile_dir: &Path, op: &str, params: Value) -> Result<Value, ErrorCode> {
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        std::fs::create_dir_all(profile_dir).map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut state = Self::load(profile_dir)?;
        let backend = BrowserBackendKind::Stub.as_str();

        let result = match op {
            "ping" => json!({ "ok": true, "backend": backend }),
            "navigate" => Self::op_navigate(&mut state, &params, backend)?,
            "snapshot" => Self::op_snapshot(&mut state, backend),
            "click" | "type" | "fill" | "press_key" | "scroll" | "select_option" | "drag"
            | "mouse_click_xy" | "highlight" => Self::op_act(&mut state, op, &params, backend)?,
            "get_bounding_box" => {
                let r = params
                    .get("ref")
                    .and_then(Value::as_str)
                    .ok_or(ErrorCode::SchemaInvalid)?;
                let _ = Self::require_ref(&state, r)?;
                json!({
                    "ok": true,
                    "ref": r,
                    "x": 10.0,
                    "y": 20.0,
                    "width": 100.0,
                    "height": 30.0,
                    "backend": backend
                })
            }
            "take_screenshot" => {
                let path = Self::write_shot(profile_dir, "stub.png")?;
                json!({
                    "ok": true,
                    "path": path.display().to_string(),
                    "bytes": TINY_PNG.len(),
                    "backend": backend
                })
            }
            "tabs" => Self::op_tabs(&mut state, &params, backend)?,
            "lock" => {
                let action = params
                    .get("lock_action")
                    .or_else(|| params.get("action"))
                    .and_then(Value::as_str)
                    .unwrap_or("lock");
                state.locked = action != "unlock";
                json!({ "ok": true, "locked": state.locked, "backend": backend })
            }
            "console" => {
                let limit = Self::limit_param(&params);
                let events: Vec<_> = state.console.iter().rev().take(limit).cloned().collect();
                json!({ "ok": true, "events": events, "backend": backend })
            }
            "network" => {
                let limit = Self::limit_param(&params);
                if state.network.is_empty() {
                    state.network.push(json!({
                        "url": Self::active_tab(&state).url,
                        "status": 200,
                        "ok": true
                    }));
                }
                let events: Vec<_> = state.network.iter().rev().take(limit).cloned().collect();
                json!({ "ok": true, "events": events, "backend": backend })
            }
            "failure_report" => Self::op_failure_report(&mut state, profile_dir, &params, backend)?,
            "cdp" => {
                let method = params
                    .get("method")
                    .and_then(Value::as_str)
                    .ok_or(ErrorCode::SchemaInvalid)?;
                if method.starts_with("Input.") {
                    return Err(ErrorCode::PolicyDenied);
                }
                json!({
                    "ok": true,
                    "method": method,
                    "result": { "stub": true },
                    "backend": backend
                })
            }
            "close" => {
                let _ = std::fs::remove_dir_all(profile_dir);
                return Ok(json!({ "ok": true, "backend": backend }));
            }
            _ => return Err(ErrorCode::SchemaInvalid),
        };

        Self::save(profile_dir, &state)?;
        Ok(result)
    }
}
