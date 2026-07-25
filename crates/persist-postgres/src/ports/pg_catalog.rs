//! Live Postgres [`CatalogStore`] (`sak070` Phase B).

use super::{CatalogOfferRow, CatalogStore, PersistPortError, PortResult};
use crate::pool::PoolHandle;
use crate::{
    catalog_offer_by_id_select_sql, catalog_offer_upsert_sql, catalog_offers_list_select_sql,
};
use std::sync::Arc;

/// Catalog adapter backed by a connected [`PoolHandle`].
#[derive(Debug, Clone)]
pub struct PostgresCatalog {
    pool: Arc<PoolHandle>,
}

impl PostgresCatalog {
    /// Wrap a connected pool handle.
    #[must_use]
    pub fn new(pool: Arc<PoolHandle>) -> Self {
        Self { pool }
    }

    /// Shared pool handle.
    #[must_use]
    pub fn pool(&self) -> &Arc<PoolHandle> {
        &self.pool
    }
}

impl CatalogStore for PostgresCatalog {
    fn upsert_offer(&self, offer_id: &str, version: &str, origin: &str) -> PortResult<()> {
        let descriptor = "{}";
        self.pool
            .execute_params(
                catalog_offer_upsert_sql(),
                &[&offer_id, &version, &origin, &descriptor],
            )
            .map_err(PersistPortError::from)?;
        Ok(())
    }

    fn get_offer(&self, offer_id: &str) -> PortResult<Option<CatalogOfferRow>> {
        self.pool
            .query_opt(catalog_offer_by_id_select_sql(), &[&offer_id], |row| {
                Ok(CatalogOfferRow {
                    offer_id: row.try_get(0).map_err(|e| e.to_string())?,
                    version: row.try_get(1).map_err(|e| e.to_string())?,
                    origin: row.try_get(2).map_err(|e| e.to_string())?,
                })
            })
            .map_err(PersistPortError::from)
    }

    fn list_offers(&self) -> PortResult<Vec<CatalogOfferRow>> {
        self.pool
            .query_map(catalog_offers_list_select_sql(), &[], |row| {
                Ok(CatalogOfferRow {
                    offer_id: row.try_get(0).map_err(|e| e.to_string())?,
                    version: row.try_get(1).map_err(|e| e.to_string())?,
                    origin: row.try_get(2).map_err(|e| e.to_string())?,
                })
            })
            .map_err(PersistPortError::from)
    }
}
