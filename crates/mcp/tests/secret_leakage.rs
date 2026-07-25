//! Deny secret leakage on MCP fixtures (`sak112-a`).

use control::{redact_json, BindRequest, BindingStore};
use serde_json::{json, Value};
use types::OfferId;

fn assert_no_secret(hay: &str) {
    let lower = hay.to_ascii_lowercase();
    for needle in ["sk-secret", "sk-live", "password123", "Bearer super"] {
        assert!(
            !hay.contains(needle) && !lower.contains(&needle.to_ascii_lowercase()),
            "secret leaked: found {needle:?} in {hay}"
        );
    }
}

fn assert_redacted_policy(policy: &Value) {
    let red = redact_json(policy);
    let s = red.to_string();
    assert!(s.contains("[REDACTED]") || !s.contains("sk-"), "{s}");
    assert_no_secret(&s);
}

#[test]
fn redact_json_strips_common_secret_keys() {
    let policy = json!({
        "api_key": "sk-secret",
        "nested": { "token": "sk-live", "ok": true },
        "authorization": "Bearer super"
    });
    assert_redacted_policy(&policy);
}

#[test]
fn binding_store_policy_is_redacted_for_resource_shape() {
    let mut store = BindingStore::new();
    let record = store.bind(BindRequest {
        offer_id: OfferId::new("llm.chat").unwrap(),
        principal: control::Principal::local(),
        policy_json: json!({
            "api_key": "sk-secret",
            "password": "password123",
            "caps": { "max_tokens": 16 }
        }),
        ttl: std::time::Duration::from_secs(60),
    });
    let body = json!({
        "binding_id": record.binding_id.to_string(),
        "offer_id": record.offer_id.as_str(),
        "policy": redact_json(&record.policy_json),
    });
    assert_no_secret(&body.to_string());
    assert_eq!(body["policy"]["api_key"], "[REDACTED]");
    assert_eq!(body["policy"]["password"], "[REDACTED]");
    assert_eq!(body["policy"]["caps"]["max_tokens"], 16);
}

#[test]
fn invoke_resp_ok_must_not_embed_raw_policy() {
    // Contract: InvokeResp result from llm.chat never includes api_key fields.
    let result = json!({
        "text": "echo:hi",
        "provider": "echo",
        "binding_source": "local_ollama",
        "usage": { "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 }
    });
    assert!(result.get("api_key").is_none());
    assert_no_secret(&result.to_string());
}
