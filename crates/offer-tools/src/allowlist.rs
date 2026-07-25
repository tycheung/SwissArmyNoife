//! Binding-frozen tool allowlist (`policy.tools.allow`).

use std::collections::BTreeSet;

use serde_json::Value;
use types::ErrorCode;

/// Tools permitted for a binding TTL. Missing policy key → allow all (ambient).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolAllowlist {
    /// `None` = unrestricted; `Some(empty)` = deny all; else exact id match (case-insensitive).
    allowed: Option<BTreeSet<String>>,
}

impl ToolAllowlist {
    /// Unrestricted (no `tools.allow` in policy).
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { allowed: None }
    }

    /// Parse from binding `policy_json`.
    ///
    /// Expected shape: `{ "tools": { "allow": ["read", "write", ...] } }`.
    /// Absent `tools.allow` → unrestricted. Present array (even empty) → deny-by-default list.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let Some(arr) = policy.pointer("/tools/allow").and_then(Value::as_array) else {
            return Self::unrestricted();
        };
        let allowed = arr
            .iter()
            .filter_map(Value::as_str)
            .map(normalize_tool_id)
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            allowed: Some(allowed),
        }
    }

    /// Whether any explicit allowlist is active.
    #[must_use]
    pub fn is_restricted(&self) -> bool {
        self.allowed.is_some()
    }

    /// Listed tool ids (empty when unrestricted).
    #[must_use]
    pub fn allowed_ids(&self) -> Vec<&str> {
        match &self.allowed {
            None => Vec::new(),
            Some(set) => set.iter().map(String::as_str).collect(),
        }
    }

    /// Permit `tool_id` under this binding policy.
    ///
    /// # Errors
    /// Returns [`ErrorCode::PolicyDenied`] when the tool is not on the allowlist.
    pub fn permits(&self, tool_id: &str) -> Result<(), ErrorCode> {
        let Some(set) = &self.allowed else {
            return Ok(());
        };
        let id = normalize_tool_id(tool_id);
        if set.contains(&id) {
            Ok(())
        } else {
            Err(ErrorCode::PolicyDenied)
        }
    }
}

fn normalize_tool_id(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_tools_allow_is_unrestricted() {
        let a = ToolAllowlist::from_policy(&json!({ "risk_caps": {} }));
        assert!(!a.is_restricted());
        assert!(a.permits("shell").is_ok());
        assert!(a.permits("read").is_ok());
    }

    #[test]
    fn empty_allow_denies_all() {
        let a = ToolAllowlist::from_policy(&json!({ "tools": { "allow": [] } }));
        assert!(a.is_restricted());
        assert_eq!(a.permits("read"), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn listed_tools_permitted_case_insensitive() {
        let a = ToolAllowlist::from_policy(&json!({
            "tools": { "allow": ["Read", "WRITE", "shell"] }
        }));
        assert!(a.permits("read").is_ok());
        assert!(a.permits("Write").is_ok());
        assert!(a.permits("shell").is_ok());
        assert_eq!(a.permits("grep"), Err(ErrorCode::PolicyDenied));
        assert_eq!(a.permits("edit"), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn whitespace_and_empty_entries_ignored() {
        let a = ToolAllowlist::from_policy(&json!({
            "tools": { "allow": ["  grep  ", "", "   "] }
        }));
        assert!(a.permits("grep").is_ok());
        assert_eq!(a.permits("read"), Err(ErrorCode::PolicyDenied));
    }
}
