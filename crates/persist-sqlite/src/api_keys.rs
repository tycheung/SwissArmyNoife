//! API key persistence (`api_keys` table, `sak059-b`).

use rusqlite::{params, Connection};

use crate::Result;

/// One row from `api_keys` (no secret material).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiKeyDbRow {
    pub key_id: String,
    pub hash_hex: String,
    pub principal_id: String,
}

/// Insert or replace an API key row.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn put_api_key(
    conn: &Connection,
    key_id: &str,
    hash_hex: &str,
    principal_id: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO api_keys (key_id, hash_hex, principal_id)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key_id) DO UPDATE SET
           hash_hex = excluded.hash_hex,
           principal_id = excluded.principal_id",
        params![key_id, hash_hex, principal_id],
    )?;
    Ok(())
}

/// List all API keys ordered by `key_id`.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn list_api_keys(conn: &Connection) -> Result<Vec<ApiKeyDbRow>> {
    let mut stmt =
        conn.prepare("SELECT key_id, hash_hex, principal_id FROM api_keys ORDER BY key_id ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(ApiKeyDbRow {
            key_id: row.get(0)?,
            hash_hex: row.get(1)?,
            principal_id: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Lookup by SHA-256 hex digest of the bearer secret.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn get_api_key_by_hash(conn: &Connection, hash_hex: &str) -> Result<Option<ApiKeyDbRow>> {
    let mut stmt = conn.prepare(
        "SELECT key_id, hash_hex, principal_id FROM api_keys WHERE hash_hex = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query([hash_hex])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(ApiKeyDbRow {
        key_id: row.get(0)?,
        hash_hex: row.get(1)?,
        principal_id: row.get(2)?,
    }))
}

/// Delete an API key by id.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn revoke_api_key(conn: &Connection, key_id: &str) -> Result<bool> {
    let n = conn.execute("DELETE FROM api_keys WHERE key_id = ?1", [key_id])?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_and_migrate, put_api_key, CONFIG_DIR, DB_PATH};
    use control::{hash_api_key_secret, ApiKeyRow, ApiKeyStore};

    fn with_conn(test: impl FnOnce(&Connection)) {
        let _guard = crate::ENV_LOCK.lock().expect("env lock");
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var(CONFIG_DIR, tmp.path());
        std::env::remove_var(DB_PATH);
        let path = tmp.path().join("broker.db");
        let conn = open_and_migrate(&path).expect("migrate");
        test(&conn);
        std::env::remove_var(CONFIG_DIR);
        std::env::remove_var(DB_PATH);
    }

    #[test]
    fn mint_export_persist_reload_verify_roundtrip() {
        with_conn(|conn| {
            let store = ApiKeyStore::new();
            let (_info, secret) = store.mint("alice").expect("mint");
            for row in store.export_rows().expect("export") {
                put_api_key(conn, &row.key_id, &row.hash_hex, &row.principal_id).expect("put");
            }

            let db_rows = list_api_keys(conn).expect("list");
            assert_eq!(db_rows.len(), 1);

            let store2 = ApiKeyStore::new();
            let loaded: Vec<ApiKeyRow> = db_rows
                .into_iter()
                .map(|r| ApiKeyRow {
                    key_id: r.key_id,
                    hash_hex: r.hash_hex,
                    principal_id: r.principal_id,
                })
                .collect();
            store2.load_rows(loaded).expect("load");
            let p = store2.verify(&secret).expect("verify");
            assert_eq!(p.id, "alice");
        });
    }

    #[test]
    fn get_by_hash_and_revoke() {
        with_conn(|conn| {
            let hash = hash_api_key_secret("sk_live_test");
            put_api_key(conn, "sak_test", &hash, "bob").expect("put");
            let row = get_api_key_by_hash(conn, &hash).expect("get").expect("row");
            assert_eq!(row.key_id, "sak_test");
            assert!(revoke_api_key(conn, "sak_test").expect("revoke"));
            assert!(get_api_key_by_hash(conn, &hash).expect("get").is_none());
        });
    }
}
