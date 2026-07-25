//! In-memory offer catalog (`catalog.list` / `catalog.get`).

use std::collections::BTreeMap;

use types::{ErrorCode, OfferId};

use crate::CatalogEntry;

/// Process-local catalog of available offers.
#[derive(Clone, Debug, Default)]
pub struct CatalogRegistry {
    /// Keyed by offer id string for stable ordered listing.
    entries: BTreeMap<String, CatalogEntry>,
}

impl CatalogRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace an entry.
    pub fn register(&mut self, entry: CatalogEntry) {
        let key = entry.id.as_str().to_owned();
        self.entries.insert(key, entry);
    }

    /// Register from an [`crate::Offer`]'s catalog metadata.
    pub fn register_offer(&mut self, offer: &impl crate::Offer) {
        self.register(offer.catalog_entry().clone());
    }

    /// Lookup by offer id.
    ///
    /// # Errors
    /// Returns [`ErrorCode::OfferNotFound`] when the id is not registered.
    pub fn get(&self, id: &OfferId) -> Result<&CatalogEntry, ErrorCode> {
        self.entries
            .get(id.as_str())
            .ok_or(ErrorCode::OfferNotFound)
    }

    /// All entries in offer-id order.
    #[must_use]
    pub fn list(&self) -> Vec<&CatalogEntry> {
        self.entries.values().collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_list_get_roundtrip() {
        let mut cat = CatalogRegistry::new();
        assert!(cat.is_empty());
        cat.register(CatalogEntry::new("llm.chat", "0.1.0").expect("valid"));
        cat.register(CatalogEntry::new("sandbox.exec", "0.1.0").expect("valid"));
        assert_eq!(cat.len(), 2);

        let ids: Vec<_> = cat.list().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["llm.chat", "sandbox.exec"]);

        let offer_id = OfferId::new("llm.chat").expect("valid");
        let entry = cat.get(&offer_id).expect("found");
        assert_eq!(entry.version, "0.1.0");
    }

    #[test]
    fn get_missing_is_offer_not_found() {
        let cat = CatalogRegistry::new();
        let offer_id = OfferId::new("missing.offer").expect("valid");
        assert_eq!(cat.get(&offer_id), Err(ErrorCode::OfferNotFound));
    }

    #[test]
    fn register_replaces_same_id() {
        let mut cat = CatalogRegistry::new();
        cat.register(CatalogEntry::new("llm.chat", "0.1.0").expect("v1"));
        cat.register(CatalogEntry::new("llm.chat", "0.2.0").expect("v2"));
        assert_eq!(cat.len(), 1);
        let offer_id = OfferId::new("llm.chat").expect("valid");
        assert_eq!(cat.get(&offer_id).expect("found").version, "0.2.0");
    }
}
