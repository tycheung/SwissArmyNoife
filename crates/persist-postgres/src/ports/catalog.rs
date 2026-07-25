//! Catalog persistence port — mirrors `persist-sqlite::catalog` (`sak070-b` / `sak070-ae`).

use super::{PersistPortError, PortResult};
use std::collections::HashMap;
use std::sync::Mutex;

/// Row shape for catalog offers (mirrors `persist-sqlite::catalog::CatalogOfferRow`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogOfferRow {
    pub offer_id: String,
    pub version: String,
    pub origin: String,
}

/// Catalog persistence port — future Postgres impl in **sak070**.
pub trait CatalogStore: Send + Sync {
    /// Insert or replace a catalog offer.
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the backing store fails.
    fn upsert_offer(&self, offer_id: &str, version: &str, origin: &str) -> PortResult<()>;

    /// Fetch one offer by id.
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the backing store fails.
    fn get_offer(&self, offer_id: &str) -> PortResult<Option<CatalogOfferRow>>;

    /// List all offers ordered by id.
    ///
    /// # Errors
    /// Returns [`PersistPortError`] when the backing store fails.
    fn list_offers(&self) -> PortResult<Vec<CatalogOfferRow>>;
}

/// Test double that always returns [`PersistPortError::NotImplemented`] (`sak070-b`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnimplementedCatalog;

impl CatalogStore for UnimplementedCatalog {
    fn upsert_offer(&self, _offer_id: &str, _version: &str, _origin: &str) -> PortResult<()> {
        Err(PersistPortError::NotImplemented)
    }

    fn get_offer(&self, _offer_id: &str) -> PortResult<Option<CatalogOfferRow>> {
        Err(PersistPortError::NotImplemented)
    }

    fn list_offers(&self) -> PortResult<Vec<CatalogOfferRow>> {
        Err(PersistPortError::NotImplemented)
    }
}

/// In-memory catalog for port tests (no Postgres) (`sak070-ae`).
#[derive(Debug, Default)]
pub struct MemoryCatalog {
    offers: Mutex<HashMap<String, CatalogOfferRow>>,
}

impl MemoryCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CatalogStore for MemoryCatalog {
    fn upsert_offer(&self, offer_id: &str, version: &str, origin: &str) -> PortResult<()> {
        let mut map = self
            .offers
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("catalog lock poisoned".into()))?;
        map.insert(
            offer_id.to_string(),
            CatalogOfferRow {
                offer_id: offer_id.to_string(),
                version: version.to_string(),
                origin: origin.to_string(),
            },
        );
        Ok(())
    }

    fn get_offer(&self, offer_id: &str) -> PortResult<Option<CatalogOfferRow>> {
        let map = self
            .offers
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("catalog lock poisoned".into()))?;
        Ok(map.get(offer_id).cloned())
    }

    fn list_offers(&self) -> PortResult<Vec<CatalogOfferRow>> {
        let map = self
            .offers
            .lock()
            .map_err(|_| PersistPortError::InvalidConfig("catalog lock poisoned".into()))?;
        let mut rows: Vec<_> = map.values().cloned().collect();
        rows.sort_by(|a, b| a.offer_id.cmp(&b.offer_id));
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_offer_row_equality() {
        let row = CatalogOfferRow {
            offer_id: "llm.chat".into(),
            version: "0.1.0".into(),
            origin: "core".into(),
        };
        assert_eq!(row, row.clone());
    }

    #[test]
    fn unimplemented_catalog_all_methods_err() {
        let store = UnimplementedCatalog;
        assert_eq!(
            store.upsert_offer("sandbox.exec", "0.1.0", "core"),
            Err(PersistPortError::NotImplemented)
        );
        assert_eq!(
            store.get_offer("sandbox.exec"),
            Err(PersistPortError::NotImplemented)
        );
        assert_eq!(store.list_offers(), Err(PersistPortError::NotImplemented));
    }

    #[test]
    fn memory_catalog_roundtrip() {
        let store = MemoryCatalog::new();
        store
            .upsert_offer("llm.chat", "0.1.0", "core")
            .expect("upsert");
        let got = store.get_offer("llm.chat").expect("get").expect("some");
        assert_eq!(got.version, "0.1.0");
        assert_eq!(store.list_offers().unwrap().len(), 1);
    }
}
