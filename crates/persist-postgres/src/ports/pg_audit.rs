//! Live Postgres [`AuditStore`] (`sak070` Phase B).

use super::{AuditEventRow, AuditStore, PersistPortError, PortResult};
use crate::pool::PoolHandle;
use crate::{audit_invoke_exists_by_id_sql, audit_invoke_insert_sql};
use std::sync::Arc;

/// Audit adapter backed by a connected [`PoolHandle`].
#[derive(Debug, Clone)]
pub struct PostgresAuditStore {
    pool: Arc<PoolHandle>,
}

impl PostgresAuditStore {
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

    /// Whether an invoke row exists (test / health helper).
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the query fails.
    pub fn event_exists(&self, event_id: &str) -> PortResult<bool> {
        let rows = self
            .pool
            .query_map(audit_invoke_exists_by_id_sql(), &[&event_id], |row| {
                let _: i32 = row.try_get(0).map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(PersistPortError::from)?;
        Ok(!rows.is_empty())
    }
}

impl AuditStore for PostgresAuditStore {
    fn append_event(&self, row: &AuditEventRow) -> PortResult<()> {
        let offer_id: Option<&str> = None;
        let code: Option<&str> = None;
        let detail_json = "{}";
        self.pool
            .execute_params(
                audit_invoke_insert_sql(),
                &[
                    &row.event_id,
                    &row.binding_id,
                    &offer_id,
                    &row.kind,
                    &code,
                    &detail_json,
                ],
            )
            .map_err(PersistPortError::from)?;
        let _ = row.recorded_at_unix; // DB DEFAULT NOW()
        Ok(())
    }
}
