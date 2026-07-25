//! `SQLite` persistence for memory index metadata (`sak226`).

use rusqlite::Connection;
use types::ErrorCode;

/// Upsert metadata row for a scope.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on `SQLite` failure.
pub fn upsert_index_meta(
    conn: &Connection,
    scope_key: &str,
    fingerprint: &str,
    backend: &str,
    vector_count: i64,
) -> Result<(), ErrorCode> {
    conn.execute(
        "INSERT INTO memory_index_meta (scope_key, fingerprint, backend, vector_count, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(scope_key) DO UPDATE SET
           fingerprint=excluded.fingerprint,
           backend=excluded.backend,
           vector_count=excluded.vector_count,
           updated_at=excluded.updated_at",
        rusqlite::params![scope_key, fingerprint, backend, vector_count],
    )
    .map_err(|_| ErrorCode::SchemaInvalid)?;
    Ok(())
}

/// Load fingerprint for a scope, if any.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on `SQLite` failure.
pub fn get_index_fingerprint(
    conn: &Connection,
    scope_key: &str,
) -> Result<Option<String>, ErrorCode> {
    let mut stmt = conn
        .prepare("SELECT fingerprint FROM memory_index_meta WHERE scope_key = ?1")
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    let mut rows = stmt
        .query(rusqlite::params![scope_key])
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    match rows.next().map_err(|_| ErrorCode::SchemaInvalid)? {
        Some(row) => Ok(Some(row.get(0).map_err(|_| ErrorCode::SchemaInvalid)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use persist_sqlite::open_and_migrate;

    #[test]
    fn upsert_and_get() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("broker.db");
        let conn = open_and_migrate(&path).expect("migrate");
        upsert_index_meta(&conn, "repo:x", "fp1", "exact", 3).expect("up");
        assert_eq!(
            get_index_fingerprint(&conn, "repo:x")
                .expect("get")
                .as_deref(),
            Some("fp1")
        );
        upsert_index_meta(&conn, "repo:x", "fp2", "hnsw", 9).expect("up2");
        assert_eq!(
            get_index_fingerprint(&conn, "repo:x")
                .expect("get")
                .as_deref(),
            Some("fp2")
        );
    }
}
