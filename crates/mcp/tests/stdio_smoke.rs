//! Stdio MCP smoke: resources + real sandbox / llm invoke.

use rmcp::{
    model::{CallToolRequestParam, ReadResourceRequestParam, ResourceContents},
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Map, Value};
use tokio::process::Command;

#[tokio::test]
async fn stdio_resources_and_invoke() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "llm.chat",
                "ttl_secs": 120,
                "policy": { "api_key": "sk-secret" }
            }))),
        })
        .await?;
    let bound_json: Value = serde_json::from_str(&tool_text(&bound))?;
    let binding_id = bound_json["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let resources = client.list_resources(Option::default()).await?;
    let uris: Vec<_> = resources.resources.iter().map(|r| r.uri.as_str()).collect();
    assert!(uris.contains(&"offer://llm.chat"), "uris={uris:?}");
    let binding_uri = format!("binding://{binding_id}");
    assert!(uris.contains(&binding_uri.as_str()), "uris={uris:?}");

    let read = client
        .read_resource(ReadResourceRequestParam { uri: binding_uri })
        .await?;
    let text = resource_text(&read);
    assert!(
        text.contains("[REDACTED]") && !text.contains("sk-secret"),
        "read={text}"
    );

    let invoked = client
        .call_tool(CallToolRequestParam {
            name: "llm_chat".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "messages": [{"role": "user", "content": "hi"}],
                "model": "fixture"
            }))),
        })
        .await?;
    let inv_text = tool_text(&invoked);
    assert!(
        inv_text.contains("\"status\":\"ok\"") && inv_text.contains("echo:hi"),
        "invoke={inv_text}"
    );

    let sandbox_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "sandbox.exec",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let sandbox_id = serde_json::from_str::<Value>(&tool_text(&sandbox_bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let argv = if cfg!(windows) {
        json!(["cmd", "/C", "echo hello"])
    } else {
        json!(["echo", "hello"])
    };
    let exec = client
        .call_tool(CallToolRequestParam {
            name: "sandbox_exec".into(),
            arguments: Some(obj(json!({
                "binding_id": sandbox_id,
                "argv": argv,
                "cwd": "."
            }))),
        })
        .await?;
    let exec_text = tool_text(&exec);
    assert!(
        exec_text.contains("\"status\":\"ok\"") && exec_text.to_lowercase().contains("hello"),
        "exec={exec_text}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_fs_and_shell_tools() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let listed = client.list_tools(Option::default()).await?;
    let names: Vec<_> = listed.tools.iter().map(|t| t.name.to_string()).collect();
    for need in ["fs_read", "fs_write", "fs_edit", "fs_grep", "shell_exec"] {
        assert!(
            names.iter().any(|n| n == need),
            "missing {need} in {names:?}"
        );
    }

    client
        .call_tool(CallToolRequestParam {
            name: "fs_write".into(),
            arguments: Some(obj(json!({
                "path": "smoke.txt",
                "content": "smoke-ok"
            }))),
        })
        .await?;
    let read = client
        .call_tool(CallToolRequestParam {
            name: "fs_read".into(),
            arguments: Some(obj(json!({
                "path": "smoke.txt",
                "mode": "full"
            }))),
        })
        .await?;
    assert!(
        tool_text(&read).contains("smoke-ok"),
        "fs_read={}",
        tool_text(&read)
    );

    let shell_argv = if cfg!(windows) {
        json!(["cmd", "/C", "echo shell-ok"])
    } else {
        json!(["echo", "shell-ok"])
    };
    let shell = client
        .call_tool(CallToolRequestParam {
            name: "shell_exec".into(),
            arguments: Some(obj(json!({
                "argv": shell_argv,
                "cwd": "."
            }))),
        })
        .await?;
    assert!(
        tool_text(&shell).to_lowercase().contains("shell-ok"),
        "shell_exec={}",
        tool_text(&shell)
    );

    let denied = client
        .call_tool(CallToolRequestParam {
            name: "fs_read".into(),
            arguments: Some(obj(json!({
                "path": "../escape.txt",
                "mode": "full"
            }))),
        })
        .await;
    match denied {
        Err(_) => {}
        Ok(res) => {
            let t = tool_text(&res);
            assert!(
                res.is_error == Some(true)
                    || t.contains("violation")
                    || t.contains("schema.invalid")
                    || t.contains("invalid"),
                "expected deny, got {t}"
            );
        }
    }

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_egress_check() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "network.egress.check",
                "ttl_secs": 120,
                "policy": {
                    "egress": {
                        "allow_hosts": ["api.example.com"],
                        "allow_principals": ["local"]
                    }
                }
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let ok = client
        .call_tool(CallToolRequestParam {
            name: "egress_check".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "url": "https://api.example.com/x"
            }))),
        })
        .await?;
    let ok_text = tool_text(&ok);
    assert!(
        ok_text.contains("\"allowed\":true") && ok_text.contains("api.example.com"),
        "egress={ok_text}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_egress_fetch_deny() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "network.egress.fetch",
                "ttl_secs": 120,
                "policy": {
                    "egress": {
                        "allow_hosts": ["api.example.com"],
                        "allow_principals": ["local"],
                        "max_response_bytes": 1024
                    }
                }
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let denied = client
        .call_tool(CallToolRequestParam {
            name: "egress_fetch".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "url": "https://evil.example/x"
            }))),
        })
        .await?;
    let text = tool_text(&denied);
    assert!(
        text.contains("egress.denied") || text.contains("EgressDenied"),
        "fetch deny={text}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_memory_index_search() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let idx_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "memory.index",
                "ttl_secs": 120,
                "policy": { "memory": { "backend": "exact" } }
            }))),
        })
        .await?;
    let idx_id = serde_json::from_str::<Value>(&tool_text(&idx_bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let search_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "memory.search",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let search_id = serde_json::from_str::<Value>(&tool_text(&search_bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let indexed = client
        .call_tool(CallToolRequestParam {
            name: "memory_index".into(),
            arguments: Some(obj(json!({
                "binding_id": idx_id,
                "documents": [
                    {"id": "1", "text": "swiss army memory index"},
                    {"id": "2", "text": "garden tomato tips"}
                ]
            }))),
        })
        .await?;
    let idx_text = tool_text(&indexed);
    assert!(
        idx_text.contains("\"rebuilt\":true") || idx_text.contains("\"status\":\"ok\""),
        "index={idx_text}"
    );

    let searched = client
        .call_tool(CallToolRequestParam {
            name: "memory_search".into(),
            arguments: Some(obj(json!({
                "binding_id": search_id,
                "query": "memory index",
                "k": 1
            }))),
        })
        .await?;
    let stext = tool_text(&searched);
    assert!(
        stext.contains("hits") && (stext.contains("memory") || stext.contains("\"status\":\"ok\"")),
        "search={stext}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_research_brief_and_fetch_deny() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let brief_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "research.brief",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let brief_id = serde_json::from_str::<Value>(&tool_text(&brief_bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let put = client
        .call_tool(CallToolRequestParam {
            name: "research_brief".into(),
            arguments: Some(obj(json!({
                "binding_id": brief_id,
                "action": "put",
                "title": "stdio-brief",
                "body": "notes from research",
                "source_url": "https://example.com"
            }))),
        })
        .await?;
    let put_text = tool_text(&put);
    assert!(
        put_text.contains("stdio-brief") && put_text.contains("\"status\":\"ok\""),
        "put={put_text}"
    );

    let fetch_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "research.fetch",
                "ttl_secs": 120,
                "policy": {
                    "egress": {
                        "allow_hosts": ["api.example.com"],
                        "allow_principals": ["local"],
                        "max_response_bytes": 1024
                    }
                }
            }))),
        })
        .await?;
    let fetch_id = serde_json::from_str::<Value>(&tool_text(&fetch_bound))?["binding_id"]
        .as_str()
        .expect("id")
        .to_owned();

    let denied = client
        .call_tool(CallToolRequestParam {
            name: "research_fetch".into(),
            arguments: Some(obj(json!({
                "binding_id": fetch_id,
                "url": "https://evil.example/x"
            }))),
        })
        .await?;
    let dtext = tool_text(&denied);
    assert!(
        dtext.contains("egress.denied") || dtext.contains("\"status\":\"error\""),
        "deny={dtext}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_module_list_invoke() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp");
    let tmp = tempfile::tempdir()?;
    let cfg = tmp.path().join("cfg");
    std::fs::create_dir_all(&cfg)?;
    std::env::set_var("CONFIG_DIR", &cfg);
    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/community.echo");
    module_registry::install_and_pin(&src, "path")?;
    std::env::remove_var("CONFIG_DIR");

    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", &cfg)
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;

    let listed = client.list_tools(Option::default()).await?;
    let names: Vec<_> = listed.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"module_list"), "{names:?}");
    assert!(names.contains(&"module_invoke"), "{names:?}");

    let mods = client
        .call_tool(CallToolRequestParam {
            name: "module_list".into(),
            arguments: Some(obj(json!({}))),
        })
        .await?;
    let mtext = tool_text(&mods);
    assert!(mtext.contains("community.echo"), "{mtext}");

    let inv = client
        .call_tool(CallToolRequestParam {
            name: "module_invoke".into(),
            arguments: Some(obj(json!({ "id": "community.echo", "a": 20, "b": 22 }))),
        })
        .await?;
    let itext = tool_text(&inv);
    assert!(
        itext.contains("\"sum\":42") || itext.contains("\"sum\": 42"),
        "{itext}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_capacity_probe_pressure_fit() -> Result<(), Box<dyn std::error::Error>> {
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

    let listed = client.list_tools(Option::default()).await?;
    let names: Vec<_> = listed.tools.iter().map(|t| t.name.as_ref()).collect();
    for need in ["capacity_probe", "capacity_pressure", "capacity_fit"] {
        assert!(names.contains(&need), "missing {need} in {names:?}");
    }

    let probe_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "capacity.probe",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let probe_id = serde_json::from_str::<Value>(&tool_text(&probe_bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let probed = client
        .call_tool(CallToolRequestParam {
            name: "capacity_probe".into(),
            arguments: Some(obj(json!({ "binding_id": probe_id }))),
        })
        .await?;
    let ptext = tool_text(&probed);
    assert!(
        ptext.contains("\"status\":\"ok\"") && ptext.contains("total_ram_mb"),
        "probe={ptext}"
    );

    let pressure_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "capacity.pressure",
                "ttl_secs": 120,
                "policy": { "capacity": { "max_ram_mb": 4096, "min_free_ram_mb": 512, "max_cpu_pct": 99.0 } }
            }))),
        })
        .await?;
    let pressure_id = serde_json::from_str::<Value>(&tool_text(&pressure_bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let pressured = client
        .call_tool(CallToolRequestParam {
            name: "capacity_pressure".into(),
            arguments: Some(obj(json!({ "binding_id": pressure_id }))),
        })
        .await?;
    let pressured_text = tool_text(&pressured);
    assert!(
        pressured_text.contains("\"status\":\"ok\"") && pressured_text.contains("admit"),
        "pressure={pressured_text}"
    );

    let fit_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "capacity.fit",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let fit_id = serde_json::from_str::<Value>(&tool_text(&fit_bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let fitted = client
        .call_tool(CallToolRequestParam {
            name: "capacity_fit".into(),
            arguments: Some(obj(json!({
                "binding_id": fit_id,
                "candidates": [
                    { "id": "small", "ram_mb": 1000 },
                    { "id": "big", "ram_mb": 5000 }
                ]
            }))),
        })
        .await?;
    let ftext = tool_text(&fitted);
    assert!(
        ftext.contains("\"status\":\"ok\"") && ftext.contains("ranks"),
        "fit={ftext}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn stdio_compute_node_work_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
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

    let listed = client.list_tools(Option::default()).await?;
    let names: Vec<_> = listed.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"compute_node"), "{names:?}");
    assert!(names.contains(&"compute_work"), "{names:?}");

    let node_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "compute.node",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let node_binding = serde_json::from_str::<Value>(&tool_text(&node_bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let work_bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "compute.work",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let work_binding = serde_json::from_str::<Value>(&tool_text(&work_bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let reg = client
        .call_tool(CallToolRequestParam {
            name: "compute_node".into(),
            arguments: Some(obj(json!({
                "binding_id": node_binding,
                "action": "register",
                "label": "stdio-worker",
                "caps": ["echo"]
            }))),
        })
        .await?;
    let rtext = tool_text(&reg);
    assert!(rtext.contains("\"status\":\"ok\""), "{rtext}");
    let node_id = serde_json::from_str::<Value>(&rtext)?["result"]["id"]
        .as_str()
        .expect("node id")
        .to_owned();

    let enq = client
        .call_tool(CallToolRequestParam {
            name: "compute_work".into(),
            arguments: Some(obj(json!({
                "binding_id": work_binding,
                "action": "enqueue",
                "kind": "echo",
                "payload": { "n": 1, "api_key": "sk-secret" }
            }))),
        })
        .await?;
    let etext = tool_text(&enq);
    assert!(etext.contains("[REDACTED]"), "{etext}");
    assert!(!etext.contains("sk-secret"), "{etext}");

    let claimed = client
        .call_tool(CallToolRequestParam {
            name: "compute_work".into(),
            arguments: Some(obj(json!({
                "binding_id": work_binding,
                "action": "claim",
                "node_id": node_id
            }))),
        })
        .await?;
    let ctext = tool_text(&claimed);
    let work_id = serde_json::from_str::<Value>(&ctext)?["result"]["id"]
        .as_str()
        .expect("work id")
        .to_owned();

    // sak430-d: requeue claimed work back to queued, then claim+complete
    let requeued = client
        .call_tool(CallToolRequestParam {
            name: "compute_work".into(),
            arguments: Some(obj(json!({
                "binding_id": work_binding,
                "action": "requeue",
                "work_id": work_id
            }))),
        })
        .await?;
    let requeued_text = tool_text(&requeued);
    assert!(
        requeued_text.contains("queued") || requeued_text.contains("requeue"),
        "{requeued_text}"
    );

    let claimed2 = client
        .call_tool(CallToolRequestParam {
            name: "compute_work".into(),
            arguments: Some(obj(json!({
                "binding_id": work_binding,
                "action": "claim",
                "node_id": node_id
            }))),
        })
        .await?;
    let ctext2 = tool_text(&claimed2);
    let work_id2 = serde_json::from_str::<Value>(&ctext2)?["result"]["id"]
        .as_str()
        .expect("work id after requeue")
        .to_owned();
    assert_eq!(work_id2, work_id);

    let done = client
        .call_tool(CallToolRequestParam {
            name: "compute_work".into(),
            arguments: Some(obj(json!({
                "binding_id": work_binding,
                "action": "complete",
                "node_id": node_id,
                "work_id": work_id2,
                "result": { "ok": true }
            }))),
        })
        .await?;
    let dtext = tool_text(&done);
    assert!(
        dtext.contains("\"status\":\"ok\"") && dtext.contains("completed"),
        "{dtext}"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_llm_preflight() -> Result<(), Box<dyn std::error::Error>> {
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

    let listed = client.list_tools(Option::default()).await?;
    let names: Vec<_> = listed.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"llm_preflight"), "{names:?}");

    let bound = client
        .call_tool(CallToolRequestParam {
            name: "bind".into(),
            arguments: Some(obj(json!({
                "offer_id": "llm.preflight",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let pre = client
        .call_tool(CallToolRequestParam {
            name: "llm_preflight".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "provider": "echo",
                "candidates": [
                    { "id": "tiny", "ram_mb": 512 },
                    { "id": "huge", "ram_mb": 500_000 }
                ]
            }))),
        })
        .await?;
    let text = tool_text(&pre);
    assert!(text.contains("\"status\":\"ok\""), "{text}");
    assert!(
        text.contains("\"reachable\":true") || text.contains("\"reachable\": true"),
        "{text}"
    );
    assert!(text.contains("fit_ranks"), "{text}");
    assert!(text.contains("tiny"), "{text}");

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stdio_llm_telemetry() -> Result<(), Box<dyn std::error::Error>> {
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
                "offer_id": "llm.telemetry",
                "ttl_secs": 120
            }))),
        })
        .await?;
    let binding_id = serde_json::from_str::<Value>(&tool_text(&bound))?["binding_id"]
        .as_str()
        .expect("binding_id")
        .to_owned();

    let recorded = client
        .call_tool(CallToolRequestParam {
            name: "llm_telemetry".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "action": "record",
                "record": {
                    "provider": "echo",
                    "binding_source": "local",
                    "prompt_tokens": 1,
                    "completion_tokens": 2,
                    "model": "fixture"
                }
            }))),
        })
        .await?;
    let rtext = tool_text(&recorded);
    assert!(rtext.contains("\"status\":\"ok\""), "{rtext}");

    let listed = client
        .call_tool(CallToolRequestParam {
            name: "llm_telemetry".into(),
            arguments: Some(obj(json!({
                "binding_id": binding_id,
                "action": "list",
                "limit": 5
            }))),
        })
        .await?;
    let ltext = tool_text(&listed);
    assert!(
        ltext.contains("echo") && ltext.contains("prompt_tokens"),
        "{ltext}"
    );

    client.cancel().await?;
    Ok(())
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

fn resource_text(result: &rmcp::model::ReadResourceResult) -> String {
    result
        .contents
        .iter()
        .filter_map(|c| match c {
            ResourceContents::TextResourceContents { text, .. } => Some(text.as_str()),
            ResourceContents::BlobResourceContents { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
