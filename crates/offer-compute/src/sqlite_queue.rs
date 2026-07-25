//! SQLite-backed work queue (`sak291-b`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use types::ErrorCode;
use uuid::Uuid;

use crate::merge::MergeHook;
use crate::node::NodeId;
use crate::queue::{WorkId, WorkQueue, WorkStatus, WorkUnit};
use crate::sanitize::sanitize_payload;

/// Durable FIFO queue in `broker.db` / path.
pub struct SqliteQueue {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl SqliteQueue {
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

impl WorkQueue for SqliteQueue {
    fn enqueue(&self, kind: &str, payload: Value) -> Result<WorkUnit, ErrorCode> {
        let unit = WorkUnit {
            id: WorkId::new(),
            kind: kind.to_owned(),
            payload: sanitize_payload(payload),
            status: WorkStatus::Queued,
            claimed_by: None,
            result: None,
        };
        let now = unix_now();
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM work_units",
                [],
                |r| r.get(0),
            )
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.execute(
            "INSERT INTO work_units (id, kind, payload_json, status, claimed_by, result_json, created_at, seq)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6)",
            params![
                unit.id.to_string(),
                unit.kind,
                unit.payload.to_string(),
                unit.status.as_str(),
                now as i64,
                seq,
            ],
        )
        .map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(unit)
    }

    fn claim(&self, node: NodeId) -> Result<WorkUnit, ErrorCode> {
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let row: Option<(String, String, String, i64)> = conn
            .query_row(
                "SELECT id, kind, payload_json, seq FROM work_units
                 WHERE status = 'queued' ORDER BY seq ASC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        let Some((id, kind, payload_json, _seq)) = row else {
            return Err(ErrorCode::OfferNotFound);
        };
        let n = conn
            .execute(
                "UPDATE work_units SET status = 'claimed', claimed_by = ?1
                 WHERE id = ?2 AND status = 'queued'",
                params![node.to_string(), id],
            )
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        if n != 1 {
            return Err(ErrorCode::OfferNotFound);
        }
        let payload: Value =
            serde_json::from_str(&payload_json).map_err(|_| ErrorCode::SchemaInvalid)?;
        let work_id =
            WorkId::from_uuid(Uuid::parse_str(&id).map_err(|_| ErrorCode::SchemaInvalid)?);
        Ok(WorkUnit {
            id: work_id,
            kind,
            payload,
            status: WorkStatus::Claimed,
            claimed_by: Some(node),
            result: None,
        })
    }

    fn complete(
        &self,
        work_id: WorkId,
        node: NodeId,
        result: Value,
        merge: &dyn MergeHook,
    ) -> Result<WorkUnit, ErrorCode> {
        let mut unit = self.get(work_id)?;
        if unit.status != WorkStatus::Claimed || unit.claimed_by != Some(node) {
            return Err(ErrorCode::PolicyDenied);
        }
        let merged = merge.merge(&unit.payload, &result)?;
        let sanitized = sanitize_payload(merged);
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.execute(
            "UPDATE work_units SET status = 'completed', result_json = ?1
             WHERE id = ?2 AND status = 'claimed' AND claimed_by = ?3",
            params![sanitized.to_string(), work_id.to_string(), node.to_string()],
        )
        .map_err(|_| ErrorCode::SchemaInvalid)?;
        unit.status = WorkStatus::Completed;
        unit.result = Some(sanitized);
        Ok(unit)
    }

    fn get(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode> {
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let row: (String, String, String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT kind, payload_json, status, claimed_by, result_json FROM work_units WHERE id = ?1",
                params![work_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| ErrorCode::OfferNotFound)?;
        row_to_unit(work_id, row)
    }

    fn list(&self, limit: usize) -> Result<Vec<WorkUnit>, ErrorCode> {
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, payload_json, status, claimed_by, result_json FROM work_units
                 ORDER BY seq DESC LIMIT ?1",
            )
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind, payload_json, status, claimed_by, result_json) =
                row.map_err(|_| ErrorCode::SchemaInvalid)?;
            let work_id =
                WorkId::from_uuid(Uuid::parse_str(&id).map_err(|_| ErrorCode::SchemaInvalid)?);
            out.push(row_to_unit(
                work_id,
                (kind, payload_json, status, claimed_by, result_json),
            )?);
        }
        Ok(out)
    }

    fn requeue(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode> {
        let unit = self.get(work_id)?;
        if unit.status != WorkStatus::Claimed && unit.status != WorkStatus::Failed {
            return Err(ErrorCode::PolicyDenied);
        }
        let conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let n = conn
            .execute(
                "UPDATE work_units SET status = 'queued', claimed_by = NULL, result_json = NULL
                 WHERE id = ?1 AND status IN ('claimed', 'failed')",
                params![work_id.to_string()],
            )
            .map_err(|_| ErrorCode::SchemaInvalid)?;
        if n != 1 {
            return Err(ErrorCode::OfferNotFound);
        }
        Ok(WorkUnit {
            id: work_id,
            kind: unit.kind,
            payload: unit.payload,
            status: WorkStatus::Queued,
            claimed_by: None,
            result: None,
        })
    }
}

fn row_to_unit(
    id: WorkId,
    row: (String, String, String, Option<String>, Option<String>),
) -> Result<WorkUnit, ErrorCode> {
    let (kind, payload_json, status, claimed_by, result_json) = row;
    let payload: Value =
        serde_json::from_str(&payload_json).map_err(|_| ErrorCode::SchemaInvalid)?;
    let result = match result_json {
        Some(s) if !s.is_empty() => {
            Some(serde_json::from_str(&s).map_err(|_| ErrorCode::SchemaInvalid)?)
        }
        _ => None,
    };
    let claimed_by = match claimed_by {
        Some(s) if !s.is_empty() => {
            let u = Uuid::parse_str(&s).map_err(|_| ErrorCode::SchemaInvalid)?;
            Some(NodeId::from_uuid(u))
        }
        _ => None,
    };
    Ok(WorkUnit {
        id,
        kind,
        payload,
        status: WorkStatus::parse(&status)?,
        claimed_by,
        result,
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
    use crate::merge::IdentityMerge;
    use serde_json::json;

    #[test]
    fn sqlite_enqueue_claim_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("w.db");
        let q = SqliteQueue::open(&path).unwrap();
        let node = NodeId::new();
        let u = q.enqueue("echo", json!({"api_key": "x"})).unwrap();
        assert_eq!(u.payload["api_key"], "[REDACTED]");
        let claimed = q.claim(node).unwrap();
        assert_eq!(claimed.id, u.id);
        let done = q
            .complete(u.id, node, json!({"ok": true}), &IdentityMerge)
            .unwrap();
        assert_eq!(done.status, WorkStatus::Completed);
        assert_eq!(q.list(10).unwrap().len(), 1);
    }
}
