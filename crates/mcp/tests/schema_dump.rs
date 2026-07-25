//! Guard: tool input schemas must expose `type` on every property (Cursor discovery).

use mcp::tool_input_schemas;
use rmcp::{
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use tokio::process::Command;

/// Canonical MCP tool names the schema dump must cover (`sak111-b`).
const TOOL_NAMES: &[&str] = &[
    "ping",
    "broker_health",
    "catalog_list",
    "catalog_get",
    "provision",
    "bind",
    "unbind",
    "session_bind",
    "invoke",
    "llm_chat",
    "llm_preflight",
    "sandbox_exec",
    "fs_read",
    "memory_search",
    "research_fetch",
    "compute_work",
    "module_list",
    "module_invoke",
];

#[test]
fn tool_input_schemas_cover_canonical_tool_names() {
    let doc = tool_input_schemas();
    let tools = doc["tools"].as_object().expect("tools object");
    for name in TOOL_NAMES {
        assert!(tools.contains_key(*name), "missing schema for {name}");
    }
    let bind = &tools["bind"];
    assert!(
        bind["properties"].get("idempotency_key").is_some(),
        "bind missing idempotency_key: {bind}"
    );
    assert!(
        bind["properties"].get("policy_template").is_some(),
        "bind missing policy_template: {bind}"
    );
}

#[tokio::test]
async fn tool_schemas_have_typed_properties() -> Result<(), Box<dyn std::error::Error>> {
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

    let tools = client.list_all_tools().await?;
    assert!(
        tools.len() >= 20,
        "expected many tools, got {}",
        tools.len()
    );

    let mut missing = Vec::new();
    for t in &tools {
        let schema = serde_json::to_value(&t.input_schema)?;
        let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
            continue;
        };
        for (key, val) in props {
            if !has_type(val) {
                missing.push(format!("{}:{key} -> {val}", t.name));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "typeless JSON Schema properties (breaks Cursor MCP discovery):\n{}",
        missing.join("\n")
    );

    client.cancel().await?;
    Ok(())
}

fn has_type(v: &serde_json::Value) -> bool {
    if v.get("type").is_some() {
        return true;
    }
    if v.get("$ref").is_some() {
        return true;
    }
    // schemars Option<T> null arm
    if v.get("const").is_some() {
        return true;
    }
    if let Some(arr) = v
        .get("anyOf")
        .or_else(|| v.get("oneOf"))
        .and_then(|a| a.as_array())
    {
        return !arr.is_empty() && arr.iter().all(has_type);
    }
    false
}
