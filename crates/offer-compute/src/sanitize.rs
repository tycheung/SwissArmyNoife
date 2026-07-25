//! Payload sanitization — never persist/forward secrets (`sak294-a`).

use serde_json::{Map, Value};

const SECRET_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "authorization",
    "password",
    "secret",
    "token",
    "access_token",
    "refresh_token",
    "private_key",
];

/// Recursively redact known secret field names to `[REDACTED]`.
#[must_use]
pub fn sanitize_payload(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(sanitize_map(map)),
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_payload).collect()),
        other => other,
    }
}

fn sanitize_map(map: Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (k, v) in map {
        if is_secret_key(&k) {
            out.insert(k, Value::String("[REDACTED]".into()));
        } else {
            out.insert(k, sanitize_payload(v));
        }
    }
    out
}

fn is_secret_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SECRET_KEYS
        .iter()
        .any(|s| lower == *s || lower.ends_with(&format!("_{s}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_nested_secrets() {
        let v = sanitize_payload(json!({
            "task": "x",
            "api_key": "sk-live",
            "nested": { "token": "abc", "ok": 1 }
        }));
        assert_eq!(v["api_key"], "[REDACTED]");
        assert_eq!(v["nested"]["token"], "[REDACTED]");
        assert_eq!(v["nested"]["ok"], 1);
        assert_eq!(v["task"], "x");
    }
}
