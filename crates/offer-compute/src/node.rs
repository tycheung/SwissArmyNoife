//! Node registry register / heartbeat (`sak290`).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use types::ErrorCode;
use uuid::Uuid;

/// Stable worker/node identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Registered compute node metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: NodeId,
    pub label: String,
    pub caps: Vec<String>,
    pub last_heartbeat_unix: u64,
    /// Optional collaborative session scope (`sak423-d`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Port for memory / SQLite node registries (`sak426-a`).
pub trait NodeStore: Send + Sync {
    /// Register without session scope.
    fn register(
        &self,
        label: &str,
        caps: Vec<String>,
        id: Option<NodeId>,
    ) -> Result<NodeRecord, ErrorCode>;

    /// Register with optional session scope.
    fn register_scoped(
        &self,
        label: &str,
        caps: Vec<String>,
        id: Option<NodeId>,
        session_id: Option<String>,
    ) -> Result<NodeRecord, ErrorCode>;

    /// Touch heartbeat for an existing node.
    fn heartbeat(&self, id: NodeId) -> Result<NodeRecord, ErrorCode>;

    /// List nodes; optionally drop those older than `stale_after`.
    fn list(&self, stale_after: Option<Duration>) -> Result<Vec<NodeRecord>, ErrorCode>;

    /// List nodes with optional session filter.
    fn list_filtered(
        &self,
        stale_after: Option<Duration>,
        session_id: Option<&str>,
    ) -> Result<Vec<NodeRecord>, ErrorCode>;
}

/// In-memory node registry (`sak290-b`).
#[derive(Debug, Default)]
pub struct NodeRegistry {
    nodes: Mutex<HashMap<NodeId, NodeRecord>>,
}

impl NodeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl NodeStore for NodeRegistry {
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
        let mut map = self.nodes.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let id = id.unwrap_or_else(NodeId::new);
        let session_id = session_id.filter(|s| !s.is_empty());
        let record = NodeRecord {
            id,
            label: label.to_owned(),
            caps,
            last_heartbeat_unix: now,
            session_id,
        };
        map.insert(id, record.clone());
        Ok(record)
    }

    fn heartbeat(&self, id: NodeId) -> Result<NodeRecord, ErrorCode> {
        let mut map = self.nodes.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let rec = map.get_mut(&id).ok_or(ErrorCode::OfferNotFound)?;
        rec.last_heartbeat_unix = unix_now();
        Ok(rec.clone())
    }

    fn list(&self, stale_after: Option<Duration>) -> Result<Vec<NodeRecord>, ErrorCode> {
        self.list_filtered(stale_after, None)
    }

    fn list_filtered(
        &self,
        stale_after: Option<Duration>,
        session_id: Option<&str>,
    ) -> Result<Vec<NodeRecord>, ErrorCode> {
        let map = self.nodes.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let now = unix_now();
        let mut out: Vec<_> = map
            .values()
            .filter(|n| match stale_after {
                Some(d) => now.saturating_sub(n.last_heartbeat_unix) <= d.as_secs(),
                None => true,
            })
            .filter(|n| match session_id {
                Some(sid) if !sid.is_empty() => n.session_id.as_deref() == Some(sid),
                _ => true,
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| a.label.cmp(&b.label).then(a.id.0.cmp(&b.id.0)));
        Ok(out)
    }
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
    fn register_heartbeat_list() {
        let reg = NodeRegistry::new();
        let n = NodeStore::register(&reg, "worker-a", vec!["echo".into()], None).unwrap();
        let hb = NodeStore::heartbeat(&reg, n.id).unwrap();
        assert!(hb.last_heartbeat_unix >= n.last_heartbeat_unix);
        assert_eq!(NodeStore::list(&reg, None).unwrap().len(), 1);
    }
}
