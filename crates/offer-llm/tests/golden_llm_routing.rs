//! Load Nimbusware llm-routing golden fixtures (`sak142-a` / `sak142-b` / `sak142-c`).

use offer_llm::{resolve, ConnectionRef, EchoChatProvider, ResolveHint};
use provider_core::{ChatMessage, ChatRequest, ChatRole, LlmProvider};
use serde_json::Value;
use types::{load_offer_fixture, ErrorCode};

const ECHO_FIXTURES: &[&str] = &[
    "llm-routing/echo-chat.json",
    "llm-routing/echo-chat-system.json",
];

const ROUTING_FIXTURES: &[&str] = &[
    "llm-routing/vault-missing.json",
    "llm-routing/provider-openai-hint.json",
];

fn load(name: &str) -> Value {
    load_offer_fixture(env!("CARGO_MANIFEST_DIR"), name).expect("fixture")
}

fn messages_from(fix: &Value) -> Vec<ChatMessage> {
    fix["request"]["args"]["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .map(|m| ChatMessage {
            role: match m["role"].as_str().expect("role") {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                "assistant" => ChatRole::Assistant,
                "tool" => ChatRole::Tool,
                other => panic!("unknown role {other}"),
            },
            content: m["content"].as_str().expect("content").into(),
        })
        .collect()
}

fn resolve_hint_from(fix: &Value) -> ResolveHint {
    let args = &fix["request"]["args"];
    ResolveHint {
        connection_id: args
            .get("connection_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        provider: args
            .get("provider")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        model: args
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    }
}

fn openai_catalog() -> Vec<ConnectionRef> {
    vec![
        ConnectionRef {
            connection_id: "conn-openai-default".into(),
            provider: "openai".into(),
            label: "default".into(),
        },
        ConnectionRef {
            connection_id: "conn-openai-other".into(),
            provider: "openai".into(),
            label: "staging".into(),
        },
    ]
}

#[test]
fn llm_routing_fixtures_parse() {
    for name in ECHO_FIXTURES {
        let fix = load(name);
        assert_eq!(fix["schema"], "sak.fixture.offer/v0");
        assert_eq!(fix["offer"], "llm.chat");
        assert_eq!(fix["expect"]["status"], "ok");
        assert_eq!(
            fix["expect"]["routing"]["provider"].as_str().unwrap(),
            "echo"
        );
    }

    for name in ROUTING_FIXTURES {
        let fix = load(name);
        assert_eq!(fix["schema"], "sak.fixture.offer/v0");
        assert_eq!(fix["offer"], "llm.chat");
        assert!(fix["expect"]["routing"].is_object());
    }
}

#[tokio::test]
async fn llm_routing_fixtures_match_echo_provider() {
    for name in ECHO_FIXTURES {
        let fix = load(name);
        let model = fix["request"]["args"]["model"]
            .as_str()
            .expect("model")
            .to_string();
        let resp = EchoChatProvider
            .chat(ChatRequest {
                model,
                messages: messages_from(&fix),
                max_tokens: None,
                temperature: None,
                prompt_cache_key: None,
            })
            .await
            .expect("echo chat");
        assert_eq!(
            resp.content,
            fix["expect"]["result"]["text"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn llm_routing_vault_missing_fixture_matches_resolve() {
    let fix = load("llm-routing/vault-missing.json");
    assert_eq!(fix["expect"]["status"], "error");
    assert_eq!(
        fix["expect"]["error"]["code"].as_str().unwrap(),
        "vault.missing"
    );

    let hint = resolve_hint_from(&fix);
    let err = resolve(&hint, &openai_catalog()).expect_err("missing connection");
    assert_eq!(err.to_error_code(), ErrorCode::VaultMissing);
}

#[test]
fn llm_routing_provider_openai_fixture_matches_resolve() {
    let fix = load("llm-routing/provider-openai-hint.json");
    assert_eq!(fix["expect"]["status"], "ok");

    let hint = resolve_hint_from(&fix);
    let got = resolve(&hint, &openai_catalog()).expect("resolve openai");
    let routing = &fix["expect"]["routing"];
    assert_eq!(got.provider, routing["provider"].as_str().unwrap());
    assert_eq!(
        got.connection_id.as_deref(),
        routing["connection_id"].as_str()
    );
    assert_eq!(got.model, routing["model"].as_str().unwrap());
    assert_eq!(
        serde_json::to_value(got.binding_source).unwrap(),
        routing["binding_source"]
    );
}
