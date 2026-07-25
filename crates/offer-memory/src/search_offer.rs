//! `memory.search` — query shared plane + excerpt formatting.

use std::sync::Arc;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::plane::{excerpt, MemoryPlane};

/// First-party `memory.search` offer.
pub struct MemorySearchOffer {
    entry: CatalogEntry,
    plane: Arc<MemoryPlane>,
}

impl MemorySearchOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new(plane: Arc<MemoryPlane>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("memory.search", "0.1.0")?,
            plane,
        })
    }
}

impl Offer for MemorySearchOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-memory.search".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_search(&self.plane, &req.args) {
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
struct SearchArgs {
    query: String,
    #[serde(default = "default_k")]
    k: u32,
    #[serde(default = "default_excerpt")]
    excerpt_chars: u32,
}

fn default_k() -> u32 {
    5
}

fn default_excerpt() -> u32 {
    120
}

fn run_search(plane: &MemoryPlane, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: SearchArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("search args: {e}")))?;
    if parsed.query.trim().is_empty() {
        return Err((ErrorCode::SchemaInvalid, "query must be non-empty".into()));
    }
    let hits = plane
        .search(&parsed.query, parsed.k as usize)
        .map_err(|e| (ErrorCode::SchemaInvalid, e))?;
    let max_c = parsed.excerpt_chars as usize;
    let hits: Vec<_> = hits
        .into_iter()
        .map(|h| {
            json!({
                "id": h.id,
                "score": h.score,
                "excerpt": excerpt(&h.text, max_c),
            })
        })
        .collect();
    Ok(json!({ "hits": hits, "query": parsed.query }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index_offer::MemoryIndexOffer;
    use types::InvokeId;

    #[tokio::test]
    async fn search_after_index() {
        let plane = Arc::new(MemoryPlane::new());
        let index = MemoryIndexOffer::new(Arc::clone(&plane)).expect("index");
        let search = MemorySearchOffer::new(Arc::clone(&plane)).expect("search");
        index
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "documents": [
                        {"id": "1", "text": "swiss army knife memory search"},
                        {"id": "2", "text": "unrelated gardening tips"}
                    ]
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        let resp = search
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "query": "memory search", "k": 1 }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                let hits = result["hits"].as_array().unwrap();
                assert!(!hits.is_empty());
                assert!(hits[0]["excerpt"].as_str().unwrap().contains("memory"));
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
