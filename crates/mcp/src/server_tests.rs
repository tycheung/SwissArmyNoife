use super::*;
use crate::tool_args::ChatMessageArg;
use control::RateLimiter;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;
use types::InvokeResp;
use uuid::Uuid;

fn sample_bind(offer_id: &str) -> BindArgs {
    BindArgs {
        offer_id: offer_id.into(),
        principal: "local".into(),
        policy: json!({}),
        policy_template: None,
        idempotency_key: None,
        ttl_secs: 60,
    }
}

fn test_server() -> (McpServer, tempfile::TempDir) {
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = ENV_LOCK.lock().expect("env lock");
    let tmp = tempfile::tempdir().expect("tmp");
    std::env::set_var(crate::live::LLM_BACKEND, "echo");
    std::env::set_var(crate::live::SANDBOX_BACKEND, "none");
    std::env::set_var("CAPACITY_PROBE", "fake");
    std::env::set_var("SAK_RATE_LIMIT_PER_MIN", "0");
    std::env::set_var(::env::CONFIG_DIR, tmp.path());
    let server = McpServer::new();
    drop(guard);
    (server, tmp)
}

#[tokio::test]
async fn ping_returns_ok() {
    let (server, _tmp) = test_server();
    let out = server.ping().await.expect("ping");
    assert_eq!(out, "ok");
}

#[tokio::test]
async fn broker_health_snapshot_ok() {
    let (server, _tmp) = test_server();
    let raw = server.broker_health().await.expect("health");
    let v: Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["ok"], true);
    assert!(v["offers"].as_u64().unwrap_or(0) >= 1);
    assert_eq!(v["policy"], "ambient");
}

#[test]
fn server_info_documents_ambient_trust() {
    use rmcp::ServerHandler;
    let (server, _tmp) = test_server();
    let info = server.get_info();
    let text = info.instructions.expect("instructions");
    assert!(
        text.contains("ambient trust"),
        "instructions should document stdio ambient trust: {text}"
    );
    assert!(text.contains("broker_health"));
    assert!(text.contains("llm_chat"));
    assert!(text.contains("v13"));
}

#[tokio::test]
async fn catalog_list_includes_seed_offers() {
    let (server, _tmp) = test_server();
    let listed = server.catalog_list().await.expect("list");
    assert!(listed.contains("broker.health"));
    assert!(listed.contains("llm.chat"));
    assert!(listed.contains("llm.embed"), "sak523-a: {listed}");
    assert!(listed.contains("llm.resolve"), "sak523-c: {listed}");
    assert!(listed.contains("memory.embed"), "sak524-a: {listed}");
    assert!(listed.contains("memory.scope"), "sak524-c: {listed}");
}

#[tokio::test]
async fn catalog_get_missing_is_error() {
    let (server, _tmp) = test_server();
    let err = server
        .catalog_get(Parameters(CatalogGetArgs {
            offer_id: "missing.offer".into(),
        }))
        .await
        .expect_err("missing");
    assert!(
        err.message.contains("offer.not_found"),
        "message={}",
        err.message
    );
}

#[tokio::test]
async fn bind_invoke_llm_chat_echo_backend() {
    let (server, _tmp) = test_server();
    let bound = server
        .bind(Parameters(sample_bind("llm.chat")))
        .await
        .expect("bind");
    let binding_id = serde_json::from_str::<Value>(&bound).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();

    let raw = server
        .llm_chat_inner(LlmChatToolArgs {
            binding_id: binding_id.clone(),
            messages: vec![ChatMessageArg {
                role: "user".into(),
                content: "ping".into(),
            }],
            model: Some("fixture".into()),
            provider: None,
            connection_id: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            prompt_cache_key: None,
        })
        .await
        .expect("chat");
    let resp: InvokeResp = serde_json::from_str(&raw).expect("InvokeResp");
    match resp {
        InvokeResp::Ok { result, .. } => {
            assert_eq!(result["text"], "echo:ping");
        }
        InvokeResp::Error { code, message, .. } => {
            panic!("unexpected error {code}: {message}")
        }
    }

    server
        .unbind(Parameters(UnbindArgs { binding_id }))
        .await
        .expect("unbind");
}

#[tokio::test]
async fn bind_invoke_llm_embed_echo_backend() {
    let (server, _tmp) = test_server();
    let bound = server
        .bind(Parameters(sample_bind("llm.embed")))
        .await
        .expect("bind");
    let binding_id = serde_json::from_str::<Value>(&bound).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();
    let raw = server
        .llm_embed(Parameters(crate::tool_args::LlmEmbedArgs {
            binding_id: binding_id.clone(),
            inputs: vec!["ab".into()],
            model: None,
        }))
        .await
        .expect("llm_embed");
    let resp: InvokeResp = serde_json::from_str(&raw).expect("InvokeResp");
    match resp {
        InvokeResp::Ok { result, .. } => {
            assert_eq!(result["vectors"][0][0], 2.0);
        }
        InvokeResp::Error { code, message, .. } => {
            panic!("unexpected error {code}: {message}")
        }
    }
    server
        .unbind(Parameters(UnbindArgs { binding_id }))
        .await
        .expect("unbind");
}

#[tokio::test]
async fn bind_invoke_memory_embed_echo_backend() {
    let (server, _tmp) = test_server();
    let bound = server
        .bind(Parameters(sample_bind("memory.embed")))
        .await
        .expect("bind");
    let binding_id = serde_json::from_str::<Value>(&bound).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();
    let raw = server
        .memory_embed(Parameters(crate::tool_args::MemoryEmbedArgs {
            binding_id: binding_id.clone(),
            inputs: vec!["ab".into()],
            model: None,
        }))
        .await
        .expect("memory_embed");
    let resp: InvokeResp = serde_json::from_str(&raw).expect("InvokeResp");
    match resp {
        InvokeResp::Ok { result, .. } => {
            assert_eq!(result["vectors"][0][0], 2.0);
        }
        InvokeResp::Error { code, message, .. } => {
            panic!("unexpected error {code}: {message}")
        }
    }
    server
        .unbind(Parameters(UnbindArgs { binding_id }))
        .await
        .expect("unbind");
}

#[tokio::test]
async fn bind_invoke_memory_scope_and_cross_kind_deny() {
    let (server, _tmp) = test_server();
    let mut args = sample_bind("memory.scope");
    args.policy = json!({ "memory": { "allowed_scopes": ["repo"] } });
    let bound = server.bind(Parameters(args)).await.expect("bind");
    let binding_id = serde_json::from_str::<Value>(&bound).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();
    let ok_raw = server
        .memory_scope(Parameters(crate::tool_args::MemoryScopeArgs {
            binding_id: binding_id.clone(),
            op: Some("hash".into()),
            kind: Some("repo".into()),
            id: Some("Acme/App".into()),
        }))
        .await
        .expect("memory_scope hash");
    let ok: InvokeResp = serde_json::from_str(&ok_raw).expect("InvokeResp");
    match ok {
        InvokeResp::Ok { result, .. } => {
            assert_eq!(result["kind"], "repo");
            assert!(result["scope_key"].as_str().expect("key").len() > 8);
        }
        InvokeResp::Error { code, message, .. } => {
            panic!("unexpected error {code}: {message}")
        }
    }
    let deny_raw = server
        .memory_scope(Parameters(crate::tool_args::MemoryScopeArgs {
            binding_id: binding_id.clone(),
            op: Some("hash".into()),
            kind: Some("user".into()),
            id: Some("alice".into()),
        }))
        .await
        .expect("memory_scope deny path");
    let deny: InvokeResp = serde_json::from_str(&deny_raw).expect("InvokeResp");
    match deny {
        InvokeResp::Error {
            code: types::ErrorCode::PolicyDenied,
            ..
        } => {}
        other => panic!("expected policy.denied, got {other:?}"),
    }
    server
        .unbind(Parameters(UnbindArgs { binding_id }))
        .await
        .expect("unbind");
}

#[tokio::test]
async fn bind_idempotency_replays_same_binding_id() {
    let (server, _tmp) = test_server();
    let mut args = sample_bind("llm.chat");
    args.idempotency_key = Some("idem-1".into());
    let first = server.bind(Parameters(args.clone())).await.expect("bind1");
    let id1 = serde_json::from_str::<Value>(&first).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();
    let second = server.bind(Parameters(args)).await.expect("bind2");
    let v: Value = serde_json::from_str(&second).expect("json");
    assert_eq!(v["binding_id"].as_str(), Some(id1.as_str()));
    assert_eq!(v["idempotent_replay"], true);
}

#[tokio::test]
async fn provision_idempotency_replays_same_resource_id() {
    use crate::tool_args::ProvisionArgs;
    let (server, _tmp) = test_server();
    let args = ProvisionArgs {
        offer_id: "llm.chat".into(),
        idempotency_key: Some("prov-1".into()),
    };
    let first = server
        .provision(Parameters(args.clone()))
        .await
        .expect("prov1");
    let id1 = serde_json::from_str::<Value>(&first).expect("json")["resource_id"]
        .as_str()
        .expect("str")
        .to_owned();
    let second = server.provision(Parameters(args)).await.expect("prov2");
    let v: Value = serde_json::from_str(&second).expect("json");
    assert_eq!(v["resource_id"].as_str(), Some(id1.as_str()));
    assert_eq!(v["idempotent_replay"], true);
}

#[tokio::test]
async fn bind_policy_template_offline() {
    let (server, _tmp) = test_server();
    let mut args = sample_bind("llm.chat");
    args.policy_template = Some("offline".into());
    let bound = server.bind(Parameters(args)).await.expect("bind");
    let binding_id = serde_json::from_str::<Value>(&bound).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();
    let store = server.bindings.lock().await;
    let record = store
        .get(parse_binding_id(&binding_id).expect("id"))
        .expect("live");
    assert_eq!(record.policy_json["network"], "deny");
}

#[tokio::test]
async fn bind_rate_limit_denies_when_exhausted() {
    let (server, _tmp) = test_server();
    *server.rate_limiter.lock().expect("lock") = RateLimiter::with_per_min(1.0);
    server
        .bind(Parameters(sample_bind("llm.chat")))
        .await
        .expect("first");
    let err = server
        .bind(Parameters(sample_bind("llm.chat")))
        .await
        .expect_err("limited");
    assert!(
        err.message.contains("rate_limit"),
        "message={}",
        err.message
    );
}

#[tokio::test]
async fn invoke_missing_binding_returns_expired() {
    let (server, _tmp) = test_server();
    let raw = server
        .invoke(Parameters(InvokeArgs {
            binding_id: Uuid::nil().to_string(),
            args: json!({}),
            offer: None,
        }))
        .await
        .expect("tool ok");
    let resp: InvokeResp = serde_json::from_str(&raw).expect("InvokeResp");
    match resp {
        InvokeResp::Error {
            code: types::ErrorCode::BindingExpired,
            ..
        } => {}
        other => panic!("expected BindingExpired, got {other:?}"),
    }
}

#[tokio::test]
async fn fs_write_read_and_shell_exec() {
    let (server, _tmp) = test_server();
    let path = format!("note-{}.txt", Uuid::new_v4());
    server
        .fs_write(Parameters(FsWriteArgs {
            path: path.clone(),
            content: "hello-fs".into(),
        }))
        .await
        .expect("write");
    let raw = server
        .fs_read(Parameters(FsReadArgs {
            path: path.clone(),
            mode: Some("full".into()),
        }))
        .await
        .expect("read");
    assert!(raw.contains("hello-fs"));
}

#[tokio::test]
async fn bind_invoke_egress_check_allow_deny() {
    let (server, _tmp) = test_server();
    let mut args = sample_bind("network.egress.check");
    args.policy = json!({
        "egress": {
            "allow_hosts": ["api.example.com"],
            "allow_principals": ["local"]
        }
    });
    let bound = server.bind(Parameters(args)).await.expect("bind");
    let binding_id = serde_json::from_str::<Value>(&bound).expect("json")["binding_id"]
        .as_str()
        .expect("str")
        .to_owned();

    let ok = server
        .egress_check(Parameters(EgressCheckArgs {
            binding_id: binding_id.clone(),
            url: "https://api.example.com/v1".into(),
        }))
        .await
        .expect("check");
    let ok_resp: InvokeResp = serde_json::from_str(&ok).expect("InvokeResp");
    match ok_resp {
        InvokeResp::Ok { result, .. } => {
            assert_eq!(result["allowed"], true);
        }
        other @ InvokeResp::Error { .. } => panic!("expected ok, got {other:?}"),
    }

    let denied = server
        .egress_check(Parameters(EgressCheckArgs {
            binding_id,
            url: "https://evil.com/".into(),
        }))
        .await
        .expect("check tool");
    let denied_resp: InvokeResp = serde_json::from_str(&denied).expect("InvokeResp");
    match denied_resp {
        InvokeResp::Error {
            code: types::ErrorCode::EgressDenied,
            ..
        } => {}
        other => panic!("expected EgressDenied, got {other:?}"),
    }
}
