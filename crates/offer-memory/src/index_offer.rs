//! `memory.index` — rebuild / fingerprint-skip.

use std::sync::{Arc, Mutex};

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::plane::MemoryPlane;
use crate::vector::BackendKind;

/// First-party `memory.index` offer.
pub struct MemoryIndexOffer {
    entry: CatalogEntry,
    plane: Arc<MemoryPlane>,
    backend: Mutex<BackendKind>,
}

impl MemoryIndexOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new(plane: Arc<MemoryPlane>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("memory.index", "0.1.0")?,
            plane,
            backend: Mutex::new(BackendKind::Exact),
        })
    }

    #[must_use]
    pub fn plane(&self) -> Arc<MemoryPlane> {
        Arc::clone(&self.plane)
    }
}

impl Offer for MemoryIndexOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-memory.index".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let kind = params
            .pointer("/memory/backend")
            .and_then(Value::as_str)
            .and_then(BackendKind::parse)
            .or_else(|| {
                params
                    .get("backend")
                    .and_then(Value::as_str)
                    .and_then(BackendKind::parse)
            })
            .unwrap_or(BackendKind::Exact);
        {
            let mut b = self.backend.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
            *b = kind;
        }
        self.plane.set_backend(kind);
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_index(&self.plane, &req.args) {
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
struct DocArg {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct IndexArgs {
    documents: Vec<DocArg>,
    #[serde(default)]
    scope_key: Option<String>,
}

fn run_index(plane: &MemoryPlane, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: IndexArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("index args: {e}")))?;
    if parsed.documents.is_empty() {
        return Err((
            ErrorCode::SchemaInvalid,
            "documents must be non-empty".into(),
        ));
    }
    let docs: Vec<_> = parsed
        .documents
        .into_iter()
        .map(|d| (d.id, d.text))
        .collect();
    let scope = parsed.scope_key.unwrap_or_else(|| "default".into());
    let (rebuilt, count, fp) = plane
        .rebuild(&docs, &scope)
        .map_err(|e| (ErrorCode::SchemaInvalid, e))?;
    let (_, backend, _) = plane.meta().map_err(|e| (ErrorCode::SchemaInvalid, e))?;
    if let Ok(conn) = persist_sqlite::open_default() {
        let _ = crate::upsert_index_meta(
            &conn,
            &scope,
            &fp,
            &backend,
            i64::try_from(count).unwrap_or(i64::MAX),
        );
    }
    Ok(json!({
        "rebuilt": rebuilt,
        "vector_count": count,
        "fingerprint": fp,
        "backend": backend,
        "scope_key": scope,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn index_rebuild_once() {
        let plane = Arc::new(MemoryPlane::new());
        let offer = MemoryIndexOffer::new(Arc::clone(&plane)).expect("offer");
        offer
            .bind(BindingId::new(), json!({ "memory": { "backend": "hnsw" } }))
            .await
            .expect("bind");
        let args = json!({
            "documents": [
                {"id": "a", "text": "alpha rust"},
                {"id": "b", "text": "beta python"}
            ]
        });
        let r1 = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: args.clone(),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match r1 {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["rebuilt"], true);
                assert_eq!(result["backend"], "hnsw");
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
        let r2 = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args,
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match r2 {
            InvokeResp::Ok { result, .. } => assert_eq!(result["rebuilt"], false),
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
