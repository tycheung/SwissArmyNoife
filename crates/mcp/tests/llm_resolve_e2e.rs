//! sak576-b: MCP `llm_resolve` happy path (provider precedence JSON, no secrets).

use rmcp::{
    model::CallToolRequestParam,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Map, Value};
use tokio::process::Command;

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        other => panic!("expected object, got {other}"),
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

#[tokio::test]
async fn llm_resolve_ollama_hint() -> Result<(), Box<dyn std::error::Error>> {
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

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({ "offer_id": "llm.resolve", "ttl_secs": 120 }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();
    let raw = client
        .call_tool(CallToolRequestParam {
            name: "llm_resolve".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "provider": "ollama",
                "model": "llama3"
            }))),
        })
        .await?;
    let text = tool_text(&raw);
    assert!(!text.contains("sk-"), "no secrets: {text}");
    assert!(
        text.contains("\"status\":\"ok\"")
            && text.contains("ollama")
            && text.contains("local_ollama"),
        "llm_resolve={text}"
    );
    client.cancel().await?;
    Ok(())
}
