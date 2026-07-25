//! Postgres pool config + connect (`sak070-f` / `sak070-g` / `sak070-k` / live pool).
//!
//! Without `--features postgres`, [`PoolHandle::try_connect`] validates the URL then
//! returns [`PoolConnectError::NotImplemented`].
//! With the feature, connect uses `deadpool-postgres` on a dedicated Tokio runtime
//! (sync API via `block_on`).
//!
//! # Env
//!
//! | Variable | Role |
//! |----------|------|
//! | `SAK_PG_URL` / `DATABASE_URL` | URL via [`crate::pg_url_from_env`] |
//! | `SAK_PG_MAX_CONNECTIONS` | Pool size (default [`DEFAULT_MAX_CONNECTIONS`]) |
//! | `SAK_PERSIST_BACKEND=postgres` | Gate for [`PoolConfig::from_env_if_postgres_backend`] |
//!
//! # Safety
//!
//! [`Display`] for [`PoolConfig`] **redacts** the URL (credentials must not land in logs).

use crate::env::{pg_url_from_env, postgres_enabled};
use std::fmt;
use thiserror::Error;

/// Env override for pool size (`SAK_PG_MAX_CONNECTIONS`).
pub const PG_MAX_CONNECTIONS_ENV: &str = "SAK_PG_MAX_CONNECTIONS";

/// Default max connections when env is unset or invalid.
pub const DEFAULT_MAX_CONNECTIONS: u32 = 10;

/// Errors from pool config validation (`sak070-g`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PoolConfigError {
    #[error("postgres URL missing")]
    MissingUrl,
    #[error("invalid postgres URL scheme (expected postgres:// or postgresql://)")]
    InvalidUrl,
}

/// Errors from [`PoolHandle::try_connect`] (`sak070-k`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PoolConnectError {
    #[error(transparent)]
    Config(#[from] PoolConfigError),
    #[error("postgres pool connect not implemented (build with --features postgres)")]
    NotImplemented,
    #[error("postgres pool connect failed: {0}")]
    Connect(String),
}

/// Opaque handle for a Postgres pool (`sak070-k`).
///
/// With `--features postgres`, [`Self::try_connect`] opens a live `deadpool` pool.
/// Without the feature (or via [`Self::unconnected`]), [`Self::is_connected`] is false.
#[derive(Debug)]
pub struct PoolHandle {
    config: PoolConfig,
    #[cfg(feature = "postgres")]
    pool: Option<deadpool_postgres::Pool>,
}

impl PoolHandle {
    /// Validate `config` then open a pool when built with `--features postgres`.
    pub fn try_connect(config: PoolConfig) -> Result<Self, PoolConnectError> {
        config.validate_url_scheme()?;
        #[cfg(not(feature = "postgres"))]
        {
            let _ = config;
            return Err(PoolConnectError::NotImplemented);
        }
        #[cfg(feature = "postgres")]
        {
            Self::connect_deadpool(config)
        }
    }

    /// Build an **unconnected** handle after scheme validation (`sak070-y`).
    pub fn unconnected(config: PoolConfig) -> Result<Self, PoolConfigError> {
        config.validate_url_scheme()?;
        Ok(Self {
            config,
            #[cfg(feature = "postgres")]
            pool: None,
        })
    }

    /// Access validated config.
    #[must_use]
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Whether this handle owns a live pool.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        #[cfg(feature = "postgres")]
        {
            self.pool.is_some()
        }
        #[cfg(not(feature = "postgres"))]
        {
            false
        }
    }

    /// Resolve [`PoolConfig::try_from_env`] then [`Self::try_connect`].
    pub fn try_connect_from_env() -> Result<Self, PoolConnectError> {
        let cfg = PoolConfig::try_from_env()?;
        Self::try_connect(cfg)
    }

    /// Run `sql` on a connected pool (sync wrapper).
    ///
    /// Returns [`PoolConnectError::NotImplemented`] when not connected / feature off.
    pub fn execute_sql(&self, sql: &str) -> Result<(), PoolConnectError> {
        #[cfg(not(feature = "postgres"))]
        {
            let _ = sql;
            return Err(PoolConnectError::NotImplemented);
        }
        #[cfg(feature = "postgres")]
        {
            let pool = self.pool.as_ref().ok_or(PoolConnectError::NotImplemented)?;
            pg_runtime()
                .block_on(async {
                    let client = pool.get().await.map_err(|e| e.to_string())?;
                    client.batch_execute(sql).await.map_err(|e| e.to_string())?;
                    Ok::<(), String>(())
                })
                .map_err(PoolConnectError::Connect)
        }
    }

    /// Execute parameterized SQL; returns rows affected (`sak070` live stores).
    ///
    /// Returns [`PoolConnectError::NotImplemented`] when not connected / feature off.
    #[cfg(feature = "postgres")]
    pub fn execute_params(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, PoolConnectError> {
        let pool = self.pool.as_ref().ok_or(PoolConnectError::NotImplemented)?;
        pg_runtime()
            .block_on(async {
                let client = pool.get().await.map_err(|e| e.to_string())?;
                let n = client
                    .execute(sql, params)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(n)
            })
            .map_err(PoolConnectError::Connect)
    }

    /// Query rows and map each via `map` (`sak070` live stores).
    ///
    /// Returns [`PoolConnectError::NotImplemented`] when not connected / feature off.
    #[cfg(feature = "postgres")]
    pub fn query_map<T, F>(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        mut map: F,
    ) -> Result<Vec<T>, PoolConnectError>
    where
        F: FnMut(tokio_postgres::Row) -> Result<T, String>,
    {
        let pool = self.pool.as_ref().ok_or(PoolConnectError::NotImplemented)?;
        pg_runtime()
            .block_on(async {
                let client = pool.get().await.map_err(|e| e.to_string())?;
                let rows = client.query(sql, params).await.map_err(|e| e.to_string())?;
                let mut out = Vec::with_capacity(rows.len());
                for row in rows {
                    out.push(map(row)?);
                }
                Ok(out)
            })
            .map_err(PoolConnectError::Connect)
    }

    /// Query at most one row (`sak070` live stores).
    #[cfg(feature = "postgres")]
    pub fn query_opt<T, F>(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        map: F,
    ) -> Result<Option<T>, PoolConnectError>
    where
        F: FnOnce(tokio_postgres::Row) -> Result<T, String>,
    {
        let pool = self.pool.as_ref().ok_or(PoolConnectError::NotImplemented)?;
        pg_runtime()
            .block_on(async {
                let client = pool.get().await.map_err(|e| e.to_string())?;
                let row = client
                    .query_opt(sql, params)
                    .await
                    .map_err(|e| e.to_string())?;
                match row {
                    None => Ok(None),
                    Some(r) => map(r).map(Some),
                }
            })
            .map_err(PoolConnectError::Connect)
    }

    #[cfg(feature = "postgres")]
    fn connect_deadpool(config: PoolConfig) -> Result<Self, PoolConnectError> {
        use std::str::FromStr;

        let pg_config = tokio_postgres::Config::from_str(&config.url)
            .map_err(|e| PoolConnectError::Connect(e.to_string()))?;
        let mgr = deadpool_postgres::Manager::new(pg_config, tokio_postgres::NoTls);
        let pool = deadpool_postgres::Pool::builder(mgr)
            .max_size(config.max_connections.max(1) as usize)
            .build()
            .map_err(|e| PoolConnectError::Connect(e.to_string()))?;

        // Smoke: checkout + SELECT 1
        pg_runtime()
            .block_on(async {
                let client = pool.get().await.map_err(|e| e.to_string())?;
                client
                    .simple_query("SELECT 1")
                    .await
                    .map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            })
            .map_err(PoolConnectError::Connect)?;

        Ok(Self {
            config,
            pool: Some(pool),
        })
    }
}

#[cfg(feature = "postgres")]
fn pg_runtime() -> &'static tokio::runtime::Runtime {
    use std::sync::OnceLock;
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("persist-postgres")
            .build()
            .expect("persist-postgres tokio runtime")
    })
}

/// Configuration for a Postgres connection pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfig {
    /// Connection URL (`SAK_PG_URL` or `DATABASE_URL`).
    pub url: String,
    /// Maximum connections (default [`DEFAULT_MAX_CONNECTIONS`]).
    pub max_connections: u32,
}

impl fmt::Display for PoolConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PoolConfig {{ max_connections: {}, url: <redacted> }}",
            self.max_connections
        )
    }
}

impl PoolConfig {
    /// Build from env when a Postgres URL is present.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let url = pg_url_from_env()?;
        Some(Self {
            url,
            max_connections: max_connections_from_env(),
        })
    }

    /// `from_env` only when `SAK_PERSIST_BACKEND=postgres`.
    #[must_use]
    pub fn from_env_if_postgres_backend() -> Option<Self> {
        if !postgres_enabled() {
            return None;
        }
        Self::from_env()
    }

    /// Build from env and validate URL scheme (`sak070-h`).
    pub fn try_from_env() -> Result<Self, PoolConfigError> {
        let cfg = Self::from_env().ok_or(PoolConfigError::MissingUrl)?;
        cfg.validate_url_scheme()?;
        Ok(cfg)
    }

    /// Soft-validate URL scheme before connect (`sak070-g`).
    pub fn validate_url_scheme(&self) -> Result<(), PoolConfigError> {
        let lower = self.url.trim().to_ascii_lowercase();
        if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
            Ok(())
        } else if lower.is_empty() {
            Err(PoolConfigError::MissingUrl)
        } else {
            Err(PoolConfigError::InvalidUrl)
        }
    }
}

fn max_connections_from_env() -> u32 {
    std::env::var(PG_MAX_CONNECTIONS_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_MAX_CONNECTIONS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{test_lock, DATABASE_URL_ENV, PERSIST_BACKEND_ENV, PG_URL_ENV};
    use std::error::Error;

    fn restore(key: &str, prior: Option<String>) {
        match prior {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn from_env_reads_url_and_default_max() {
        let _g = test_lock::lock();
        let p_url = std::env::var(PG_URL_ENV).ok();
        let p_max = std::env::var(PG_MAX_CONNECTIONS_ENV).ok();
        std::env::set_var(PG_URL_ENV, "postgres://pool/stub");
        std::env::remove_var(PG_MAX_CONNECTIONS_ENV);
        let cfg = PoolConfig::from_env().expect("cfg");
        assert_eq!(cfg.url, "postgres://pool/stub");
        assert_eq!(cfg.max_connections, DEFAULT_MAX_CONNECTIONS);
        restore(PG_URL_ENV, p_url);
        restore(PG_MAX_CONNECTIONS_ENV, p_max);
    }

    #[test]
    fn from_env_respects_max_connections_override() {
        let _g = test_lock::lock();
        let p_url = std::env::var(PG_URL_ENV).ok();
        let p_db = std::env::var(DATABASE_URL_ENV).ok();
        let p_max = std::env::var(PG_MAX_CONNECTIONS_ENV).ok();
        std::env::remove_var(PG_URL_ENV);
        std::env::set_var(DATABASE_URL_ENV, "postgres://db/url");
        std::env::set_var(PG_MAX_CONNECTIONS_ENV, "4");
        let cfg = PoolConfig::from_env().expect("cfg");
        assert_eq!(cfg.max_connections, 4);
        restore(PG_URL_ENV, p_url);
        restore(DATABASE_URL_ENV, p_db);
        restore(PG_MAX_CONNECTIONS_ENV, p_max);
    }

    #[test]
    fn from_env_if_postgres_backend_requires_flag() {
        let _g = test_lock::lock();
        let p_back = std::env::var(PERSIST_BACKEND_ENV).ok();
        let p_url = std::env::var(PG_URL_ENV).ok();
        std::env::set_var(PG_URL_ENV, "postgres://x");
        std::env::set_var(PERSIST_BACKEND_ENV, "sqlite");
        assert!(PoolConfig::from_env_if_postgres_backend().is_none());
        std::env::set_var(PERSIST_BACKEND_ENV, "postgres");
        assert!(PoolConfig::from_env_if_postgres_backend().is_some());
        restore(PERSIST_BACKEND_ENV, p_back);
        restore(PG_URL_ENV, p_url);
    }

    #[test]
    fn display_redacts_url() {
        let cfg = PoolConfig {
            url: "postgres://secret:pass@host/db".into(),
            max_connections: 3,
        };
        let s = cfg.to_string();
        assert!(s.contains("max_connections: 3"));
        assert!(s.contains("<redacted>"));
        assert!(!s.contains("secret"));
        assert!(!s.contains("pass"));
    }

    #[test]
    fn validate_url_scheme_accepts_postgres() {
        let cfg = PoolConfig {
            url: "PostgreSQL://localhost/sak".into(),
            max_connections: 1,
        };
        assert!(cfg.validate_url_scheme().is_ok());
    }

    #[test]
    fn validate_url_scheme_rejects_http() {
        let cfg = PoolConfig {
            url: "http://localhost/sak".into(),
            max_connections: 1,
        };
        let err = cfg.validate_url_scheme().expect_err("http");
        assert_eq!(err, PoolConfigError::InvalidUrl);
        assert!(err.source().is_none());
        assert!(err.to_string().contains("invalid postgres URL"));
    }

    #[test]
    fn try_from_env_ok_with_valid_url() {
        let _g = test_lock::lock();
        let p_url = std::env::var(PG_URL_ENV).ok();
        let p_db = std::env::var(DATABASE_URL_ENV).ok();
        std::env::set_var(PG_URL_ENV, "postgres://ok/db");
        std::env::remove_var(DATABASE_URL_ENV);
        let cfg = PoolConfig::try_from_env().expect("ok");
        assert_eq!(cfg.url, "postgres://ok/db");
        restore(PG_URL_ENV, p_url);
        restore(DATABASE_URL_ENV, p_db);
    }

    #[test]
    fn try_connect_from_env_without_feature_or_unreachable() {
        let _g = test_lock::lock();
        let p_url = std::env::var(PG_URL_ENV).ok();
        let p_db = std::env::var(DATABASE_URL_ENV).ok();
        // Intentionally bad host — with feature: Connect err; without: NotImplemented.
        std::env::set_var(PG_URL_ENV, "postgres://127.0.0.1:1/nope");
        std::env::remove_var(DATABASE_URL_ENV);
        let err = PoolHandle::try_connect_from_env().unwrap_err();
        #[cfg(not(feature = "postgres"))]
        assert_eq!(err, PoolConnectError::NotImplemented);
        #[cfg(feature = "postgres")]
        assert!(matches!(err, PoolConnectError::Connect(_)), "{err:?}");
        restore(PG_URL_ENV, p_url);
        restore(DATABASE_URL_ENV, p_db);
    }

    #[test]
    fn try_from_env_missing_url() {
        let _g = test_lock::lock();
        let p_url = std::env::var(PG_URL_ENV).ok();
        let p_db = std::env::var(DATABASE_URL_ENV).ok();
        std::env::remove_var(PG_URL_ENV);
        std::env::remove_var(DATABASE_URL_ENV);
        assert_eq!(
            PoolConfig::try_from_env().unwrap_err(),
            PoolConfigError::MissingUrl
        );
        restore(PG_URL_ENV, p_url);
        restore(DATABASE_URL_ENV, p_db);
    }

    #[test]
    fn try_from_env_invalid_scheme() {
        let _g = test_lock::lock();
        let p_url = std::env::var(PG_URL_ENV).ok();
        let p_db = std::env::var(DATABASE_URL_ENV).ok();
        std::env::set_var(PG_URL_ENV, "mysql://nope");
        std::env::remove_var(DATABASE_URL_ENV);
        assert_eq!(
            PoolConfig::try_from_env().unwrap_err(),
            PoolConfigError::InvalidUrl
        );
        restore(PG_URL_ENV, p_url);
        restore(DATABASE_URL_ENV, p_db);
    }

    #[test]
    fn try_connect_validates_scheme_then_feature_path() {
        let cfg = PoolConfig {
            url: "postgres://127.0.0.1:1/sak".into(),
            max_connections: 2,
        };
        let err = PoolHandle::try_connect(cfg).unwrap_err();
        #[cfg(not(feature = "postgres"))]
        assert_eq!(err, PoolConnectError::NotImplemented);
        #[cfg(feature = "postgres")]
        assert!(matches!(err, PoolConnectError::Connect(_)), "{err:?}");
    }

    #[test]
    fn unconnected_ok_after_scheme_validation() {
        let cfg = PoolConfig {
            url: "postgresql://localhost/sak".into(),
            max_connections: 3,
        };
        let handle = PoolHandle::unconnected(cfg).expect("unconnected");
        assert_eq!(handle.config().max_connections, 3);
        assert!(!handle.is_connected());
    }

    #[test]
    fn try_connect_rejects_bad_scheme() {
        let cfg = PoolConfig {
            url: "http://localhost/sak".into(),
            max_connections: 2,
        };
        assert_eq!(
            PoolHandle::try_connect(cfg).unwrap_err(),
            PoolConnectError::Config(PoolConfigError::InvalidUrl)
        );
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn live_connect_when_sak_pg_url_set() {
        let _g = test_lock::lock();
        let url = match std::env::var("SAK_PG_URL").or_else(|_| std::env::var("DATABASE_URL")) {
            Ok(u) if !u.trim().is_empty() => u,
            _ => {
                eprintln!("skip live_connect_when_sak_pg_url_set (no SAK_PG_URL/DATABASE_URL)");
                return;
            }
        };
        let handle = PoolHandle::try_connect(PoolConfig {
            url,
            max_connections: 2,
        })
        .expect("live connect");
        assert!(handle.is_connected());
        handle.execute_sql("SELECT 1").expect("select");
    }
}
