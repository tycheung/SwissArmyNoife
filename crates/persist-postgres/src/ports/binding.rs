//! Binding persistence port sketch (`sak070-a` / `sak070-ag`).

use super::{PersistPortError, PortResult};
use std::collections::HashMap;
use std::sync::Mutex;

/// Binding row persisted for active broker sessions (sketch).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingRow {
    pub binding_id: String,
    pub offer_id: String,
    pub created_at_unix: i64,
}

/// Binding persistence port — future Postgres impl in **sak070**.
pub trait BindingStore: Send + Sync {
    /// Persist a new binding row.
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the backing store fails.
    fn insert_binding(&self, row: &BindingRow) -> PortResult<()>;

    /// Load a binding by id.
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the backing store fails.
    fn get_binding(&self, binding_id: &str) -> PortResult<Option<BindingRow>>;
}

/// Test double that always returns [`PersistPortError::NotImplemented`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnimplementedBindingStore;

impl BindingStore for UnimplementedBindingStore {
    fn insert_binding(&self, _row: &BindingRow) -> PortResult<()> {
        Err(PersistPortError::NotImplemented)
    }

    fn get_binding(&self, _binding_id: &str) -> PortResult<Option<BindingRow>> {
        Err(PersistPortError::NotImplemented)
    }
}

/// In-memory binding store for port tests (no Postgres) (`sak070-ag`).
#[derive(Debug, Default)]
pub struct MemoryBindingStore {
    rows: Mutex<HashMap<String, BindingRow>>,
}

impl MemoryBindingStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl BindingStore for MemoryBindingStore {
    fn insert_binding(&self, row: &BindingRow) -> PortResult<()> {
        let mut map = self
            .rows
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("binding lock poisoned".into()))?;
        map.insert(row.binding_id.clone(), row.clone());
        Ok(())
    }

    fn get_binding(&self, binding_id: &str) -> PortResult<Option<BindingRow>> {
        let map = self
            .rows
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("binding lock poisoned".into()))?;
        Ok(map.get(binding_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unimplemented_binding_store_all_methods_err() {
        let store = UnimplementedBindingStore;
        let row = BindingRow {
            binding_id: "bind-1".into(),
            offer_id: "llm.chat".into(),
            created_at_unix: 1,
        };
        assert_eq!(
            store.insert_binding(&row),
            Err(PersistPortError::NotImplemented)
        );
        assert_eq!(
            store.get_binding("bind-1"),
            Err(PersistPortError::NotImplemented)
        );
    }

    #[test]
    fn memory_binding_roundtrip() {
        let store = MemoryBindingStore::new();
        let row = BindingRow {
            binding_id: "bind-1".into(),
            offer_id: "llm.chat".into(),
            created_at_unix: 42,
        };
        store.insert_binding(&row).expect("insert");
        let got = store.get_binding("bind-1").expect("get").expect("some");
        assert_eq!(got.offer_id, "llm.chat");
        assert_eq!(got.created_at_unix, 42);
    }
}
