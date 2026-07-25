//! Execute one agent step against a registry + binding allowlist.

use serde_json::{json, Value};
use types::ErrorCode;

use crate::allowlist::ToolAllowlist;
use crate::loop_types::{AgentStep, LoopBudget, ToolCall, ToolResult};
use crate::registry::ToolRegistry;

/// Dispatches a single tool call (fs/shell/adapters inject this).
pub trait ToolExecutor {
    /// Run `call.tool` with `call.args`.
    ///
    /// # Errors
    /// Executor-specific failure mapped by the caller.
    fn execute(&self, call: &ToolCall) -> Result<Value, String>;
}

/// Multi-turn tool loop driver (model turns supplied by the caller).
#[derive(Clone, Debug)]
pub struct AgentLoop {
    budget: LoopBudget,
}

impl AgentLoop {
    #[must_use]
    pub fn new(budget: LoopBudget) -> Self {
        Self { budget }
    }

    #[must_use]
    pub fn budget(&self) -> &LoopBudget {
        &self.budget
    }

    /// Validate + execute every tool call on `step` (one loop turn).
    ///
    /// # Errors
    /// [`ErrorCode::BudgetExhausted`] when `step_index` is past the budget;
    /// [`ErrorCode::PolicyDenied`] / schema errors from the registry gate.
    pub fn run_tools(
        &self,
        step_index: u32,
        step: &AgentStep,
        registry: &ToolRegistry,
        allowlist: &ToolAllowlist,
        executor: &impl ToolExecutor,
    ) -> Result<Vec<ToolResult>, ErrorCode> {
        if step_index >= self.budget.max_steps {
            return Err(ErrorCode::BudgetExhausted);
        }
        let mut out = Vec::with_capacity(step.tool_calls.len());
        for call in &step.tool_calls {
            registry.get_allowed(allowlist, &call.tool)?;
            registry.validate_args(&call.tool, &call.args)?;
            let result = match executor.execute(call) {
                Ok(output) => ToolResult {
                    call_id: call.id.clone(),
                    tool: call.tool.clone(),
                    ok: true,
                    output,
                },
                Err(message) => ToolResult {
                    call_id: call.id.clone(),
                    tool: call.tool.clone(),
                    ok: false,
                    output: json!({ "error": message }),
                },
            };
            out.push(result);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolSpec;
    use serde_json::json;

    struct EchoExec;
    impl ToolExecutor for EchoExec {
        fn execute(&self, call: &ToolCall) -> Result<Value, String> {
            Ok(json!({ "echo": call.args }))
        }
    }

    fn registry_with_echo() -> ToolRegistry {
        let mut reg = ToolRegistry::new();
        reg.register(
            ToolSpec::new(
                "echo",
                "Echo",
                json!({
                    "type": "object",
                    "properties": { "message": { "type": "string" } },
                    "required": ["message"]
                }),
            )
            .expect("spec"),
        )
        .expect("reg");
        reg
    }

    #[test]
    fn run_tools_happy_path() {
        let loop_ = AgentLoop::new(LoopBudget::new(4));
        let step = AgentStep {
            text: None,
            tool_calls: vec![ToolCall {
                id: "1".into(),
                tool: "echo".into(),
                args: json!({"message": "hi"}),
            }],
        };
        let results = loop_
            .run_tools(
                0,
                &step,
                &registry_with_echo(),
                &ToolAllowlist::unrestricted(),
                &EchoExec,
            )
            .expect("ok");
        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        assert_eq!(results[0].output["echo"]["message"], "hi");
    }

    #[test]
    fn run_tools_policy_denied() {
        let loop_ = AgentLoop::new(LoopBudget::default());
        let step = AgentStep {
            text: None,
            tool_calls: vec![ToolCall {
                id: "1".into(),
                tool: "echo".into(),
                args: json!({"message": "x"}),
            }],
        };
        let deny = ToolAllowlist::from_policy(&json!({ "tools": { "allow": ["shell"] } }));
        assert_eq!(
            loop_
                .run_tools(0, &step, &registry_with_echo(), &deny, &EchoExec)
                .err(),
            Some(ErrorCode::PolicyDenied)
        );
    }

    #[test]
    fn budget_exhausts() {
        let loop_ = AgentLoop::new(LoopBudget::new(1));
        let step = AgentStep::final_text("x");
        assert_eq!(
            loop_
                .run_tools(
                    1,
                    &step,
                    &registry_with_echo(),
                    &ToolAllowlist::unrestricted(),
                    &EchoExec
                )
                .err(),
            Some(ErrorCode::BudgetExhausted)
        );
    }
}
