//! MCP conformance fixture client skeleton (`sak106-a`).
//!
//! Loads JSON step scripts from `fixtures/mcp/conformance/` and drives stdio MCP.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use rmcp::{
    model::CallToolRequestParam,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Step {
    tool: String,
    #[serde(default)]
    arguments: Option<Value>,
    #[serde(default)]
    expect_contains: Vec<String>,
}

#[tokio::test]
async fn conformance_ping_catalog_fixture() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture("ping-catalog.json", "ping-catalog-smoke").await
}

#[tokio::test]
async fn conformance_bind_ping_pack_fixture() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture("bind-ping-pack.json", "bind-ping-pack").await
}

#[tokio::test]
async fn conformance_invoke_deny_fixture() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture("invoke-deny.json", "invoke-deny").await
}

#[tokio::test]
async fn conformance_memory_search_empty_fixture() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture("memory-search-empty.json", "memory-search-empty").await
}

#[tokio::test]
async fn conformance_llm_chat_echo_fixture() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture("llm-chat-echo.json", "llm-chat-echo").await
}

async fn run_fixture(file: &str, expect_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_path = fixture_dir().join(file);
    let fixture: Fixture = serde_json::from_str(&fs::read_to_string(fixture_path)?)?;
    assert_eq!(fixture.name, expect_name);

    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none")
                .env("CAPACITY_PROBE", "fake");
        }))?)
        .await?;

    let mut bindings: HashMap<String, String> = HashMap::new();

    for step in &fixture.steps {
        let args = step
            .arguments
            .clone()
            .and_then(|v| match v {
                Value::Object(m) => Some(m),
                _ => None,
            })
            .map(|mut m| {
                if let Some(Value::String(raw)) = m.get("binding_id") {
                    if let Some(resolved) = resolve_binding_placeholder(raw, &bindings) {
                        m.insert("binding_id".into(), Value::String(resolved));
                    }
                }
                m
            });
        let result = client
            .call_tool(CallToolRequestParam {
                name: step.tool.clone().into(),
                arguments: args,
            })
            .await?;
        let text = tool_text(&result);
        if step.tool == "session_bind" {
            capture_session_bindings(&text, &mut bindings);
        }
        for needle in &step.expect_contains {
            assert!(
                text.contains(needle),
                "fixture={} tool={} missing {needle:?} in {text}",
                fixture.name,
                step.tool
            );
        }
    }

    let _ = client.cancel().await;
    Ok(())
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/mcp/conformance")
}

fn resolve_binding_placeholder(raw: &str, bindings: &HashMap<String, String>) -> Option<String> {
    raw.strip_prefix('$')
        .and_then(|offer_id| bindings.get(offer_id).cloned())
}

fn capture_session_bindings(text: &str, bindings: &mut HashMap<String, String>) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let Some(rows) = value.get("bindings").and_then(Value::as_array) else {
        return;
    };
    for row in rows {
        let Some(offer_id) = row.get("offer_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(binding_id) = row.get("binding_id").and_then(Value::as_str) else {
            continue;
        };
        bindings.insert(offer_id.to_owned(), binding_id.to_owned());
    }
}

fn tool_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("")
}
