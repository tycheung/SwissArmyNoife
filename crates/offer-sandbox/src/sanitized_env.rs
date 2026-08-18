//! Allowlisted environment for host sandbox children (`sak555`).

use std::collections::BTreeMap;

/// Keys copied into child processes. Anything else (including secrets) is dropped.
const ALLOWED_KEYS: &[&str] = &[
    "PATH",
    "PATHEXT",
    "HOME",
    "USER",
    "USERNAME",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TMP",
    "TEMP",
    "TMPDIR",
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "SYSTEMDRIVE",
];

/// Broker/host env snapshot with secret-bearing keys stripped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SanitizedEnv {
    vars: BTreeMap<String, String>,
}

impl SanitizedEnv {
    /// Filter an iterator of env pairs (tests + OS snapshot).
    #[must_use]
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut vars = BTreeMap::new();
        for (k, v) in pairs {
            let key = k.as_ref();
            if is_allowed_key(key) {
                vars.insert(key.to_string(), v.as_ref().to_string());
            }
        }
        Self { vars }
    }

    /// Snapshot the current process environment through the allowlist.
    #[must_use]
    pub fn from_os() -> Self {
        Self::from_pairs(std::env::vars())
    }

    /// Apply `env_clear` plus allowlisted keys onto `cmd`.
    pub fn apply_to(&self, cmd: &mut std::process::Command) {
        cmd.env_clear();
        for (k, v) in &self.vars {
            cmd.env(k, v);
        }
    }

    /// Keys present in the snapshot (sorted).
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.vars.keys().map(String::as_str).collect()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.iter().find_map(|(k, v)| {
            if k.eq_ignore_ascii_case(key) {
                Some(v.as_str())
            } else {
                None
            }
        })
    }
}

fn is_allowed_key(key: &str) -> bool {
    ALLOWED_KEYS.iter().any(|a| a.eq_ignore_ascii_case(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains_secret_key_name(blob: &str) -> bool {
        const SECRETS: &[&str] = &["VAULT_KEY", "OPENAI_API_KEY", "MCP_HTTP_TOKEN"];
        let upper = blob.to_ascii_uppercase();
        SECRETS.iter().any(|s| upper.contains(*s))
    }

    #[test]
    fn strips_vault_openai_and_mcp_token() {
        let env = SanitizedEnv::from_pairs([
            ("PATH", "/usr/bin"),
            ("VAULT_KEY", "super-secret"),
            ("OPENAI_API_KEY", "sk-test"),
            ("MCP_HTTP_TOKEN", "tok"),
            ("HOME", "/home/sak"),
        ]);
        assert_eq!(env.get("PATH"), Some("/usr/bin"));
        assert_eq!(env.get("HOME"), Some("/home/sak"));
        assert!(env.get("VAULT_KEY").is_none());
        assert!(env.get("OPENAI_API_KEY").is_none());
        assert!(env.get("MCP_HTTP_TOKEN").is_none());
        let blob = format!("{:?}", env.keys());
        assert!(!contains_secret_key_name(&blob));
    }

    #[test]
    fn os_snapshot_excludes_injected_secrets() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("VAULT_KEY", "do-not-leak");
        std::env::set_var("OPENAI_API_KEY", "do-not-leak");
        std::env::set_var("MCP_HTTP_TOKEN", "do-not-leak");
        let env = SanitizedEnv::from_os();
        std::env::remove_var("VAULT_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("MCP_HTTP_TOKEN");
        let dump = format!("{env:?}");
        assert!(!contains_secret_key_name(&dump));
        assert!(env.get("VAULT_KEY").is_none());
    }
}
