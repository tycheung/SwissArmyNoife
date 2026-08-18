//! Persist invoke audit (`audit_invokes` table).

use rusqlite::{params, Connection};

use crate::Result;

/// One row from `audit_invokes` (already redacted; no secrets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRow {
    pub invoke_id: String,
    pub binding_id: String,
    pub offer_id: String,
    pub status: String,
    pub code: Option<String>,
    pub detail_json: String,
    pub created_at_unix: i64,
}

/// Insert or replace an audit row.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn put_audit(conn: &Connection, row: &AuditRow) -> Result<()> {
    conn.execute(
        "INSERT INTO audit_invokes (
            invoke_id, binding_id, offer_id, status, code, detail_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(invoke_id) DO UPDATE SET
           binding_id = excluded.binding_id,
           offer_id = excluded.offer_id,
           status = excluded.status,
           code = excluded.code,
           detail_json = excluded.detail_json,
           created_at = excluded.created_at",
        params![
            row.invoke_id,
            row.binding_id,
            row.offer_id,
            row.status,
            row.code,
            row.detail_json,
            row.created_at_unix.to_string()
        ],
    )?;
    Ok(())
}

/// List audit rows in append order (caller maps to control events).
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn list_audit(conn: &Connection) -> Result<Vec<AuditRow>> {
    let mut stmt = conn.prepare(
        "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at
         FROM audit_invokes ORDER BY created_at ASC, invoke_id ASC",
    )?;
    let mapped = stmt.query_map([], row_from_sql)?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditRow> {
    let created: Option<String> = row.get(6)?;
    let created_at_unix = created
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    Ok(AuditRow {
        invoke_id: row.get(0)?,
        binding_id: row.get(1)?,
        offer_id: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        status: row.get(3)?,
        code: row.get(4)?,
        detail_json: row.get(5)?,
        created_at_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_and_migrate, CONFIG_DIR, DB_PATH};

    #[test]
    fn put_list_survives_reopen() {
        let tmp = tempfile::tempdir().expect("tmp");
        let _guard = crate::ENV_LOCK.lock().expect("env lock");
        std::env::set_var(CONFIG_DIR, tmp.path());
        std::env::remove_var(DB_PATH);
        let path = tmp.path().join("broker.db");
        let row = AuditRow {
            invoke_id: "00000000-0000-0000-0000-000000000002".into(),
            binding_id: "00000000-0000-0000-0000-000000000001".into(),
            offer_id: "llm.chat".into(),
            status: "ok".into(),
            code: None,
            detail_json: r#"{"args":{"q":"hi"}}"#.into(),
            created_at_unix: 1_700_000_000,
        };
        {
            let conn = open_and_migrate(&path).expect("migrate");
            crate::put_binding(
                &conn,
                &crate::BindingRow {
                    binding_id: row.binding_id.clone(),
                    offer_id: row.offer_id.clone(),
                    principal: "local".into(),
                    policy_json: "{}".into(),
                    expires_at_unix: 4_000_000_000,
                },
            )
            .expect("binding fk");
            put_audit(&conn, &row).expect("put");
        }
        let conn = open_and_migrate(&path).expect("reopen");
        let list = list_audit(&conn).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].offer_id, "llm.chat");
        assert_eq!(list[0].status, "ok");
        assert!(!list[0].detail_json.contains("sk-"));
        std::env::remove_var(CONFIG_DIR);
        std::env::remove_var(DB_PATH);
    }
}
