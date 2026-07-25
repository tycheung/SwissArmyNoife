//! Vault-backed provider connection CRUD (`vault_connections`).
//!
//! List/get metadata never includes secrets. Plaintext is only returned via
//! [`get_connection_secret`] as [`SecretString`].

use rusqlite::{params, Connection};
use vault::{decrypt, encrypt, SecretString, VaultError, VaultKey};

use crate::{PersistError, Result};

/// Public connection metadata (no secret material).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionMeta {
    pub connection_id: String,
    pub provider: String,
    pub label: String,
}

/// Insert or replace a connection; encrypts `secret` with `key`.
///
/// # Errors
/// Returns [`PersistError`] on encrypt or database failure.
pub fn put_connection(
    conn: &Connection,
    key: &VaultKey,
    connection_id: &str,
    provider: &str,
    label: &str,
    secret: &SecretString,
) -> Result<()> {
    let blob = encrypt(key, secret.expose().as_bytes())?;
    conn.execute(
        "INSERT INTO vault_connections (connection_id, provider, label, secret_blob, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(connection_id) DO UPDATE SET
           provider = excluded.provider,
           label = excluded.label,
           secret_blob = excluded.secret_blob,
           updated_at = datetime('now')",
        params![connection_id, provider, label, blob],
    )?;
    Ok(())
}

/// Fetch metadata only (no decrypt).
///
/// # Errors
/// Returns [`PersistError::Sqlite`] on database failure.
pub fn get_connection_meta(
    conn: &Connection,
    connection_id: &str,
) -> Result<Option<ConnectionMeta>> {
    let mut stmt = conn.prepare(
        "SELECT connection_id, provider, label FROM vault_connections WHERE connection_id = ?1",
    )?;
    let mut rows = stmt.query([connection_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(ConnectionMeta {
        connection_id: row.get(0)?,
        provider: row.get(1)?,
        label: row.get(2)?,
    }))
}

/// List metadata ordered by `connection_id` (no secrets).
///
/// # Errors
/// Returns [`PersistError::Sqlite`] on database failure.
pub fn list_connections(conn: &Connection) -> Result<Vec<ConnectionMeta>> {
    let mut stmt = conn.prepare(
        "SELECT connection_id, provider, label FROM vault_connections ORDER BY connection_id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ConnectionMeta {
            connection_id: row.get(0)?,
            provider: row.get(1)?,
            label: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Decrypt the secret for `connection_id`, if present.
///
/// # Errors
/// Returns [`PersistError`] on database or decrypt failure.
pub fn get_connection_secret(
    conn: &Connection,
    key: &VaultKey,
    connection_id: &str,
) -> Result<Option<SecretString>> {
    let mut stmt =
        conn.prepare("SELECT secret_blob FROM vault_connections WHERE connection_id = ?1")?;
    let mut rows = stmt.query([connection_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let blob: Vec<u8> = row.get(0)?;
    let plain = decrypt(key, &blob)?;
    let text = String::from_utf8(plain).map_err(|_| PersistError::Vault(VaultError::Decrypt))?;
    Ok(Some(SecretString::new(text)))
}

/// Delete a connection. Returns whether a row was removed.
///
/// # Errors
/// Returns [`PersistError::Sqlite`] on database failure.
pub fn delete_connection(conn: &Connection, connection_id: &str) -> Result<bool> {
    let n = conn.execute(
        "DELETE FROM vault_connections WHERE connection_id = ?1",
        [connection_id],
    )?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_and_migrate, CONFIG_DIR, DB_PATH};
    use vault::VaultKey;

    fn with_conn(test: impl FnOnce(&Connection, &VaultKey)) {
        let _guard = crate::ENV_LOCK.lock().expect("env lock");
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var(CONFIG_DIR, tmp.path());
        std::env::remove_var(DB_PATH);
        let path = tmp.path().join("broker.db");
        let conn = open_and_migrate(&path).expect("migrate");
        let key = VaultKey::generate();
        test(&conn, &key);
        std::env::remove_var(CONFIG_DIR);
    }

    #[test]
    fn put_get_list_delete_roundtrip() {
        with_conn(|conn, key| {
            let secret = SecretString::new("sk-live-secret-value");
            put_connection(conn, key, "conn-a", "openai", "prod", &secret).expect("put");
            put_connection(
                conn,
                key,
                "conn-b",
                "anthropic",
                "dev",
                &SecretString::new("x"),
            )
            .expect("put b");

            let meta = get_connection_meta(conn, "conn-a")
                .expect("get")
                .expect("some");
            assert_eq!(meta.provider, "openai");
            assert_eq!(meta.label, "prod");
            let meta_dbg = format!("{meta:?}");
            assert!(!meta_dbg.contains("sk-live"));

            let listed = list_connections(conn).expect("list");
            assert_eq!(listed.len(), 2);
            assert_eq!(listed[0].connection_id, "conn-a");

            let revealed = get_connection_secret(conn, key, "conn-a")
                .expect("secret")
                .expect("some");
            assert_eq!(revealed.expose(), "sk-live-secret-value");
            assert!(!format!("{revealed:?}").contains("sk-live"));

            assert!(delete_connection(conn, "conn-a").expect("del"));
            assert!(get_connection_meta(conn, "conn-a").expect("get").is_none());
            assert!(get_connection_secret(conn, key, "conn-a")
                .expect("secret")
                .is_none());
        });
    }

    #[test]
    fn put_replaces_secret() {
        with_conn(|conn, key| {
            put_connection(conn, key, "c1", "ollama", "", &SecretString::new("first"))
                .expect("put");
            put_connection(
                conn,
                key,
                "c1",
                "ollama",
                "local",
                &SecretString::new("second"),
            )
            .expect("replace");
            let meta = get_connection_meta(conn, "c1").expect("get").expect("some");
            assert_eq!(meta.label, "local");
            assert_eq!(
                get_connection_secret(conn, key, "c1")
                    .expect("sec")
                    .expect("some")
                    .expose(),
                "second"
            );
        });
    }
}
