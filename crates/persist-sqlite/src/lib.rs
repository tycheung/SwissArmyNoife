//! `SQLite` persistence for `SwissArmyNoife` (migrations + DB open).

mod api_keys;
mod catalog;
mod connections;

use std::fs;
use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

pub use api_keys::{get_api_key_by_hash, list_api_keys, put_api_key, revoke_api_key, ApiKeyDbRow};
pub use catalog::{get_offer, list_offers, upsert_offer, CatalogOfferRow};
pub use connections::{
    delete_connection, get_connection_meta, get_connection_secret, list_connections,
    put_connection, ConnectionMeta,
};
pub use env::{config_dir, db_path, CONFIG_DIR, DB_PATH};

/// Persistence errors.
#[derive(Debug, Error)]
pub enum PersistError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("vault: {0}")]
    Vault(#[from] vault::VaultError),
}

type Result<T> = std::result::Result<T, PersistError>;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/001_bootstrap.sql")),
    (2, include_str!("../migrations/002_core_tables.sql")),
    (3, include_str!("../migrations/003_vault_connections.sql")),
    (4, include_str!("../migrations/004_memory_index_meta.sql")),
    (5, include_str!("../migrations/005_research_briefs.sql")),
    (6, include_str!("../migrations/006_work_units.sql")),
    (7, include_str!("../migrations/007_api_keys.sql")),
    (8, include_str!("../migrations/008_compute_nodes.sql")),
];

/// Open (or create) `path`, ensure parent dirs exist, and apply pending migrations.
///
/// # Errors
/// Returns [`PersistError`] on I/O or `SQLite` failures.
pub fn open_and_migrate(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;
    for &(version, sql) in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )?;
        if already {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [version],
        )?;
    }
    Ok(conn)
}

/// Open the default `broker.db` (see [`env::db_path`]) and migrate.
///
/// # Errors
/// Returns [`PersistError`] on I/O or `SQLite` failures.
pub fn open_default() -> Result<Connection> {
    open_and_migrate(&db_path())
}

#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn with_temp_config(test: impl FnOnce(&Connection)) {
        let _guard = crate::ENV_LOCK.lock().expect("env lock");
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var(CONFIG_DIR, tmp.path());
        std::env::remove_var(DB_PATH);
        let conn = open_default().expect("migrate");
        test(&conn);
        std::env::remove_var(CONFIG_DIR);
        std::env::remove_var(DB_PATH);
    }

    #[test]
    fn migrate_creates_db_under_config_dir() {
        with_temp_config(|_conn| {
            assert_eq!(
                db_path().file_name().and_then(|s| s.to_str()),
                Some("broker.db")
            );
            assert!(db_path().is_file());
            open_default().expect("idempotent remigrate");
        });
    }

    #[test]
    fn core_tables_accept_smoke_rows() {
        with_temp_config(|conn| {
            conn.execute(
                "INSERT INTO catalog_offers (offer_id, version) VALUES (?1, ?2)",
                ["llm.chat", "0.1.0"],
            )
            .expect("catalog insert");
            conn.execute(
                "INSERT INTO bindings (binding_id, offer_id) VALUES (?1, ?2)",
                ["00000000-0000-0000-0000-000000000001", "llm.chat"],
            )
            .expect("binding insert");
            conn.execute(
                "INSERT INTO audit_invokes (invoke_id, binding_id, status) VALUES (?1, ?2, ?3)",
                [
                    "00000000-0000-0000-0000-000000000002",
                    "00000000-0000-0000-0000-000000000001",
                    "ok",
                ],
            )
            .expect("audit insert");

            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM audit_invokes", [], |row| row.get(0))
                .expect("count");
            assert_eq!(count, 1);
        });
    }
}
