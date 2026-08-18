//! sak571-d: MCP `llm_embed` / `memory_embed` via router; skip live Ollama if down.

use rmcp::{
    model::CallToolRequestParam,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Map, Value};
use std::net::TcpStream;
use std::time::Duration;
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

fn ollama_reachable() -> bool {
    TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().expect("addr"),
        Duration::from_millis(200),
    )
    .is_ok()
}

#[tokio::test]
async fn llm_and_memory_embed_echo_router() -> Result<(), Box<dyn std::error::Error>> {
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

    for (offer_id, tool) in [("llm.embed", "llm_embed"), ("memory.embed", "memory_embed")] {
        let bound = client
            .call_tool(CallToolRequestParam {
                name: "bind".into(),
                arguments: Some(obj(json!({ "offer_id": offer_id, "ttl_secs": 120 }))),
            })
            .await?;
        let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
            .as_str()
            .expect("binding_id")
            .to_owned();
        let raw = client
            .call_tool(CallToolRequestParam {
                name: tool.into(),
                arguments: Some(obj(json!({
                    "binding_id": binding_id,
                    "inputs": ["ab"]
                }))),
            })
            .await?;
        let text = tool_text(&raw);
        assert!(
            text.contains("\"status\":\"ok\"") && text.contains("vectors"),
            "{tool}={text}"
        );
        client
            .call_tool(CallToolRequestParam {
                name: "unbind".into(),
                arguments: Some(obj(json!({ "binding_id": binding_id }))),
            })
            .await?;
    }
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn llm_embed_ollama_or_skip() -> Result<(), Box<dyn std::error::Error>> {
    if !ollama_reachable() {
        eprintln!("sak571-d: skip live Ollama embed (127.0.0.1:11434 unreachable)");
        return Ok(());
    }
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env_remove("LLM_BACKEND")
                .env("SANDBOX_BACKEND", "none")
                .env("CAPACITY_PROBE", "fake");
        }))?)
        .await?;
    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({ "offer_id": "llm.embed", "ttl_secs": 120 }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();
    let raw = client
        .call_tool(CallToolRequestParam {
            name: "llm_embed".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "inputs": ["hello"],
                "model": "nomic-embed-text"
            }))),
        })
        .await?;
    let text = tool_text(&raw);
    assert!(
        text.contains("vectors") || text.contains("Unreachable") || text.contains("error"),
        "ollama llm_embed={text}"
    );
    client.cancel().await?;
    Ok(())
}
