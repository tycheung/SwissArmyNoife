//! Environment and path helpers for `SwissArmyNoife`.
//!
//! # Environment variables (process env)
//!
//! | Variable | Purpose |
//! |----------|---------|
//! | `CONFIG_DIR` | Config directory (default: platform config dir + `swissarmynoife`) |
//! | `DB_PATH` | Absolute/relative path to `broker.db` (overrides `{config}/broker.db`) |
//! | `LLM_BACKEND` | MCP: `ollama` (default) or `echo` |
//! | `SANDBOX_BACKEND` | MCP: `none` host+jail (default) or `stub` |
//!
//! # Resolution order
//!
//! **Config dir:** `CONFIG_DIR` → `{platform config}/swissarmynoife` → `./swissarmynoife`
//!
//! **Database path:** `DB_PATH` → `{config_dir}/broker.db`

use std::path::PathBuf;

/// `CONFIG_DIR`
pub const CONFIG_DIR: &str = "CONFIG_DIR";

/// `DB_PATH`
pub const DB_PATH: &str = "DB_PATH";

/// Platform config directory (no third-party `dirs` crate — keeps licenses MIT/Apache-clean).
fn platform_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            Some(PathBuf::from(xdg))
        } else {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

/// Resolve the config directory.
#[must_use]
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(CONFIG_DIR) {
        return PathBuf::from(dir);
    }
    platform_config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("swissarmynoife")
}

/// Resolve the `SQLite` database path.
#[must_use]
pub fn db_path() -> PathBuf {
    if let Some(path) = std::env::var_os(DB_PATH) {
        return PathBuf::from(path);
    }
    config_dir().join("broker.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_env() {
        std::env::remove_var(CONFIG_DIR);
        std::env::remove_var(DB_PATH);
    }

    #[test]
    fn config_dir_prefers_config_dir() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_env();
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var(CONFIG_DIR, tmp.path());
        assert_eq!(config_dir(), tmp.path());
        clear_env();
    }

    #[test]
    fn db_path_prefers_db_path_over_config_dir() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_env();
        let tmp = tempfile::tempdir().expect("tempdir");
        let explicit = tmp.path().join("custom.db");
        std::env::set_var(CONFIG_DIR, tmp.path().join("ignored-config"));
        std::env::set_var(DB_PATH, &explicit);
        assert_eq!(db_path(), explicit);
        clear_env();
    }

    #[test]
    fn db_path_falls_back_to_config_dir_join() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_env();
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var(CONFIG_DIR, tmp.path());
        assert_eq!(db_path(), tmp.path().join("broker.db"));
        clear_env();
    }
}
