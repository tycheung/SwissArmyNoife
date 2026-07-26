//! `tools.loop` — run one agent tool step via [`AgentLoop`].

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::agent_loop::{AgentLoop, ToolExecutor};
use crate::allowlist::ToolAllowlist;
use crate::loop_types::{AgentStep, LoopBudget, ToolCall};
use crate::registry::{ToolRegistry, ToolSpec};

/// Echo executor for default registry tools (`tools.echo` / `tools.ping`).
struct EchoExec;

impl ToolExecutor for EchoExec {
    fn execute(&self, call: &ToolCall) -> Result<Value, String> {
        match call.tool.as_str() {
            "tools.echo" => Ok(json!({ "echo": call.args })),
            "tools.ping" => Ok(json!({ "pong": true })),
            other => Err(format!("no executor for {other}")),
        }
    }
}

/// First-party `tools.loop` offer.
pub struct ToolsLoopOffer {
    entry: CatalogEntry,
    registry: ToolRegistry,
    allowlist: Mutex<ToolAllowlist>,
    budget: Mutex<LoopBudget>,
}

impl ToolsLoopOffer {
    /// # Errors
    /// Catalog / register errors.
    pub fn new(registry: ToolRegistry) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("tools.loop", "0.1.0")?,
            registry,
            allowlist: Mutex::new(ToolAllowlist::unrestricted()),
            budget: Mutex::new(LoopBudget::default()),
        })
    }

    /// Seed echo + ping tools (matches `tools.registry` defaults).
    ///
    /// # Errors
    /// Propagates catalog or register errors.
    pub fn with_defaults() -> Result<Self, ErrorCode> {
        let mut reg = ToolRegistry::new();
        reg.register(ToolSpec::new(
            "tools.echo",
            "Echo a message",
            json!({
                "type": "object",
                "properties": { "message": { "type": "string" } },
                "required": ["message"]
            }),
        )?)?;
        reg.register(ToolSpec::new(
            "tools.ping",
            "Ping",
            json!({ "type": "object", "properties": {} }),
        )?)?;
        Self::new(reg)
    }
}

impl Offer for ToolsLoopOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-tools.loop".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        {
            let mut g = self
                .allowlist
                .lock()
                .map_err(|_| ErrorCode::SchemaInvalid)?;
            *g = ToolAllowlist::from_policy(&params);
        }
        let max_steps = params
            .pointer("/tools/max_steps")
            .or_else(|| params.get("max_steps"))
            .and_then(Value::as_u64)
            .map_or(32, |n| u32::try_from(n).unwrap_or(u32::MAX));
        {
            let mut g = self.budget.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
            *g = LoopBudget::new(max_steps.max(1));
        }
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        let allow = self
            .allowlist
            .lock()
            .map_or_else(|_| ToolAllowlist::unrestricted(), |g| g.clone());
        let budget = self
            .budget
            .lock()
            .map_or_else(|_| LoopBudget::default(), |g| g.clone());
        match run_loop(&self.registry, &allow, &budget, &req.args) {
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
struct LoopArgs {
    #[serde(default)]
    step_index: u32,
    step: AgentStep,
}

fn run_loop(
    registry: &ToolRegistry,
    allow: &ToolAllowlist,
    budget: &LoopBudget,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: LoopArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("loop args: {e}")))?;
    let driver = AgentLoop::new(budget.clone());
    if !parsed.step.wants_tools() {
        return Ok(json!({
            "results": [],
            "continue": false,
            "text": parsed.step.text,
            "step_index": parsed.step_index,
        }));
    }
    let results = driver
        .run_tools(parsed.step_index, &parsed.step, registry, allow, &EchoExec)
        .map_err(|c| (c, format!("{c}")))?;
    Ok(json!({
        "results": results,
        "continue": true,
        "text": parsed.step.text,
        "step_index": parsed.step_index,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    #[tokio::test]
    async fn run_echo_step() {
        let offer = ToolsLoopOffer::with_defaults().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: json!({
                    "step_index": 0,
                    "step": {
                        "tool_calls": [{
                            "id": "1",
                            "tool": "tools.echo",
                            "args": { "message": "hi" }
                        }]
                    }
                }),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["continue"], true);
                assert_eq!(result["results"][0]["ok"], true);
                assert_eq!(result["results"][0]["output"]["echo"]["message"], "hi");
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn policy_deny_and_budget() {
        let offer = ToolsLoopOffer::with_defaults().expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({ "tools": { "allow": ["tools.ping"], "max_steps": 1 } }),
            )
            .await
            .expect("bind");
        let deny = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({
                    "step_index": 0,
                    "step": {
                        "tool_calls": [{
                            "id": "1",
                            "tool": "tools.echo",
                            "args": { "message": "x" }
                        }]
                    }
                }),
                offer: None,
            })
            .await;
        assert!(matches!(
            deny,
            InvokeResp::Error {
                code: ErrorCode::PolicyDenied,
                ..
            }
        ));
        let exhausted = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: None,
                args: json!({
                    "step_index": 1,
                    "step": {
                        "tool_calls": [{
                            "id": "1",
                            "tool": "tools.ping",
                            "args": {}
                        }]
                    }
                }),
                offer: None,
            })
            .await;
        assert!(matches!(
            exhausted,
            InvokeResp::Error {
                code: ErrorCode::BudgetExhausted,
                ..
            }
        ));
    }
}
