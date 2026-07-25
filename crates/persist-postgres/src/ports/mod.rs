//! Postgres persistence ports (`sak070-a` / `sak070-b` / Phase B live stores).
//!
//! Trait definitions mirror `persist-sqlite` adapters. Memory / Unimplemented doubles
//! stay for offline tests; [`PostgresCatalog`] / [`PostgresBindingStore`] /
//! [`PostgresAuditStore`] execute SQL against a live pool.

mod audit;
mod binding;
mod catalog;
mod pg_audit;
mod pg_binding;
mod pg_catalog;

pub use audit::{AuditEventRow, AuditStore, MemoryAuditStore, UnimplementedAuditStore};
pub use binding::{BindingRow, BindingStore, MemoryBindingStore, UnimplementedBindingStore};
pub use catalog::{CatalogOfferRow, CatalogStore, MemoryCatalog, UnimplementedCatalog};
pub use pg_audit::PostgresAuditStore;
pub use pg_binding::PostgresBindingStore;
pub use pg_catalog::PostgresCatalog;

use thiserror::Error;

/// Port-layer errors for Postgres adapter sketches (`sak070-b` / `sak070-i`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PersistPortError {
    #[error("not implemented")]
    NotImplemented,
    /// Pool / URL / backend misconfiguration (`sak070-i`).
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

impl From<crate::PoolConfigError> for PersistPortError {
    fn from(err: crate::PoolConfigError) -> Self {
        Self::InvalidConfig(err.to_string())
    }
}

impl From<crate::PoolConnectError> for PersistPortError {
    fn from(err: crate::PoolConnectError) -> Self {
        match err {
            crate::PoolConnectError::Config(cfg) => cfg.into(),
            crate::PoolConnectError::NotImplemented => Self::NotImplemented,
            crate::PoolConnectError::Connect(msg) => Self::InvalidConfig(msg),
        }
    }
}

impl From<crate::MigrationError> for PersistPortError {
    fn from(err: crate::MigrationError) -> Self {
        match err {
            crate::MigrationError::NotImplemented(_) => Self::NotImplemented,
            crate::MigrationError::Execute(msg) => Self::InvalidConfig(msg),
        }
    }
}

pub type PortResult<T> = Result<T, PersistPortError>;

/// Marker that postgres port sketches compiled (`sak070-a`).
pub const PORTS_SKETCH_ENABLED: bool = true;

#[cfg(test)]
mod live_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn ports_sketch_marker_is_true() {
        assert!(PORTS_SKETCH_ENABLED);
    }

    #[test]
    fn persist_port_error_display_and_error_trait() {
        let err = PersistPortError::NotImplemented;
        assert_eq!(err.to_string(), "not implemented");
        assert_eq!(format!("{err}"), "not implemented");
        assert!(err.source().is_none());
    }

    #[test]
    fn unimplemented_catalog_returns_not_implemented() {
        let store = UnimplementedCatalog;
        assert_eq!(
            store.upsert_offer("llm.chat", "0.1.0", "core"),
            Err(PersistPortError::NotImplemented)
        );
        assert_eq!(
            store.get_offer("llm.chat"),
            Err(PersistPortError::NotImplemented)
        );
        assert_eq!(store.list_offers(), Err(PersistPortError::NotImplemented));
    }

    #[test]
    fn pool_config_error_maps_to_invalid_config() {
        let err: PersistPortError = crate::PoolConfigError::MissingUrl.into();
        assert_eq!(
            err,
            PersistPortError::InvalidConfig("postgres URL missing".into())
        );
        let err2: PersistPortError = crate::PoolConfigError::InvalidUrl.into();
        assert!(err2.to_string().contains("invalid config"));
        assert!(err2.to_string().contains("invalid postgres URL"));
    }

    #[test]
    fn pool_connect_not_implemented_maps() {
        let err: PersistPortError = crate::PoolConnectError::NotImplemented.into();
        assert_eq!(err, PersistPortError::NotImplemented);
    }

    #[test]
    fn pool_connect_config_maps_to_invalid_config() {
        let err: PersistPortError =
            crate::PoolConnectError::Config(crate::PoolConfigError::InvalidUrl).into();
        assert!(matches!(err, PersistPortError::InvalidConfig(_)));
    }

    #[test]
    fn migration_error_maps_to_not_implemented() {
        let err: PersistPortError =
            crate::MigrationError::NotImplemented(crate::SCHEMA_VERSION).into();
        assert_eq!(err, PersistPortError::NotImplemented);
    }
}
