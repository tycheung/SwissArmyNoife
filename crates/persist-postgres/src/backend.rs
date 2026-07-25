//! Runtime backend open helper (`sak070` Phase B).
//!
//! When `SAK_PERSIST_BACKEND=postgres` and a URL is set, connect, migrate, and
//! return live store adapters. `SQLite` remains the default when unset.

use crate::migrations::{try_apply_planned_then_post_with_pool, SCHEMA_VERSION};
use crate::pool::{PoolConfig, PoolHandle};
use crate::ports::{PortResult, PostgresAuditStore, PostgresBindingStore, PostgresCatalog};
use std::sync::Arc;

/// Connected Postgres persistence bundle for broker boot.
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    /// Shared pool.
    pub pool: Arc<PoolHandle>,
    /// Catalog store.
    pub catalog: PostgresCatalog,
    /// Binding store.
    pub bindings: PostgresBindingStore,
    /// Audit store.
    pub audit: PostgresAuditStore,
}

impl PostgresBackend {
    /// Build stores from an already-connected, migrated pool.
    #[must_use]
    pub fn from_pool(pool: PoolHandle) -> Self {
        let pool = Arc::new(pool);
        Self {
            catalog: PostgresCatalog::new(Arc::clone(&pool)),
            bindings: PostgresBindingStore::new(Arc::clone(&pool)),
            audit: PostgresAuditStore::new(Arc::clone(&pool)),
            pool,
        }
    }
}

/// Open Postgres backend when env requests it.
///
/// Returns `Ok(None)` when `SAK_PERSIST_BACKEND` is not `postgres` or no URL.
///
/// # Errors
/// Connect / migrate failures map to [`crate::ports::PersistPortError`].
pub fn try_open_from_env() -> PortResult<Option<PostgresBackend>> {
    let Some(cfg) = PoolConfig::from_env_if_postgres_backend() else {
        return Ok(None);
    };
    let pool = PoolHandle::try_connect(cfg)?;
    try_apply_planned_then_post_with_pool(SCHEMA_VERSION, &pool)?;
    Ok(Some(PostgresBackend::from_pool(pool)))
}

/// Connect + migrate using `SAK_PG_URL` / `DATABASE_URL` (no backend gate).
///
/// Intended for live tests. Returns `Ok(None)` when no URL is set.
///
/// # Errors
/// Connect / migrate failures map to [`crate::ports::PersistPortError`].
pub fn try_open_from_url_env() -> PortResult<Option<PostgresBackend>> {
    let Some(cfg) = PoolConfig::from_env() else {
        return Ok(None);
    };
    cfg.validate_url_scheme()?;
    let pool = PoolHandle::try_connect(cfg)?;
    try_apply_planned_then_post_with_pool(SCHEMA_VERSION, &pool)?;
    Ok(Some(PostgresBackend::from_pool(pool)))
}
