//! Live Postgres [`BindingStore`] (`sak070` Phase B).

use super::{BindingRow, BindingStore, PersistPortError, PortResult};
use crate::binding_upsert_sql;
use crate::pool::PoolHandle;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Port-shaped select including `created_at` (sketch select omits it).
const BINDING_BY_ID_PORT_SQL: &str =
    "SELECT binding_id, offer_id, created_at FROM bindings WHERE binding_id = $1";

/// Binding adapter backed by a connected [`PoolHandle`].
#[derive(Debug, Clone)]
pub struct PostgresBindingStore {
    pool: Arc<PoolHandle>,
}

impl PostgresBindingStore {
    /// Wrap a connected pool handle.
    #[must_use]
    pub fn new(pool: Arc<PoolHandle>) -> Self {
        Self { pool }
    }

    /// Shared pool handle.
    #[must_use]
    pub fn pool(&self) -> &Arc<PoolHandle> {
        &self.pool
    }
}

fn system_time_to_unix(t: SystemTime) -> i64 {
    i64::try_from(t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()).unwrap_or(i64::MAX)
}

impl BindingStore for PostgresBindingStore {
    fn insert_binding(&self, row: &BindingRow) -> PortResult<()> {
        let principal = "local";
        let policy_json = "{}";
        let expires_at: Option<SystemTime> = None;
        self.pool
            .execute_params(
                binding_upsert_sql(),
                &[
                    &row.binding_id,
                    &row.offer_id,
                    &principal,
                    &policy_json,
                    &expires_at,
                ],
            )
            .map_err(PersistPortError::from)?;
        let _ = row.created_at_unix; // DB DEFAULT NOW(); read back via get_binding
        Ok(())
    }

    fn get_binding(&self, binding_id: &str) -> PortResult<Option<BindingRow>> {
        self.pool
            .query_opt(BINDING_BY_ID_PORT_SQL, &[&binding_id], |row| {
                let created: SystemTime = row.try_get(2).map_err(|e| e.to_string())?;
                Ok(BindingRow {
                    binding_id: row.try_get(0).map_err(|e| e.to_string())?,
                    offer_id: row.try_get(1).map_err(|e| e.to_string())?,
                    created_at_unix: system_time_to_unix(created),
                })
            })
            .map_err(PersistPortError::from)
    }
}
