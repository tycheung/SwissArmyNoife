//! Audit append port sketch (`sak070-a` / `sak070-ah`).

use super::{PersistPortError, PortResult};
use std::sync::Mutex;

/// Append-only audit event (sketch).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEventRow {
    pub event_id: String,
    pub binding_id: String,
    pub kind: String,
    pub recorded_at_unix: i64,
}

/// Audit append port — future Postgres impl in **sak070**.
pub trait AuditStore: Send + Sync {
    /// Append one audit event (insert-only).
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the backing store fails.
    fn append_event(&self, row: &AuditEventRow) -> PortResult<()>;
}

/// Test double that always returns [`PersistPortError::NotImplemented`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnimplementedAuditStore;

impl AuditStore for UnimplementedAuditStore {
    fn append_event(&self, _row: &AuditEventRow) -> PortResult<()> {
        Err(PersistPortError::NotImplemented)
    }
}

/// In-memory audit store for port tests (no Postgres) (`sak070-ah`).
#[derive(Debug, Default)]
pub struct MemoryAuditStore {
    events: Mutex<Vec<AuditEventRow>>,
}

impl MemoryAuditStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of appended events (test helper).
    pub fn events(&self) -> PortResult<Vec<AuditEventRow>> {
        let guard = self
            .events
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("audit lock poisoned".into()))?;
        Ok(guard.clone())
    }
}

impl AuditStore for MemoryAuditStore {
    fn append_event(&self, row: &AuditEventRow) -> PortResult<()> {
        let mut guard = self
            .events
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("audit lock poisoned".into()))?;
        guard.push(row.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unimplemented_audit_store_append_event_err() {
        let store = UnimplementedAuditStore;
        let row = AuditEventRow {
            event_id: "evt-1".into(),
            binding_id: "bind-1".into(),
            kind: "invoke".into(),
            recorded_at_unix: 1,
        };
        assert_eq!(
            store.append_event(&row),
            Err(PersistPortError::NotImplemented)
        );
    }

    #[test]
    fn memory_audit_append() {
        let store = MemoryAuditStore::new();
        let row = AuditEventRow {
            event_id: "evt-1".into(),
            binding_id: "bind-1".into(),
            kind: "invoke".into(),
            recorded_at_unix: 9,
        };
        store.append_event(&row).expect("append");
        assert_eq!(store.events().unwrap().len(), 1);
    }
}
