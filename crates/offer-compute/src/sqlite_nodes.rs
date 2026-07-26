//! SQLite-backed node registry (`sak425-a`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use types::ErrorCode;
use uuid::Uuid;

use crate::node::{NodeId, NodeRecord, NodeStore};

/// Durable node registry in `broker.db`.
pub struct SqliteNodeRegistry {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl SqliteNodeRegistry {
    /// Open (or create) DB and ensure migrations applied.
    ///
    /// # Errors
    /// Persist / I/O failures → schema invalid.
    pub fn open(path: &Path) -> Result<Self, ErrorCode> {
        let conn = persist_sqlite::open_and_migrate(path).map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    /// Open default broker DB path.
    ///
    /// # Errors
    /// Same as [`Self::open`].
    pub fn open_default() -> Result<Self, ErrorCode> {
        Self::open(&persist_sqlite::db_path())
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl NodeStore for SqliteNodeRegistry {
    fn register(
        &self,
        label: &str,
        caps: Vec<String>,
        id: Option<NodeId>,
    ) -> Result<NodeRecord, ErrorCode> {
        self.register_scoped(label, caps, id, None)
    }

    fn register_scoped(
        &self,
        label: &str,
        caps: Vec<String>,
        id: Option<NodeId>,
        session_id: Option<String>,
    ) -> Result<NodeRecord, ErrorCode> {
        let now = unix_now();
        let id = id.unwrap_or_default();
        let session_id = session_id.filter(|s| !s.is_empty());
        let record = NodeRecord {
            id,
            label: label.to_owned(),
            caps,
            last_heartbeat_unix: now,
            session_id: session_id.clone(),
        };
        let caps_json =
            serde_json::to_string(&record.caps).map_err(|_| ErrorCode::SchemaInvalid)?;
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.execute(
            "INSERT INTO compute_nodes (id, label, caps_json, last_heartbeat_unix, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               label = excluded.label,
               caps_json = excluded.caps_json,
               last_heartbeat_unix = excluded.last_heartbeat_unix,
               session_id = excluded.session_id",
            params![
                record.id.to_string(),
                record.label,
                caps_json,
                i64::try_from(now).unwrap_or(i64::MAX),
                session_id,
            ],
        )
        .map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(record)
    }

    fn heartbeat(&self, id: NodeId) -> Result<NodeRecord, ErrorCode> {
        let now = unix_now();
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let n = conn
            .execute(
                "UPDATE compute_nodes SET last_heartbeat_unix = ?1 WHERE id = ?2",
                params![i64::try_from(now).unwrap_or(i64::MAX), id.to_string()],
            )
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        if n == 0 {
            return Err(ErrorCode::OfferNotFound);
        }
        load_node(&conn, id)
    }

    fn list_filtered(
        &self,
        stale_after: Option<Duration>,
        session_id: Option<&str>,
    ) -> Result<Vec<NodeRecord>, ErrorCode> {
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, label, caps_json, last_heartbeat_unix, session_id
                 FROM compute_nodes ORDER BY label ASC, id ASC",
            )
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        let now = unix_now();
        let mut out = Vec::new();
        for row in rows {
            let (id, label, caps_json, hb, session) = row.map_err(|_| ErrorCode::SchemaInvalid)?;
            let rec = row_to_record(&id, label, &caps_json, hb, session)?;
            if let Some(d) = stale_after {
                if now.saturating_sub(rec.last_heartbeat_unix) > d.as_secs() {
                    continue;
                }
            }
            if let Some(sid) = session_id.filter(|s| !s.is_empty()) {
                if rec.session_id.as_deref() != Some(sid) {
                    continue;
                }
            }
            out.push(rec);
        }
        Ok(out)
    }

    fn list(&self, stale_after: Option<Duration>) -> Result<Vec<NodeRecord>, ErrorCode> {
        self.list_filtered(stale_after, None)
    }
}

fn load_node(conn: &Connection, id: NodeId) -> Result<NodeRecord, ErrorCode> {
    let row: (String, String, String, i64, Option<String>) = conn
        .query_row(
            "SELECT id, label, caps_json, last_heartbeat_unix, session_id
             FROM compute_nodes WHERE id = ?1",
            params![id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|_| ErrorCode::OfferNotFound)?;
    row_to_record(&row.0, row.1, &row.2, row.3, row.4)
}

fn row_to_record(
    id: &str,
    label: String,
    caps_json: &str,
    hb: i64,
    session: Option<String>,
) -> Result<NodeRecord, ErrorCode> {
    let uuid = Uuid::parse_str(id).map_err(|_| ErrorCode::SchemaInvalid)?;
    let caps: Vec<String> =
        serde_json::from_str(caps_json).map_err(|_| ErrorCode::SchemaInvalid)?;
    Ok(NodeRecord {
        id: NodeId::from_uuid(uuid),
        label,
        caps,
        last_heartbeat_unix: u64::try_from(hb).unwrap_or(0),
        session_id: session.filter(|s| !s.is_empty()),
    })
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_heartbeat_list_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broker.db");
        let reg = SqliteNodeRegistry::open(&path).unwrap();
        let n = reg
            .register_scoped("w1", vec!["mesh_worker".into()], None, Some("s1".into()))
            .unwrap();
        let beat = NodeStore::heartbeat(&reg, n.id).unwrap();
        assert!(beat.last_heartbeat_unix >= n.last_heartbeat_unix);
        let listed = NodeStore::list_filtered(&reg, None, Some("s1")).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, n.id);
        assert!(NodeStore::list_filtered(&reg, None, Some("other"))
            .unwrap()
            .is_empty());
    }
}
