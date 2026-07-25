//! Persist backend + Postgres URL env helpers (`sak070-e`).
//!
//! No connection pool yet — URL resolution only.

/// Environment variable read by [`postgres_enabled`].
pub const PERSIST_BACKEND_ENV: &str = "SAK_PERSIST_BACKEND";

/// Preferred Postgres URL env (`SAK_PG_URL`).
pub const PG_URL_ENV: &str = "SAK_PG_URL";

/// Fallback Postgres URL env (`DATABASE_URL`) when `SAK_PG_URL` is unset.
pub const DATABASE_URL_ENV: &str = "DATABASE_URL";

/// Returns `true` when `SAK_PERSIST_BACKEND` is set to `postgres` (case-insensitive).
#[must_use]
pub fn postgres_enabled() -> bool {
    std::env::var(PERSIST_BACKEND_ENV).is_ok_and(|v| v.trim().eq_ignore_ascii_case("postgres"))
}

/// Resolve a Postgres connection URL from the environment.
///
/// Preference order: non-empty `SAK_PG_URL`, then non-empty `DATABASE_URL`.
/// Does not open a pool or validate the URL scheme.
#[must_use]
pub fn pg_url_from_env() -> Option<String> {
    for key in [PG_URL_ENV, DATABASE_URL_ENV] {
        if let Ok(v) = std::env::var(key) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
pub(crate) mod test_lock {
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Shared process-env lock for `env` + `pool` tests (recover from poison).
    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restore_env(key: &str, prior: Option<String>) {
        match prior {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn postgres_enabled_reads_env() {
        let _guard = test_lock::lock();
        let prior = std::env::var(PERSIST_BACKEND_ENV).ok();
        std::env::set_var(PERSIST_BACKEND_ENV, "postgres");
        assert!(postgres_enabled());
        std::env::set_var(PERSIST_BACKEND_ENV, "POSTGRES");
        assert!(postgres_enabled());
        std::env::set_var(PERSIST_BACKEND_ENV, "sqlite");
        assert!(!postgres_enabled());
        restore_env(PERSIST_BACKEND_ENV, prior);
    }

    #[test]
    fn pg_url_prefers_sak_pg_url_over_database_url() {
        let _guard = test_lock::lock();
        let prior_sak = std::env::var(PG_URL_ENV).ok();
        let prior_db = std::env::var(DATABASE_URL_ENV).ok();
        std::env::set_var(PG_URL_ENV, "postgres://sak/local");
        std::env::set_var(DATABASE_URL_ENV, "postgres://fallback/db");
        assert_eq!(pg_url_from_env().as_deref(), Some("postgres://sak/local"));
        restore_env(PG_URL_ENV, prior_sak);
        restore_env(DATABASE_URL_ENV, prior_db);
    }

    #[test]
    fn pg_url_falls_back_to_database_url() {
        let _guard = test_lock::lock();
        let prior_sak = std::env::var(PG_URL_ENV).ok();
        let prior_db = std::env::var(DATABASE_URL_ENV).ok();
        std::env::remove_var(PG_URL_ENV);
        std::env::set_var(DATABASE_URL_ENV, "  postgres://from-database-url  ");
        assert_eq!(
            pg_url_from_env().as_deref(),
            Some("postgres://from-database-url")
        );
        restore_env(PG_URL_ENV, prior_sak);
        restore_env(DATABASE_URL_ENV, prior_db);
    }

    #[test]
    fn pg_url_none_when_unset_or_blank() {
        let _guard = test_lock::lock();
        let prior_sak = std::env::var(PG_URL_ENV).ok();
        let prior_db = std::env::var(DATABASE_URL_ENV).ok();
        std::env::set_var(PG_URL_ENV, "   ");
        std::env::remove_var(DATABASE_URL_ENV);
        assert!(pg_url_from_env().is_none());
        restore_env(PG_URL_ENV, prior_sak);
        restore_env(DATABASE_URL_ENV, prior_db);
    }
}
