//! Full MCP tool matrix against rebuilt stdio `mcp` (echo / none / fake backends).

use std::collections::BTreeSet;

use rmcp::{
    model::CallToolRequestParam,
    service::RunningService,
    transport::{ConfigureCommandExt, TokioChildProcess},
    RoleClient, ServiceExt,
};
use serde_json::{json, Map, Value};
use tokio::process::Command;

const EXPECTED_TOOLS: &[&str] = &[
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
    "llm_embed",
    "llm_preflight",
    "ollama_manage",
    "llm_telemetry",
    "sandbox_exec",
    "sandbox_jail",
    "fs_read",
    "fs_write",
    "fs_edit",
    "fs_grep",
    "shell_exec",
    "egress_check",
    "egress_fetch",
    "memory_index",
    "memory_embed",
    "memory_scope",
    "memory_search",
    "tools_registry",
    "tools_loop",
    "research_fetch",
    "research_brief",
    "module_list",
    "module_invoke",
    "capacity_probe",
    "capacity_pressure",
    "capacity_fit",
    "compute_node",
    "compute_work",
];

type Client = RunningService<RoleClient, ()>;

#[tokio::test]
#[allow(clippy::too_many_lines, clippy::await_holding_lock)]
async fn all_mcp_tools_happy_or_structured() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let cfg = tmp.path().join("cfg");
    std::fs::create_dir_all(&cfg)?;
    std::env::set_var("CONFIG_DIR", &cfg);
    let echo_mod =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/community.echo");
    module_registry::install_and_pin(&echo_mod, "path")?;
    std::env::remove_var("CONFIG_DIR");

    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", &cfg)
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none")
                .env("CAPACITY_PROBE", "fake");
        }))?)
        .await?;

    let listed = client.list_tools(Option::default()).await?;
    let names: BTreeSet<_> = listed.tools.iter().map(|t| t.name.to_string()).collect();
    let expected: BTreeSet<_> = EXPECTED_TOOLS.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        names,
        expected,
        "tool catalog mismatch\nmissing={:?}\nextra={:?}",
        expected.difference(&names).collect::<Vec<_>>(),
        names.difference(&expected).collect::<Vec<_>>()
    );

    assert_contains(&call(&client, "ping", json!({})).await?, "ok");
    assert_contains(&call(&client, "broker_health", json!({})).await?, "ok");
    assert_contains(&call(&client, "catalog_list", json!({})).await?, "llm.chat");
    assert_contains(
        &call(&client, "catalog_list", json!({})).await?,
        "llm.embed",
    );
    assert_contains(
        &call(&client, "catalog_get", json!({"offer_id": "llm.chat"})).await?,
        "llm.chat",
    );
    assert_contains(
        &call(
            &client,
            "provision",
            json!({"offer_id": "llm.chat", "idempotency_key": "matrix-prov"}),
        )
        .await?,
        "resource",
    );

    let llm_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "llm.chat", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "llm_chat",
            json!({
                "binding_id": llm_id,
                "messages": [{"role": "user", "content": "matrix"}],
                "model": "fixture"
            }),
        )
        .await?,
        "echo:matrix",
    );
    assert_contains(
        &call(
            &client,
            "invoke",
            json!({
                "binding_id": llm_id,
                "args": {
                    "messages": [{"role": "user", "content": "via-invoke"}],
                    "model": "fixture"
                }
            }),
        )
        .await?,
        "echo:via-invoke",
    );
    assert_contains(
        &call(&client, "unbind", json!({"binding_id": llm_id})).await?,
        "unbound",
    );

    let embed_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "llm.embed", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "llm_embed",
            json!({ "binding_id": embed_id, "inputs": ["ab"] }),
        )
        .await?,
        "vectors",
    );

    let session = call(
        &client,
        "session_bind",
        json!({
            "offer_ids": ["llm.chat", "llm.preflight", "llm.telemetry", "llm.ollama.manage"],
            "ttl_secs": 300
        }),
    )
    .await?;
    let session_json: Value = serde_json::from_str(&session)?;
    let mut by_offer = Map::new();
    for row in session_json["bindings"].as_array().expect("bindings") {
        by_offer.insert(
            row["offer_id"].as_str().unwrap().to_owned(),
            Value::String(row["binding_id"].as_str().unwrap().to_owned()),
        );
    }
    let preflight_id = by_offer["llm.preflight"].as_str().unwrap();
    let telem_id = by_offer["llm.telemetry"].as_str().unwrap();
    let ollama_id = by_offer["llm.ollama.manage"].as_str().unwrap();

    assert_contains(
        &call(
            &client,
            "llm_preflight",
            json!({
                "binding_id": preflight_id,
                "provider": "echo",
                "candidates": [{"id": "tiny", "ram_mb": 512}]
            }),
        )
        .await?,
        "status",
    );
    assert_contains(
        &call(
            &client,
            "llm_telemetry",
            json!({"binding_id": telem_id, "action": "list", "limit": 5}),
        )
        .await?,
        "status",
    );
    let ollama = call(
        &client,
        "ollama_manage",
        json!({"binding_id": ollama_id, "action": "list"}),
    )
    .await?;
    assert!(
        ollama.contains("status") || ollama.contains("error") || ollama.contains("models"),
        "ollama_manage={ollama}"
    );

    let sandbox_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "sandbox.exec", "ttl_secs": 300}),
        )
        .await?,
    )?;
    let argv = if cfg!(windows) {
        json!(["cmd", "/C", "echo matrix-sb"])
    } else {
        json!(["echo", "matrix-sb"])
    };
    assert_contains(
        &call(
            &client,
            "sandbox_exec",
            json!({"binding_id": sandbox_id, "argv": argv, "cwd": "."}),
        )
        .await?,
        "ok",
    );

    let jail_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "sandbox.jail", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "sandbox_jail",
            json!({ "binding_id": jail_id, "op": "probe", "path": "../secret" }),
        )
        .await?,
        "inside",
    );

    call(
        &client,
        "fs_write",
        json!({"path": "matrix.txt", "content": "alpha-beta"}),
    )
    .await?;
    assert_contains(
        &call(
            &client,
            "fs_read",
            json!({"path": "matrix.txt", "mode": "full"}),
        )
        .await?,
        "alpha-beta",
    );
    call(
        &client,
        "fs_edit",
        json!({"path": "matrix.txt", "old": "beta", "new": "gamma"}),
    )
    .await?;
    assert_contains(
        &call(
            &client,
            "fs_grep",
            json!({"path": "matrix.txt", "pattern": "gamma"}),
        )
        .await?,
        "gamma",
    );
    let shell_argv = if cfg!(windows) {
        json!(["cmd", "/C", "echo matrix-shell"])
    } else {
        json!(["echo", "matrix-shell"])
    };
    assert_contains(
        &call(
            &client,
            "shell_exec",
            json!({"argv": shell_argv, "cwd": "."}),
        )
        .await?
        .to_lowercase(),
        "matrix-shell",
    );

    let egress_check_id = binding_id(
        &call(
            &client,
            "bind",
            json!({
                "offer_id": "network.egress.check",
                "ttl_secs": 300,
                "policy": {
                    "egress": {
                        "allow_hosts": ["example.com"],
                        "allow_principals": ["local"]
                    }
                }
            }),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "egress_check",
            json!({"binding_id": egress_check_id, "url": "https://example.com/"}),
        )
        .await?,
        "ok",
    );
    let egress_fetch_id = binding_id(
        &call(
            &client,
            "bind",
            json!({
                "offer_id": "network.egress.fetch",
                "ttl_secs": 300,
                "policy": {
                    "egress": {
                        "allow_hosts": [],
                        "allow_principals": ["local"],
                        "max_response_bytes": 1024
                    }
                }
            }),
        )
        .await?,
    )?;
    let fetch_deny = call(
        &client,
        "egress_fetch",
        json!({"binding_id": egress_fetch_id, "url": "https://blocked.example/"}),
    )
    .await?;
    assert!(
        fetch_deny.contains("denied")
            || fetch_deny.contains("error")
            || fetch_deny.contains("policy")
            || fetch_deny.contains("status"),
        "egress_fetch deny={fetch_deny}"
    );

    let mem_id = binding_id(
        &call(
            &client,
            "bind",
            json!({
                "offer_id": "memory.index",
                "ttl_secs": 300,
                "policy": { "memory": { "backend": "exact" } }
            }),
        )
        .await?,
    )?;
    let mem_search_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "memory.search", "ttl_secs": 300}),
        )
        .await?,
    )?;
    call(
        &client,
        "memory_index",
        json!({
            "binding_id": mem_id,
            "documents": [{"id": "d1", "text": "matrix memory doc"}]
        }),
    )
    .await?;
    assert_contains(
        &call(
            &client,
            "memory_search",
            json!({"binding_id": mem_search_id, "query": "matrix", "limit": 5}),
        )
        .await?,
        "status",
    );

    let mem_embed_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "memory.embed", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "memory_embed",
            json!({ "binding_id": mem_embed_id, "inputs": ["ab"] }),
        )
        .await?,
        "vectors",
    );

    let mem_scope_id = binding_id(
        &call(
            &client,
            "bind",
            json!({
                "offer_id": "memory.scope",
                "ttl_secs": 300,
                "policy": { "memory": { "allowed_scopes": ["repo", "user", "org"] } }
            }),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "memory_scope",
            json!({
                "binding_id": mem_scope_id,
                "op": "hash",
                "kind": "repo",
                "id": "matrix/app"
            }),
        )
        .await?,
        "scope_key",
    );

    let tools_reg_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "tools.registry", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "tools_registry",
            json!({ "binding_id": tools_reg_id, "op": "list" }),
        )
        .await?,
        "tools",
    );

    let tools_loop_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "tools.loop", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "tools_loop",
            json!({
                "binding_id": tools_loop_id,
                "step_index": 0,
                "step": {
                    "tool_calls": [{
                        "id": "1",
                        "tool": "tools.echo",
                        "args": { "message": "matrix" }
                    }]
                }
            }),
        )
        .await?,
        "results",
    );

    let research_fetch_id = binding_id(
        &call(
            &client,
            "bind",
            json!({
                "offer_id": "research.fetch",
                "ttl_secs": 300,
                "policy": {
                    "egress": {
                        "allow_hosts": [],
                        "allow_principals": ["local"],
                        "max_response_bytes": 1024
                    }
                }
            }),
        )
        .await?,
    )?;
    let rf = call(
        &client,
        "research_fetch",
        json!({"binding_id": research_fetch_id, "url": "https://blocked.example/"}),
    )
    .await?;
    assert!(
        rf.contains("denied")
            || rf.contains("error")
            || rf.contains("policy")
            || rf.contains("status"),
        "research_fetch={rf}"
    );
    let brief_id = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "research.brief", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "research_brief",
            json!({
                "binding_id": brief_id,
                "action": "put",
                "title": "matrix",
                "body": "brief body"
            }),
        )
        .await?,
        "status",
    );

    let modules = call(&client, "module_list", json!({})).await?;
    assert!(modules.contains("community.echo"), "module_list={modules}");
    let minv = call(
        &client,
        "module_invoke",
        json!({"id": "community.echo", "a": 2, "b": 3}),
    )
    .await?;
    assert!(
        minv.contains("\"sum\":5") || minv.contains("\"sum\": 5"),
        "module_invoke={minv}"
    );

    let cap_probe = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "capacity.probe", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(&client, "capacity_probe", json!({"binding_id": cap_probe})).await?,
        "ok",
    );
    let cap_pressure = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "capacity.pressure", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "capacity_pressure",
            json!({"binding_id": cap_pressure}),
        )
        .await?,
        "status",
    );
    let cap_fit = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "capacity.fit", "ttl_secs": 300}),
        )
        .await?,
    )?;
    assert_contains(
        &call(
            &client,
            "capacity_fit",
            json!({
                "binding_id": cap_fit,
                "candidates": [{"id": "tiny", "ram_mb": 256}]
            }),
        )
        .await?,
        "status",
    );

    let node_bind = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "compute.node", "ttl_secs": 300}),
        )
        .await?,
    )?;
    let work_bind = binding_id(
        &call(
            &client,
            "bind",
            json!({"offer_id": "compute.work", "ttl_secs": 300}),
        )
        .await?,
    )?;
    let reg = call(
        &client,
        "compute_node",
        json!({
            "binding_id": node_bind,
            "action": "register",
            "label": "matrix-node",
            "caps": ["echo"]
        }),
    )
    .await?;
    assert_contains(&reg, "ok");
    let _node_id = serde_json::from_str::<Value>(&reg)?["result"]["id"]
        .as_str()
        .expect("node id")
        .to_owned();
    let enq = call(
        &client,
        "compute_work",
        json!({
            "binding_id": work_bind,
            "action": "enqueue",
            "kind": "echo",
            "payload": {"n": 1}
        }),
    )
    .await?;
    assert_contains(&enq, "ok");

    let deny = call(
        &client,
        "invoke",
        json!({
            "binding_id": "00000000-0000-0000-0000-000000000000",
            "args": {}
        }),
    )
    .await;
    match deny {
        Err(_) => {}
        Ok(t) => assert!(
            t.contains("expired") || t.contains("error") || t.contains("invalid"),
            "expected deny got {t}"
        ),
    }

    client.cancel().await?;
    Ok(())
}

async fn call(
    client: &Client,
    name: &str,
    args: Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let result = client
        .call_tool(CallToolRequestParam {
            name: name.to_owned().into(),
            arguments: Some(obj(args)),
        })
        .await?;
    Ok(tool_text(&result))
}

fn binding_id(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let v: Value = serde_json::from_str(text)?;
    Ok(v["binding_id"]
        .as_str()
        .ok_or("missing binding_id")?
        .to_owned())
}

fn assert_contains(hay: &str, needle: &str) {
    assert!(hay.contains(needle), "missing {needle:?} in {hay}");
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
