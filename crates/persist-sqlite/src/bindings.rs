//! Persist MCP bindings (`bindings` table).

use rusqlite::{params, Connection};

use crate::Result;

/// One row from `bindings` (no secrets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingRow {
    pub binding_id: String,
    pub offer_id: String,
    pub principal: String,
    pub policy_json: String,
    pub expires_at_unix: i64,
}

/// Insert or replace a binding. Upserts the catalog offer first (FK).
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn put_binding(conn: &Connection, row: &BindingRow) -> Result<()> {
    crate::upsert_offer(conn, &row.offer_id, "0.1.0", "core")?;
    conn.execute(
        "INSERT INTO bindings (binding_id, offer_id, principal, policy_json, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(binding_id) DO UPDATE SET
           offer_id = excluded.offer_id,
           principal = excluded.principal,
           policy_json = excluded.policy_json,
           expires_at = excluded.expires_at",
        params![
            row.binding_id,
            row.offer_id,
            row.principal,
            row.policy_json,
            row.expires_at_unix.to_string()
        ],
    )?;
    Ok(())
}

/// Fetch one binding by id.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn get_binding(conn: &Connection, binding_id: &str) -> Result<Option<BindingRow>> {
    let mut stmt = conn.prepare(
        "SELECT binding_id, offer_id, principal, policy_json, expires_at
         FROM bindings WHERE binding_id = ?1",
    )?;
    let mut rows = stmt.query([binding_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(row_from_sql(row)?))
}

/// List all bindings (caller filters TTL).
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn list_bindings(conn: &Connection) -> Result<Vec<BindingRow>> {
    let mut stmt = conn.prepare(
        "SELECT binding_id, offer_id, principal, policy_json, expires_at
         FROM bindings ORDER BY binding_id ASC",
    )?;
    let mapped = stmt.query_map([], row_from_sql)?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

/// Delete a binding. Returns whether a row was removed.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn delete_binding(conn: &Connection, binding_id: &str) -> Result<bool> {
    let n = conn.execute("DELETE FROM bindings WHERE binding_id = ?1", [binding_id])?;
    Ok(n > 0)
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<BindingRow> {
    let expires: Option<String> = row.get(4)?;
    let expires_at_unix = expires
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    Ok(BindingRow {
        binding_id: row.get(0)?,
        offer_id: row.get(1)?,
        principal: row.get(2)?,
        policy_json: row.get(3)?,
        expires_at_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_and_migrate, CONFIG_DIR, DB_PATH};

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
    fn put_get_survives_reopen() {
        let tmp = tempfile::tempdir().expect("tmp");
        let _guard = crate::ENV_LOCK.lock().expect("env lock");
        std::env::set_var(CONFIG_DIR, tmp.path());
        std::env::remove_var(DB_PATH);
        let path = tmp.path().join("broker.db");
        let row = BindingRow {
            binding_id: "00000000-0000-0000-0000-000000000001".into(),
            offer_id: "llm.embed".into(),
            principal: "local".into(),
            policy_json: "{}".into(),
            expires_at_unix: 4_000_000_000,
        };
        {
            let conn = open_and_migrate(&path).expect("migrate");
            put_binding(&conn, &row).expect("put");
        }
        let conn = open_and_migrate(&path).expect("reopen");
        let got = get_binding(&conn, &row.binding_id)
            .expect("get")
            .expect("row");
        assert_eq!(got.offer_id, "llm.embed");
        assert_eq!(got.principal, "local");
        assert_eq!(list_bindings(&conn).expect("list").len(), 1);
        assert!(delete_binding(&conn, &row.binding_id).expect("del"));
        std::env::remove_var(CONFIG_DIR);
        std::env::remove_var(DB_PATH);
    }

    #[test]
    fn missing_get_is_none() {
        with_conn(|conn| {
            assert!(get_binding(conn, "missing").expect("get").is_none());
        });
    }
}
