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
    /// When false, `sh -c` / `cmd /C` wrappers are denied.
    allow_shell: bool,
}

impl ProgramAllowlist {
    /// Unrestricted programs; shell wrappers still denied until `sandbox.shell`.
    #[must_use]
    pub fn unrestricted() -> Self {
        Self {
            allowed: None,
            allow_shell: false,
        }
    }

    /// Parse `{ "sandbox": { "programs": ["git"], "shell": true } }`.
    ///
    /// Absent `sandbox.programs` → unrestricted. Present array (even empty) → deny-by-default.
    /// Absent `sandbox.shell` → wrappers denied.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let allow_shell = policy
            .pointer("/sandbox/shell")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let allowed = policy
            .pointer("/sandbox/programs")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(normalize_program)
                    .filter(|s| !s.is_empty())
                    .collect()
            });
        Self {
            allowed,
            allow_shell,
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

    /// Program allowlist plus shell-wrapper gate (`sh -c` / `cmd /C`).
    ///
    /// # Errors
    /// Returns [`ErrorCode::PolicyDenied`] for unknown programs or disallowed wrappers.
    pub fn check_exec(&self, argv: &[String]) -> Result<(), ErrorCode> {
        if is_shell_wrapper(argv) && !self.allow_shell {
            return Err(ErrorCode::PolicyDenied);
        }
        argv.first().map_or(Ok(()), |p| self.permits(p))
    }
}

pub(crate) fn is_shell_wrapper(argv: &[String]) -> bool {
    let Some(prog) = argv.first() else {
        return false;
    };
    let name = normalize_program(prog);
    let rest = &argv[1..];
    match name.as_str() {
        "sh" | "bash" | "zsh" | "dash" | "ash" => rest.iter().any(|a| a == "-c"),
        "cmd" => rest.iter().any(|a| a.eq_ignore_ascii_case("/c")),
        _ => false,
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

    #[test]
    fn sh_dash_c_denied_unless_shell_true() {
        let deny = ProgramAllowlist::from_policy(&json!({}));
        assert_eq!(
            deny.check_exec(&["sh".into(), "-c".into(), "echo hi".into()]),
            Err(ErrorCode::PolicyDenied)
        );
        let allow = ProgramAllowlist::from_policy(&json!({ "sandbox": { "shell": true } }));
        assert!(allow
            .check_exec(&["sh".into(), "-c".into(), "echo hi".into()])
            .is_ok());
    }

    #[test]
    fn cmd_slash_c_denied_unless_shell_true() {
        let deny = ProgramAllowlist::from_policy(&json!({}));
        assert_eq!(
            deny.check_exec(&["cmd".into(), "/C".into(), "echo hi".into()]),
            Err(ErrorCode::PolicyDenied)
        );
        let allow = ProgramAllowlist::from_policy(&json!({ "sandbox": { "shell": true } }));
        assert!(allow
            .check_exec(&["cmd.exe".into(), "/c".into(), "echo hi".into()])
            .is_ok());
    }
}
