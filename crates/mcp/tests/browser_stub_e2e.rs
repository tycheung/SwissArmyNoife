//! sak596: browser.session full-protocol stub via stdio MCP.

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
async fn browser_parity_stub_surface() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none")
                .env("BROWSER_BACKEND", "stub")
                .env("CAPACITY_PROBE", "fake");
        }))?)
        .await?;

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "browser.session",
                "ttl_secs": 120,
                "policy": {
                    "egress": {
                        "allow_hosts": ["example.com"],
                        "allow_principals": ["local"]
                    }
                }
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let nav = tool_text(
        &client
            .call_tool(CallToolRequestParam {
                name: "browser_navigate".into(),
                arguments: Some(obj(json!({
                    "binding_id": binding_id,
                    "url": "https://example.com/path"
                }))),
            })
            .await?,
    );
    assert!(nav.contains("stub"), "navigate={nav}");

    let snap = tool_text(
        &client
            .call_tool(CallToolRequestParam {
                name: "browser_snapshot".into(),
                arguments: Some(obj(json!({ "binding_id": binding_id }))),
            })
            .await?,
    );
    assert!(snap.contains("[e4]"), "snapshot refs={snap}");

    for (name, args) in [
        (
            "browser_click",
            json!({ "binding_id": binding_id, "ref": "e4" }),
        ),
        (
            "browser_type",
            json!({ "binding_id": binding_id, "ref": "e5", "text": "hi" }),
        ),
        (
            "browser_fill",
            json!({ "binding_id": binding_id, "ref": "e5", "text": "hi" }),
        ),
        (
            "browser_press_key",
            json!({ "binding_id": binding_id, "key": "Enter" }),
        ),
        (
            "browser_scroll",
            json!({ "binding_id": binding_id, "delta_y": 100 }),
        ),
        (
            "browser_select_option",
            json!({ "binding_id": binding_id, "ref": "e6", "value": "a" }),
        ),
        (
            "browser_drag",
            json!({ "binding_id": binding_id, "ref": "e4", "target_ref": "e5" }),
        ),
        (
            "browser_mouse_click_xy",
            json!({ "binding_id": binding_id, "x": 1.0, "y": 2.0 }),
        ),
        (
            "browser_take_screenshot",
            json!({ "binding_id": binding_id }),
        ),
        (
            "browser_highlight",
            json!({ "binding_id": binding_id, "ref": "e4" }),
        ),
        (
            "browser_get_bounding_box",
            json!({ "binding_id": binding_id, "ref": "e4" }),
        ),
        (
            "browser_tabs",
            json!({ "binding_id": binding_id, "action": "list" }),
        ),
        (
            "browser_lock",
            json!({ "binding_id": binding_id, "action": "lock" }),
        ),
        (
            "browser_lock",
            json!({ "binding_id": binding_id, "action": "unlock" }),
        ),
        (
            "browser_console",
            json!({ "binding_id": binding_id, "limit": 10 }),
        ),
        (
            "browser_network",
            json!({ "binding_id": binding_id, "limit": 10 }),
        ),
    ] {
        let text = tool_text(
            &client
                .call_tool(CallToolRequestParam {
                    name: name.into(),
                    arguments: Some(obj(args)),
                })
                .await?,
        );
        assert!(
            text.contains("ok") || text.contains("status"),
            "{name}={text}"
        );
    }

    let report = tool_text(
        &client
            .call_tool(CallToolRequestParam {
                name: "browser_failure_report".into(),
                arguments: Some(obj(json!({
                    "binding_id": binding_id,
                    "step": "checkout"
                }))),
            })
            .await?,
    );
    assert!(report.contains("screenshot_path"), "report={report}");
    assert!(report.contains("console"), "report={report}");

    let denied = tool_text(
        &client
            .call_tool(CallToolRequestParam {
                name: "browser_cdp".into(),
                arguments: Some(obj(json!({
                    "binding_id": binding_id,
                    "method": "Input.dispatchKeyEvent",
                    "params": {}
                }))),
            })
            .await?,
    );
    assert!(
        denied.contains("denied") || denied.contains("policy"),
        "cdp deny={denied}"
    );

    Ok(())
}
