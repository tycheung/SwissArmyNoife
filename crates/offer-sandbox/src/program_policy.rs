//! Binding-frozen sandbox program allowlist (`policy.sandbox.programs`).

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use types::ErrorCode;

/// Programs permitted for a binding TTL. Missing policy key → allow all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramAllowlist {
    /// `None` = unrestricted; `Some(empty)` = deny all; else basename match.
    allowed: Option<BTreeSet<String>>,
}

impl ProgramAllowlist {
    /// Unrestricted (no `sandbox.programs` in policy).
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { allowed: None }
    }

    /// Parse `{ "sandbox": { "programs": ["git", "cargo"] } }`.
    ///
    /// Absent `sandbox.programs` → unrestricted. Present array (even empty) → deny-by-default.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let Some(arr) = policy
            .pointer("/sandbox/programs")
            .and_then(Value::as_array)
        else {
            return Self::unrestricted();
        };
        let allowed = arr
            .iter()
            .filter_map(Value::as_str)
            .map(normalize_program)
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

    /// Permit `argv[0]` under this binding policy.
    ///
    /// # Errors
    /// Returns [`ErrorCode::PolicyDenied`] when the program is not on the allowlist.
    pub fn permits(&self, argv0: &str) -> Result<(), ErrorCode> {
        let Some(set) = &self.allowed else {
            return Ok(());
        };
        if set.contains(&normalize_program(argv0)) {
            Ok(())
        } else {
            Err(ErrorCode::PolicyDenied)
        }
    }
}

fn normalize_program(raw: &str) -> String {
    let name = Path::new(raw)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(raw);
    let lower = name.trim().to_ascii_lowercase();
    lower
        .strip_suffix(".exe")
        .unwrap_or(lower.as_str())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_programs_is_unrestricted() {
        let a = ProgramAllowlist::from_policy(&json!({ "risk_caps": {} }));
        assert!(!a.is_restricted());
        assert!(a.permits("rm").is_ok());
    }

    #[test]
    fn empty_allow_denies_all() {
        let a = ProgramAllowlist::from_policy(&json!({ "sandbox": { "programs": [] } }));
        assert_eq!(a.permits("git"), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn git_and_cargo_fixture_allowed() {
        let a = ProgramAllowlist::from_policy(&json!({
            "sandbox": { "programs": ["git", "cargo"] }
        }));
        assert!(a.permits("git").is_ok());
        assert!(a.permits("GIT.EXE").is_ok());
        assert!(a.permits("cargo").is_ok());
        assert_eq!(a.permits("python"), Err(ErrorCode::PolicyDenied));
    }
}
