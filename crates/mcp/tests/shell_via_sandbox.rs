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

#[tokio::test]
async fn shell_exec_charges_shell_risk_cap() -> Result<(), Box<dyn std::error::Error>> {
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

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "sandbox.exec",
                "ttl_secs": 120,
                "policy": { "risk_caps": { "max_shell_invocations": 1 } }
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let first = client
        .call_tool(CallToolRequestParam {
            name: "sandbox_exec".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "argv": ["echo", "one"],
                "cwd": "."
            }))),
        })
        .await?;
    let first_text = tool_text(&first);
    assert!(
        first_text.contains("\"status\":\"ok\""),
        "first sandbox_exec={first_text}"
    );

    let second = client
        .call_tool(CallToolRequestParam {
            name: "shell_exec".into(),
            arguments: Some(obj(json!({
                "argv": ["echo", "two"],
                "cwd": "."
            }))),
        })
        .await?;
    let second_text = tool_text(&second);
    assert!(
        second_text.contains("budget.exhausted") || second_text.contains("BudgetExhausted"),
        "shell_exec should exhaust shared shell cap, got {second_text}"
    );
    client.cancel().await?;
    Ok(())
}
