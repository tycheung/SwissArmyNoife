//! sak556-a: `shell_exec` uses the live sandbox backend (`SANDBOX_BACKEND=stub`).

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
async fn shell_exec_follows_stub_sandbox_backend() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "stub")
                .env("CAPACITY_PROBE", "fake");
        }))?)
        .await?;

    let shell = client
        .call_tool(CallToolRequestParam {
            name: "shell_exec".into(),
            arguments: Some(obj(json!({
                "argv": ["echo", "via-stub"],
                "cwd": "."
            }))),
        })
        .await?;
    let text = tool_text(&shell);
    assert!(
        text.contains("stub:echo") && text.contains("via-stub"),
        "shell_exec should use stub sandbox backend, got {text}"
    );
    client.cancel().await?;
    Ok(())
}
