//! sak554-c: bind + `sandbox_exec` program deny is `PolicyDenied`, not spawn failure.

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
async fn sandbox_exec_program_deny_not_spawn() -> Result<(), Box<dyn std::error::Error>> {
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
            arguments: Some(obj(json!({
                "offer_id": "sandbox.exec",
                "ttl_secs": 120,
                "policy": { "sandbox": { "programs": ["git", "cargo"] } }
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let denied = client
        .call_tool(CallToolRequestParam {
            name: "sandbox_exec".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "argv": ["python"],
                "cwd": "."
            }))),
        })
        .await?;
    let text = tool_text(&denied);
    assert!(
        text.contains("policy.denied") && text.contains("sandbox.programs"),
        "program deny={text}"
    );
    assert!(
        !text.contains("spawn_failed") && !text.contains("provider.unreachable"),
        "deny must not look like spawn fail: {text}"
    );

    let spawn = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "sandbox.exec",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let spawn_id = serde_json::from_str::<Value>(&tool_text(&spawn))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();
    let missing = client
        .call_tool(CallToolRequestParam {
            name: "sandbox_exec".into(),
            arguments: Some(obj(json!({
                "binding_id": spawn_id,
                "argv": ["sak554-missing-binary"],
                "cwd": "."
            }))),
        })
        .await?;
    let spawn_text = tool_text(&missing);
    assert!(
        !spawn_text.contains("sandbox.programs"),
        "unrestricted missing binary must not be programs deny: {spawn_text}"
    );
    client.cancel().await?;
    Ok(())
}
