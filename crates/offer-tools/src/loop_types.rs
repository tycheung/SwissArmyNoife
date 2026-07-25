//! `tools.loop` — multi-turn JIT step types (provider via `llm.chat` later).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One model-requested tool invocation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool: String,
    pub args: Value,
}

/// Result of executing a [`ToolCall`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub tool: String,
    pub ok: bool,
    pub output: Value,
}

/// One loop iteration from the model (text and/or tool calls).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

impl AgentStep {
    /// Whether the loop should continue executing tools.
    #[must_use]
    pub fn wants_tools(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Terminal assistant text with no tool calls.
    #[must_use]
    pub fn final_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: Vec::new(),
        }
    }
}

/// Caps for a loop run (often mirrored from binding risk policy).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopBudget {
    pub max_steps: u32,
}

impl Default for LoopBudget {
    fn default() -> Self {
        Self { max_steps: 32 }
    }
}

impl LoopBudget {
    #[must_use]
    pub fn new(max_steps: u32) -> Self {
        Self { max_steps }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn step_wants_tools_and_final_text() {
        let step = AgentStep {
            text: Some("thinking".into()),
            tool_calls: vec![ToolCall {
                id: "1".into(),
                tool: "read".into(),
                args: json!({"path": "a.rs"}),
            }],
        };
        assert!(step.wants_tools());
        let done = AgentStep::final_text("done");
        assert!(!done.wants_tools());
        assert_eq!(done.text.as_deref(), Some("done"));
    }

    #[test]
    fn tool_call_roundtrip_json() {
        let call = ToolCall {
            id: "c1".into(),
            tool: "shell".into(),
            args: json!({"argv": ["echo"]}),
        };
        let v = serde_json::to_value(&call).expect("ser");
        let back: ToolCall = serde_json::from_value(v).expect("de");
        assert_eq!(back, call);
    }
}
