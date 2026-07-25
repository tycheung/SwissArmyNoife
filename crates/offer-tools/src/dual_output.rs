//! Dual tool output: model-facing payload vs redacted audit excerpt.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::loop_types::ToolResult;
use control::redact_json;

/// Split view of a tool result for the model vs the audit log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DualOutput {
    pub model: Value,
    pub audit: Value,
}

impl DualOutput {
    /// Build from a [`ToolResult`]: model sees full `output`; audit gets redacted JSON.
    #[must_use]
    pub fn from_tool_result(result: &ToolResult) -> Self {
        Self {
            model: json!({
                "call_id": result.call_id,
                "tool": result.tool,
                "ok": result.ok,
                "output": result.output,
            }),
            audit: json!({
                "call_id": result.call_id,
                "tool": result.tool,
                "ok": result.ok,
                "output": redact_json(&result.output),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loop_types::ToolResult;

    #[test]
    fn audit_redacts_secret_keys() {
        let result = ToolResult {
            call_id: "1".into(),
            tool: "echo".into(),
            ok: true,
            output: json!({ "api_key": "sk-secret", "n": 1 }),
        };
        let dual = DualOutput::from_tool_result(&result);
        assert_eq!(dual.model["output"]["api_key"], "sk-secret");
        assert_eq!(dual.audit["output"]["api_key"], "[REDACTED]");
        assert_eq!(dual.audit["output"]["n"], 1);
    }
}
