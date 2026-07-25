//! `research.brief` — put/get/list brief artifacts.

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::brief_store::{get_brief, list_briefs, put_brief};

/// First-party research brief offer.
pub struct ResearchBriefOffer {
    entry: CatalogEntry,
}

impl ResearchBriefOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when catalog id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("research.brief", "0.1.0")?,
        })
    }
}

impl Offer for ResearchBriefOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-research.brief".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run(&req.args) {
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
struct BriefArgs {
    action: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    20
}

fn run(args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: BriefArgs = serde_json::from_value(args.clone()).map_err(|e| {
        (
            ErrorCode::SchemaInvalid,
            format!("research.brief args: {e}"),
        )
    })?;
    let conn = persist_sqlite::open_default()
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("open db: {e}")))?;
    match parsed.action.as_str() {
        "put" => {
            let title = parsed
                .title
                .filter(|s| !s.trim().is_empty())
                .ok_or((ErrorCode::SchemaInvalid, "title required".into()))?;
            let body = parsed
                .body
                .filter(|s| !s.trim().is_empty())
                .ok_or((ErrorCode::SchemaInvalid, "body required".into()))?;
            let brief = put_brief(
                &conn,
                parsed.id.as_deref(),
                &title,
                &body,
                parsed.source_url.as_deref(),
            )
            .map_err(|c| (c, format!("{c}: put brief")))?;
            Ok(json!({
                "id": brief.id,
                "title": brief.title,
                "body": brief.body,
                "source_url": brief.source_url,
            }))
        }
        "get" => {
            let id = parsed
                .id
                .filter(|s| !s.trim().is_empty())
                .ok_or((ErrorCode::SchemaInvalid, "id required".into()))?;
            match get_brief(&conn, &id).map_err(|c| (c, format!("{c}: get brief")))? {
                Some(b) => Ok(json!({
                    "found": true,
                    "id": b.id,
                    "title": b.title,
                    "body": b.body,
                    "source_url": b.source_url,
                })),
                None => Ok(json!({ "found": false, "id": id })),
            }
        }
        "list" => {
            let briefs = list_briefs(&conn, parsed.limit as usize)
                .map_err(|c| (c, format!("{c}: list briefs")))?;
            let items: Vec<_> = briefs
                .into_iter()
                .map(|b| {
                    json!({
                        "id": b.id,
                        "title": b.title,
                        "body": b.body,
                        "source_url": b.source_url,
                    })
                })
                .collect();
            Ok(json!({ "briefs": items }))
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("unknown action: {other} (put|get|list)"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn put_get_roundtrip() {
        let tmp = tempfile::tempdir().expect("tmp");
        std::env::set_var("CONFIG_DIR", tmp.path());
        let offer = ResearchBriefOffer::new().expect("offer");
        let put = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "action": "put",
                    "title": "Brief",
                    "body": "Notes",
                    "source_url": "https://example.com"
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        let id = match put {
            InvokeResp::Ok { result, .. } => result["id"].as_str().unwrap().to_owned(),
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        };
        let get = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "action": "get", "id": id }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match get {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["found"], true);
                assert_eq!(result["title"], "Brief");
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
