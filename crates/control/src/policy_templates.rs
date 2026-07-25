//! Built-in policy templates (`sak065-a`).

use serde_json::{json, Value};
use types::ErrorCode;

const LOCAL_DEV: &str = "local-dev";
const STRICT_EGRESS: &str = "strict-egress";
const OFFLINE: &str = "offline";

fn template_value(name: &str) -> Result<Value, ErrorCode> {
    match name {
        LOCAL_DEV => Ok(json!({})),
        STRICT_EGRESS => Ok(json!({ "egress": { "allow_hosts": [] } })),
        OFFLINE => Ok(json!({
            "egress": { "allow_hosts": [] },
            "network": "deny"
        })),
        _ => Err(ErrorCode::SchemaInvalid),
    }
}

/// Resolve bind policy from a named template and/or inline JSON.
///
/// # Errors
/// Returns [`ErrorCode::SchemaInvalid`] when both are set or the template is unknown.
pub fn resolve_policy(template: Option<&str>, inline: Option<Value>) -> Result<Value, ErrorCode> {
    match (template, inline) {
        (Some(_), Some(_)) => Err(ErrorCode::SchemaInvalid),
        (Some(name), None) => template_value(name),
        (None, Some(v)) => Ok(v),
        (None, None) => Ok(json!({})),
    }
}

#[must_use]
pub fn list_template_names() -> &'static [&'static str] {
    &[LOCAL_DEV, STRICT_EGRESS, OFFLINE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dev_is_empty_object() {
        let v = resolve_policy(Some(LOCAL_DEV), None).expect("ok");
        assert_eq!(v, json!({}));
    }

    #[test]
    fn strict_egress_blocks_hosts() {
        let v = resolve_policy(Some(STRICT_EGRESS), None).expect("ok");
        assert_eq!(v["egress"]["allow_hosts"], json!([]));
    }

    #[test]
    fn offline_denies_network() {
        let v = resolve_policy(Some(OFFLINE), None).expect("ok");
        assert_eq!(v["network"], "deny");
    }

    #[test]
    fn both_set_is_invalid() {
        assert_eq!(
            resolve_policy(Some(LOCAL_DEV), Some(json!({}))),
            Err(ErrorCode::SchemaInvalid)
        );
    }

    #[test]
    fn unknown_template_is_invalid() {
        assert_eq!(
            resolve_policy(Some("nope"), None),
            Err(ErrorCode::SchemaInvalid)
        );
    }

    #[test]
    fn list_template_names_matches_builtins() {
        assert_eq!(
            list_template_names(),
            &["local-dev", "strict-egress", "offline"]
        );
    }

    #[test]
    fn inline_only_returns_copy() {
        let inline = json!({"caps": {"max_tokens": 8}});
        let v = resolve_policy(None, Some(inline.clone())).expect("ok");
        assert_eq!(v, inline);
    }
}
