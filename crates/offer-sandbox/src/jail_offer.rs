//! `sandbox.jail` — root label / path probe / policy view (no absolute secret paths).

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::jail::FilesystemJail;

/// First-party `sandbox.jail` introspection offer.
pub struct SandboxJailOffer {
    entry: CatalogEntry,
    jail: FilesystemJail,
}

impl SandboxJailOffer {
    /// # Errors
    /// Catalog id errors.
    pub fn new(jail: FilesystemJail) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("sandbox.jail", "0.1.0")?,
            jail,
        })
    }

    /// Root label only (final path component) — never the full host path.
    #[must_use]
    pub fn root_label(&self) -> String {
        self.jail
            .root()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("jail")
            .to_owned()
    }
}

impl Offer for SandboxJailOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-sandbox.jail".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_jail(&self.jail, &self.root_label(), &req.args) {
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
struct JailArgs {
    #[serde(default = "default_op")]
    op: String,
    #[serde(default)]
    path: Option<String>,
}

fn default_op() -> String {
    "policy".into()
}

fn run_jail(
    jail: &FilesystemJail,
    root_label: &str,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: JailArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("jail args: {e}")))?;
    match parsed.op.as_str() {
        "root" => Ok(json!({
            "label": root_label,
            // Never emit absolute host paths on the wire.
            "absolute": false,
        })),
        "policy" => Ok(json!({
            "ops": ["root", "probe", "policy"],
            "containment": "lexical",
            "root_label": root_label,
        })),
        "probe" => {
            let path = parsed
                .path
                .as_deref()
                .ok_or((ErrorCode::SchemaInvalid, "path required for probe".into()))?;
            if path.trim().is_empty() {
                return Err((ErrorCode::SchemaInvalid, "path must be non-empty".into()));
            }
            // Reject absolute probe inputs so we never echo host-absolute secrets.
            if std::path::Path::new(path).is_absolute() {
                return Err((
                    ErrorCode::SchemaInvalid,
                    "probe path must be relative (no absolute host paths)".into(),
                ));
            }
            match jail.resolve(path) {
                Ok(_) => Ok(json!({
                    "path": path,
                    "inside": true,
                })),
                Err(crate::JailError::Escape) => Ok(json!({
                    "path": path,
                    "inside": false,
                })),
                Err(e) => Err((e.to_error_code(), e.to_string())),
            }
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("unknown op: {other} (expected root|probe|policy)"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    fn offer() -> SandboxJailOffer {
        let tmp = tempfile::tempdir().expect("tmp");
        let jail = FilesystemJail::new(tmp.path()).expect("jail");
        SandboxJailOffer::new(jail).expect("offer")
    }

    #[tokio::test]
    async fn root_has_no_absolute_path() {
        let offer = offer();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({ "op": "root" }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["absolute"], false);
                let s = result.to_string();
                assert!(!s.contains(":\\") && !s.contains("/Users/") && !s.contains("/tmp/"));
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn probe_escape_reports_outside() {
        let offer = offer();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({ "op": "probe", "path": "../secret" }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["inside"], false);
                assert_eq!(result["path"], "../secret");
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn probe_rejects_absolute() {
        let offer = offer();
        let abs = std::env::temp_dir().join("outside-probe");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({ "op": "probe", "path": abs.to_string_lossy() }),
                offer: None,
            })
            .await;
        assert!(matches!(resp, InvokeResp::Error { .. }));
    }
}
