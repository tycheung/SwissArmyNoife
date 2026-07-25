//! Hostname allowlist for `network.egress` policy.

use std::collections::BTreeSet;

use serde_json::Value;
use types::ErrorCode;

/// Binding-frozen hostname allowlist (`policy.egress.allow_hosts`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostnameAllowlist {
    /// `None` = unrestricted; `Some` = deny-by-default exact/suffix match.
    hosts: Option<BTreeSet<String>>,
}

impl HostnameAllowlist {
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { hosts: None }
    }

    /// Parse `{ "egress": { "allow_hosts": ["api.example.com", "*.openai.com"] } }`.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let Some(arr) = policy
            .pointer("/egress/allow_hosts")
            .and_then(Value::as_array)
        else {
            return Self::unrestricted();
        };
        let hosts = arr
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        Self { hosts: Some(hosts) }
    }

    #[must_use]
    pub fn is_restricted(&self) -> bool {
        self.hosts.is_some()
    }

    /// Check a hostname (no scheme/port).
    ///
    /// # Errors
    /// [`ErrorCode::EgressDenied`] when not allowlisted.
    pub fn permits(&self, host: &str) -> Result<(), ErrorCode> {
        let Some(set) = &self.hosts else {
            return Ok(());
        };
        let host = host.trim().to_ascii_lowercase();
        if host.is_empty() {
            return Err(ErrorCode::SchemaInvalid);
        }
        if set.iter().any(|pat| host_matches(&host, pat)) {
            Ok(())
        } else {
            Err(ErrorCode::EgressDenied)
        }
    }
}

fn host_matches(host: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unrestricted_when_missing() {
        let a = HostnameAllowlist::from_policy(&json!({}));
        assert!(a.permits("anywhere.com").is_ok());
    }

    #[test]
    fn exact_and_wildcard() {
        let a = HostnameAllowlist::from_policy(&json!({
            "egress": { "allow_hosts": ["api.example.com", "*.openai.com"] }
        }));
        assert!(a.permits("api.example.com").is_ok());
        assert!(a.permits("cdn.openai.com").is_ok());
        assert!(a.permits("openai.com").is_ok());
        assert_eq!(a.permits("evil.com"), Err(ErrorCode::EgressDenied));
    }

    #[test]
    fn empty_list_denies() {
        let a = HostnameAllowlist::from_policy(&json!({
            "egress": { "allow_hosts": [] }
        }));
        assert_eq!(a.permits("x.com"), Err(ErrorCode::EgressDenied));
    }
}
