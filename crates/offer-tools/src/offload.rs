//! Offload oversized tool outputs to a side store (path + digest for the model).

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::loop_types::ToolResult;

/// Default max JSON chars kept inline for the model (~4 KiB).
pub const DEFAULT_INLINE_LIMIT: usize = 4096;

/// Reference returned when a tool payload is offloaded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffloadRef {
    pub digest: String,
    pub bytes: usize,
    pub preview: String,
}

/// Decide whether to keep `result.output` inline or replace with an offload stub.
#[derive(Clone, Debug)]
pub struct ResultOffload {
    inline_limit: usize,
}

impl Default for ResultOffload {
    fn default() -> Self {
        Self {
            inline_limit: DEFAULT_INLINE_LIMIT,
        }
    }
}

impl ResultOffload {
    #[must_use]
    pub fn new(inline_limit: usize) -> Self {
        Self { inline_limit }
    }

    /// Returns a model-facing [`ToolResult`] (possibly stubbed) and optional offload metadata.
    #[must_use]
    pub fn apply(&self, result: &ToolResult) -> (ToolResult, Option<OffloadRef>) {
        let raw = result.output.to_string();
        if raw.len() <= self.inline_limit {
            return (result.clone(), None);
        }
        let digest = digest_hex(&raw);
        let preview: String = raw.chars().take(120).collect();
        let off = OffloadRef {
            digest: digest.clone(),
            bytes: raw.len(),
            preview: preview.clone(),
        };
        let stub = ToolResult {
            call_id: result.call_id.clone(),
            tool: result.tool.clone(),
            ok: result.ok,
            output: json!({
                "offloaded": true,
                "digest": digest,
                "bytes": raw.len(),
                "preview": preview,
            }),
        };
        (stub, Some(off))
    }
}

fn digest_hex(raw: &str) -> String {
    let mut h = DefaultHasher::new();
    raw.hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_stays_inline() {
        let r = ToolResult {
            call_id: "1".into(),
            tool: "read".into(),
            ok: true,
            output: json!({"text": "hi"}),
        };
        let (out, meta) = ResultOffload::default().apply(&r);
        assert!(meta.is_none());
        assert_eq!(out.output, r.output);
    }

    #[test]
    fn large_is_offloaded() {
        let big = "x".repeat(5000);
        let r = ToolResult {
            call_id: "1".into(),
            tool: "read".into(),
            ok: true,
            output: json!({ "text": big }),
        };
        let (out, meta) = ResultOffload::new(100).apply(&r);
        let meta = meta.expect("offload");
        assert!(out.output["offloaded"].as_bool().unwrap());
        assert_eq!(out.output["digest"], meta.digest);
        assert!(meta.bytes > 100);
    }
}
