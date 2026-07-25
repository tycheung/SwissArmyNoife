//! Frozen egress policy snapshot for a binding.

use serde_json::Value;
use types::ErrorCode;

use crate::allowlist::HostnameAllowlist;
use crate::byte_cap::ResponseByteCap;
use crate::principal::PrincipalAllowlist;
use crate::url_host::{check_url, host_from_url};

/// Binding-frozen egress controls (hosts + principals + response bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EgressPolicy {
    pub hosts: HostnameAllowlist,
    pub principals: PrincipalAllowlist,
    pub max_response_bytes: ResponseByteCap,
}

impl EgressPolicy {
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        Self {
            hosts: HostnameAllowlist::from_policy(policy),
            principals: PrincipalAllowlist::from_policy(policy),
            max_response_bytes: ResponseByteCap::from_policy(policy),
        }
    }

    /// Principal gate then hostname check for `url`.
    ///
    /// # Errors
    /// Policy / egress / schema codes from nested checks.
    pub fn check(&self, principal: &str, url: &str) -> Result<String, ErrorCode> {
        self.principals.permits(principal)?;
        check_url(&self.hosts, url)
    }

    /// Host-only check (no principal).
    ///
    /// # Errors
    /// From [`check_url`].
    pub fn check_host_only(&self, url: &str) -> Result<String, ErrorCode> {
        check_url(&self.hosts, url)
    }

    /// Parse host without policy gates (for diagnostics).
    ///
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`].
    pub fn parse_host(url: &str) -> Result<String, ErrorCode> {
        host_from_url(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn combined_gates() {
        let p = EgressPolicy::from_policy(&json!({
            "egress": {
                "allow_hosts": ["api.example.com"],
                "allow_principals": ["local"],
                "max_response_bytes": 100
            }
        }));
        assert!(p.check("local", "https://api.example.com/").is_ok());
        assert_eq!(
            p.check("other", "https://api.example.com/"),
            Err(ErrorCode::PolicyDenied)
        );
        assert_eq!(
            p.check("local", "https://evil.com/"),
            Err(ErrorCode::EgressDenied)
        );
        assert_eq!(
            p.max_response_bytes.permits_len(101),
            Err(ErrorCode::BudgetExhausted)
        );
    }
}
