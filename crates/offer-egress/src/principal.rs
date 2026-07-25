//! Principal allowlist for egress (`policy.egress.allow_principals`).

use std::collections::BTreeSet;

use serde_json::Value;
use types::ErrorCode;

/// Binding-frozen principal gate (role/principal ids as opaque strings).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrincipalAllowlist {
    /// `None` = unrestricted; `Some` = deny-by-default exact match.
    principals: Option<BTreeSet<String>>,
}

impl PrincipalAllowlist {
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { principals: None }
    }

    /// Parse `{ "egress": { "allow_principals": ["local", "scraper"] } }`.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let Some(arr) = policy
            .pointer("/egress/allow_principals")
            .and_then(Value::as_array)
        else {
            return Self::unrestricted();
        };
        let principals = arr
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            principals: Some(principals),
        }
    }

    #[must_use]
    pub fn is_restricted(&self) -> bool {
        self.principals.is_some()
    }

    /// # Errors
    /// [`ErrorCode::PolicyDenied`] when principal not listed.
    pub fn permits(&self, principal: &str) -> Result<(), ErrorCode> {
        let Some(set) = &self.principals else {
            return Ok(());
        };
        let p = principal.trim().to_ascii_lowercase();
        if p.is_empty() {
            return Err(ErrorCode::SchemaInvalid);
        }
        if set.contains(&p) {
            Ok(())
        } else {
            Err(ErrorCode::PolicyDenied)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unrestricted_when_missing() {
        let a = PrincipalAllowlist::from_policy(&json!({}));
        assert!(a.permits("anyone").is_ok());
    }

    #[test]
    fn listed_only() {
        let a = PrincipalAllowlist::from_policy(&json!({
            "egress": { "allow_principals": ["local", "Scraper"] }
        }));
        assert!(a.permits("LOCAL").is_ok());
        assert!(a.permits("scraper").is_ok());
        assert_eq!(a.permits("other"), Err(ErrorCode::PolicyDenied));
    }
}
