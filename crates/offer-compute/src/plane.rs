//! Shared compute mesh state for offers / worker / MCP.

use std::sync::Arc;

use crate::memory_queue::MemoryQueue;
use crate::merge::{IdentityMerge, MergeHook};
use crate::node::{NodeRegistry, NodeStore};
use crate::queue::WorkQueue;
use crate::redis_queue::RedisQueue;
use crate::sqlite_nodes::SqliteNodeRegistry;
use crate::sqlite_queue::SqliteQueue;
use types::ErrorCode;

/// Process-local compute plane (registry + queue + merge).
pub struct ComputePlane {
    pub nodes: Arc<dyn NodeStore>,
    pub queue: Arc<dyn WorkQueue>,
    pub merge: Arc<dyn MergeHook>,
}

impl ComputePlane {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(NodeRegistry::new()),
            queue: Arc::new(MemoryQueue::new()),
            merge: Arc::new(IdentityMerge),
        }
    }

    #[must_use]
    pub fn with_merge(merge: Arc<dyn MergeHook>) -> Self {
        Self {
            nodes: Arc::new(NodeRegistry::new()),
            queue: Arc::new(MemoryQueue::new()),
            merge,
        }
    }

    /// SQLite durable nodes + queue (`sak291-c` / `sak425-a` / `sak426-b`).
    ///
    /// # Errors
    /// DB open/migrate failures.
    pub fn with_sqlite_queue(path: &std::path::Path) -> Result<Self, ErrorCode> {
        Ok(Self {
            nodes: Arc::new(SqliteNodeRegistry::open(path)?),
            queue: Arc::new(SqliteQueue::open(path)?),
            merge: Arc::new(IdentityMerge),
        })
    }

    /// Open default broker DB path (`sak427-a`).
    ///
    /// # Errors
    /// Same as [`Self::with_sqlite_queue`].
    pub fn open_default_sqlite() -> Result<Self, ErrorCode> {
        Self::with_sqlite_queue(&persist_sqlite::db_path())
    }

    /// Redis / FakeRedis queue with durable sqlite nodes when possible (`sak427-e`).
    #[must_use]
    pub fn with_redis_queue(queue: RedisQueue) -> Self {
        let nodes: Arc<dyn NodeStore> = match SqliteNodeRegistry::open_default() {
            Ok(reg) => Arc::new(reg),
            Err(_) => Arc::new(NodeRegistry::new()),
        };
        Self {
            nodes,
            queue: Arc::new(queue),
            merge: Arc::new(IdentityMerge),
        }
    }

    /// `COMPUTE_QUEUE=sqlite|redis|memory` — default **sqlite** (`sak427-b`).
    ///
    /// Redis: `REDIS_URL` + `--features redis` → live; else in-process [`RedisQueue::fake`].
    /// SQLite: durable nodes + queue share `broker.db` with HTTP admin.
    pub fn from_env() -> Result<Self, ErrorCode> {
        let mode = std::env::var("COMPUTE_QUEUE").unwrap_or_else(|_| "sqlite".into());
        if mode.eq_ignore_ascii_case("memory") {
            Ok(Self::new())
        } else if mode.eq_ignore_ascii_case("redis") {
            let queue = match RedisQueue::from_env() {
                Ok(q) => q,
                Err(_) => RedisQueue::fake(),
            };
            Ok(Self::with_redis_queue(queue))
        } else {
            // sqlite (explicit or default)
            Self::open_default_sqlite()
        }
    }
}

impl Default for ComputePlane {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_redis_uses_fake_without_url() {
        std::env::set_var("COMPUTE_QUEUE", "redis");
        std::env::remove_var("REDIS_URL");
        let plane = ComputePlane::from_env().expect("plane");
        let unit = plane
            .queue
            .enqueue("t", serde_json::json!({"x": 1}))
            .expect("enq");
        assert_eq!(unit.kind, "t");
        std::env::remove_var("COMPUTE_QUEUE");
    }

    #[test]
    fn from_env_default_is_sqlite() {
        std::env::remove_var("COMPUTE_QUEUE");
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("DB_PATH", dir.path().join("broker.db"));
        let plane = ComputePlane::from_env().expect("plane");
        let n = plane
            .nodes
            .register("def", vec!["echo".into()], None)
            .unwrap();
        assert_eq!(plane.nodes.list(None).unwrap().len(), 1);
        let plane2 = ComputePlane::from_env().expect("plane2");
        assert_eq!(plane2.nodes.list(None).unwrap()[0].id, n.id);
        std::env::remove_var("DB_PATH");
    }

    #[test]
    fn from_env_memory_explicit() {
        std::env::set_var("COMPUTE_QUEUE", "memory");
        let plane = ComputePlane::from_env().expect("plane");
        let _ = plane.nodes.register("m", vec![], None).unwrap();
        // Re-open does not share memory nodes.
        let plane2 = ComputePlane::from_env().expect("plane2");
        assert!(plane2.nodes.list(None).unwrap().is_empty());
        std::env::remove_var("COMPUTE_QUEUE");
    }

    #[test]
    fn with_sqlite_queue_uses_durable_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broker.db");
        let plane = ComputePlane::with_sqlite_queue(&path).expect("plane");
        let n = plane
            .nodes
            .register("sql-node", vec!["echo".into()], None)
            .unwrap();
        let listed = plane.nodes.list(None).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, n.id);
        let plane2 = ComputePlane::with_sqlite_queue(&path).expect("plane2");
        assert_eq!(plane2.nodes.list(None).unwrap().len(), 1);
    }
}
