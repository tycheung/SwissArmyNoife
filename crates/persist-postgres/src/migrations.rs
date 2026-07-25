//! Postgres migration sketch (`sak070-m` / `sak070-w` / `sak070-x` / `sak070-y` / `sak070-z`).
//!
//! Holds a schema version constant and apply helpers. Real SQL execution against
//! a live pool lands in a later sak070 slice. This module **does not** open sockets.

use crate::pool::PoolHandle;
use thiserror::Error;

/// Target schema version for a future Postgres adapter (`sak070-m`).
pub const SCHEMA_VERSION: u32 = 1;

/// Whether migrations should run given the highest applied version (`sak070-ap`).
///
/// `None` means the version table is empty / unread — treat as needs apply.
#[must_use]
pub fn schema_needs_apply(applied_max: Option<u32>) -> bool {
    match applied_max {
        None => true,
        Some(v) => v < SCHEMA_VERSION,
    }
}

/// Errors from [`try_apply`] / [`MigrationExecutor`] (`sak070-m` / `sak070-w`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MigrationError {
    #[error("postgres migrations not implemented (schema v{0})")]
    NotImplemented(u32),
    #[error("postgres migration execute failed: {0}")]
    Execute(String),
}

/// Executes one DDL statement (`sak070-w`).
pub trait MigrationExecutor {
    fn execute(&mut self, sql: &str) -> Result<(), MigrationError>;
}

/// Default executor — refuses every statement (`sak070-w`).
#[derive(Debug, Default, Clone, Copy)]
pub struct UnimplementedMigrationExecutor;

impl MigrationExecutor for UnimplementedMigrationExecutor {
    fn execute(&mut self, _sql: &str) -> Result<(), MigrationError> {
        Err(MigrationError::NotImplemented(SCHEMA_VERSION))
    }
}

/// Test / dry-run executor that records SQL without a database (`sak070-w`).
#[derive(Debug, Default, Clone)]
pub struct RecordingMigrationExecutor {
    pub statements: Vec<String>,
}

impl MigrationExecutor for RecordingMigrationExecutor {
    fn execute(&mut self, sql: &str) -> Result<(), MigrationError> {
        self.statements.push(sql.to_string());
        Ok(())
    }
}

/// Executor bound to a [`PoolHandle`] (`sak070-y`).
///
/// When the handle is connected (`--features postgres` + successful connect),
/// statements run via [`PoolHandle::execute_sql`]. Unconnected handles still
/// return [`MigrationError::NotImplemented`].
#[derive(Debug)]
pub struct PoolBoundMigrationExecutor<'a> {
    pub pool: &'a PoolHandle,
}

impl MigrationExecutor for PoolBoundMigrationExecutor<'_> {
    fn execute(&mut self, sql: &str) -> Result<(), MigrationError> {
        if !self.pool.is_connected() {
            let _ = self.pool.config();
            return Err(MigrationError::NotImplemented(SCHEMA_VERSION));
        }
        self.pool
            .execute_sql(sql)
            .map_err(|e| MigrationError::Execute(e.to_string()))
    }
}

/// Apply [`planned_statements`] via `executor` (`sak070-x`).
pub fn try_apply_with_executor(
    _schema_version: u32,
    executor: &mut dyn MigrationExecutor,
) -> Result<(), MigrationError> {
    for stmt in planned_statements() {
        executor.execute(stmt)?;
    }
    Ok(())
}

/// Apply planned DDL via a pool-bound executor (`sak070-z`).
///
/// Requires a connected [`PoolHandle`] (`--features postgres`); otherwise
/// [`MigrationError::NotImplemented`].
pub fn try_apply_with_pool(schema_version: u32, pool: &PoolHandle) -> Result<(), MigrationError> {
    let mut exec = PoolBoundMigrationExecutor { pool };
    try_apply_with_executor(schema_version, &mut exec)
}

/// Apply planned DDL then post-apply DML via `executor` (`sak070-aj`).
pub fn try_apply_planned_then_post(
    schema_version: u32,
    executor: &mut dyn MigrationExecutor,
) -> Result<(), MigrationError> {
    try_apply_with_executor(schema_version, executor)?;
    for stmt in crate::planned_post_apply_statements(schema_version) {
        executor.execute(&stmt)?;
    }
    Ok(())
}

/// Apply planned DDL then post-apply DML via a pool-bound executor (`sak070-al`).
///
/// Requires a connected [`PoolHandle`] (`--features postgres`); otherwise
/// [`MigrationError::NotImplemented`].
pub fn try_apply_planned_then_post_with_pool(
    schema_version: u32,
    pool: &PoolHandle,
) -> Result<(), MigrationError> {
    let mut exec = PoolBoundMigrationExecutor { pool };
    try_apply_planned_then_post(schema_version, &mut exec)
}

/// Apply pending migrations against an open pool (`sak070-m`).
///
/// Uses [`UnimplementedMigrationExecutor`] until a real pool executor lands.
/// See [`planned_statements`] for the DDL that will run later (`sak070-q`).
pub fn try_apply(schema_version: u32) -> Result<(), MigrationError> {
    let mut exec = UnimplementedMigrationExecutor;
    try_apply_with_executor(schema_version, &mut exec)
}

/// Ordered DDL statements planned for schema v1 (`sak070-q`).
#[must_use]
pub fn planned_statements() -> &'static [&'static str] {
    crate::V1_DDL
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::PoolConfig;

    #[test]
    fn schema_version_is_one() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn schema_needs_apply_when_missing_or_behind() {
        assert!(schema_needs_apply(None));
        assert!(schema_needs_apply(Some(0)));
        assert!(!schema_needs_apply(Some(SCHEMA_VERSION)));
        assert!(!schema_needs_apply(Some(SCHEMA_VERSION + 1)));
    }

    #[test]
    fn try_apply_not_implemented() {
        let err = try_apply(SCHEMA_VERSION).unwrap_err();
        assert_eq!(err, MigrationError::NotImplemented(SCHEMA_VERSION));
        assert!(err.to_string().contains("not implemented"));
    }

    #[test]
    fn planned_statements_match_v1_ddl() {
        assert_eq!(planned_statements().len(), 10);
        assert!(planned_statements()[0].contains("schema_migrations"));
        assert!(planned_statements()[1].contains("catalog_offers"));
        assert!(planned_statements()[4].contains("idx_bindings_offer"));
        assert!(planned_statements()[6].contains("idx_catalog_origin"));
    }

    #[test]
    fn recording_executor_runs_all_planned() {
        let mut rec = RecordingMigrationExecutor::default();
        try_apply_with_executor(SCHEMA_VERSION, &mut rec).expect("record");
        assert_eq!(rec.statements.len(), planned_statements().len());
        assert!(rec.statements[0].contains("schema_migrations"));
    }

    #[test]
    fn planned_then_post_records_version_insert() {
        let mut rec = RecordingMigrationExecutor::default();
        try_apply_planned_then_post(SCHEMA_VERSION, &mut rec).expect("record");
        assert_eq!(rec.statements.len(), planned_statements().len() + 1);
        assert!(rec
            .statements
            .last()
            .unwrap()
            .contains("INSERT INTO schema_migrations"));
    }

    #[test]
    fn pool_bound_apply_not_implemented() {
        let pool = PoolHandle::unconnected(PoolConfig {
            url: "postgres://localhost/sak".into(),
            max_connections: 4,
        })
        .expect("unconnected");
        let err = try_apply_with_pool(SCHEMA_VERSION, &pool).unwrap_err();
        assert_eq!(err, MigrationError::NotImplemented(SCHEMA_VERSION));
    }

    #[test]
    fn pool_planned_then_post_not_implemented() {
        let pool = PoolHandle::unconnected(PoolConfig {
            url: "postgres://localhost/sak".into(),
            max_connections: 4,
        })
        .expect("unconnected");
        let err = try_apply_planned_then_post_with_pool(SCHEMA_VERSION, &pool).unwrap_err();
        assert_eq!(err, MigrationError::NotImplemented(SCHEMA_VERSION));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn live_apply_v1_ddl_when_sak_pg_url_set() {
        use crate::env::test_lock;
        use crate::pool::PoolConfig;

        let _g = test_lock::lock();
        let url = match std::env::var("SAK_PG_URL").or_else(|_| std::env::var("DATABASE_URL")) {
            Ok(u) if !u.trim().is_empty() => u,
            _ => {
                eprintln!(
                    "skip live_apply_v1_ddl_when_sak_pg_url_set (no SAK_PG_URL/DATABASE_URL)"
                );
                return;
            }
        };
        let pool = PoolHandle::try_connect(PoolConfig {
            url,
            max_connections: 2,
        })
        .expect("live connect");
        try_apply_planned_then_post_with_pool(SCHEMA_VERSION, &pool).expect("apply V1_DDL");
        // Idempotent re-apply (IF NOT EXISTS / ON CONFLICT).
        try_apply_planned_then_post_with_pool(SCHEMA_VERSION, &pool).expect("re-apply");
        pool.execute_sql("SELECT version FROM schema_migrations WHERE version = 1")
            .expect("version row selectable");
    }
}
