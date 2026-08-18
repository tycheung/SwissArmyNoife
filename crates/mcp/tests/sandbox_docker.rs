//! sak550-c: `SANDBOX_BACKEND=docker` MCP smoke; skip when Docker is unavailable.

use rmcp::{
    model::CallToolRequestParam,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Map, Value};
use std::process::Command as StdCommand;
use tokio::process::Command;

fn docker_available() -> bool {
    StdCommand::new("docker")
        .args(["info"])
        .output()
        .is_ok_and(|o| o.status.success())
}

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
async fn sandbox_exec_docker_or_skip() -> Result<(), Box<dyn std::error::Error>> {
    if !docker_available() {
        eprintln!("sak550-c: skip live docker MCP test (docker unavailable)");
        return Ok(());
    }
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "docker")
                .env("CAPACITY_PROBE", "fake");
        }))?)
        .await?;

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "sandbox.exec",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let exec = client
        .call_tool(CallToolRequestParam {
            name: "sandbox_exec".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "argv": ["echo", "sak550-docker"],
                "cwd": "."
            }))),
        })
        .await?;
    let text = tool_text(&exec);
    assert!(
        text.contains("\"status\":\"ok\"") && text.contains("sak550-docker"),
        "docker sandbox_exec={text}"
    );
    client.cancel().await?;
    Ok(())
}
