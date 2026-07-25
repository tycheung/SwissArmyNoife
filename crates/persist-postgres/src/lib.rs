//! Postgres persistence adapter (`sak064` / `sak070`).
//!
//! Enable `--features postgres` for deadpool connect, `V1_DDL` migrations, and live
//! Catalog/Binding/Audit stores. Select at runtime with `SAK_PERSIST_BACKEND=postgres`.

#[cfg(feature = "postgres")]
mod backend;
mod env;
mod migrations;
mod pool;
mod sql_bulk;
mod sql_clears;
mod sql_clears_audit;
mod sql_clears_bindings;
mod sql_clears_bulk;
mod sql_clears_catalog;
mod sql_clears_schema;
mod sql_counts;
mod sql_exists;
mod sql_lists;
mod sql_mutates;
mod sql_page;
mod sql_page_filtered;
mod sql_sketch;
mod sql_touches;

#[cfg(feature = "postgres")]
pub use backend::{try_open_from_env, try_open_from_url_env, PostgresBackend};
pub use env::{
    pg_url_from_env, postgres_enabled, DATABASE_URL_ENV, PERSIST_BACKEND_ENV, PG_URL_ENV,
};
pub use migrations::{
    planned_statements, schema_needs_apply, try_apply as try_apply_migrations,
    try_apply_planned_then_post, try_apply_planned_then_post_with_pool, try_apply_with_executor,
    try_apply_with_pool, MigrationError, MigrationExecutor, PoolBoundMigrationExecutor,
    RecordingMigrationExecutor, UnimplementedMigrationExecutor, SCHEMA_VERSION,
};
pub use pool::{
    PoolConfig, PoolConfigError, PoolConnectError, PoolHandle, DEFAULT_MAX_CONNECTIONS,
    PG_MAX_CONNECTIONS_ENV,
};
pub use sql_bulk::{
    audit_invokes_delete_by_binding_sql, audit_invokes_delete_by_offer_sql,
    bindings_delete_by_offer_sql, bindings_delete_by_principal_sql, bindings_delete_expired_sql,
    catalog_offers_delete_by_origin_sql,
};
pub use sql_clears::{
    audit_invoke_clear_binding_id_sql, audit_invoke_clear_code_sql,
    audit_invoke_clear_created_at_sql, audit_invoke_clear_detail_sql,
    audit_invoke_clear_offer_id_sql, audit_invoke_clear_status_sql,
    audit_invokes_clear_binding_id_by_offer_sql, audit_invokes_clear_code_by_offer_sql,
    audit_invokes_clear_created_at_by_offer_sql, audit_invokes_clear_detail_by_offer_sql,
    audit_invokes_clear_status_by_offer_sql, binding_clear_created_at_sql,
    binding_clear_expires_at_sql, binding_clear_offer_id_sql, binding_clear_policy_json_sql,
    binding_clear_principal_sql, bindings_clear_created_at_by_offer_sql,
    bindings_clear_expires_at_by_offer_sql, bindings_clear_policy_json_by_offer_sql,
    bindings_clear_principal_by_offer_sql, catalog_offer_clear_created_at_sql,
    catalog_offer_clear_descriptor_sql, catalog_offer_clear_origin_sql,
    catalog_offer_clear_version_sql, catalog_offers_clear_created_at_by_origin_sql,
    catalog_offers_clear_descriptor_by_origin_sql, catalog_offers_clear_version_by_origin_sql,
    schema_migrations_clear_applied_at_sql,
};
pub use sql_counts::{
    audit_by_binding_count_sql, audit_by_offer_count_sql, audit_invokes_count_sql,
    bindings_by_offer_count_sql, bindings_by_principal_count_sql, bindings_count_sql,
    bindings_expired_count_sql, catalog_offers_by_origin_count_sql, catalog_offers_count_sql,
    schema_migrations_count_sql,
};
pub use sql_exists::{
    audit_invoke_exists_by_id_sql, binding_exists_by_id_sql, catalog_offer_exists_by_id_sql,
};
pub use sql_lists::{
    audit_invokes_list_select_sql, bindings_expired_select_sql, bindings_list_select_sql,
    catalog_offers_list_select_sql, schema_migrations_list_select_sql,
};
pub use sql_mutates::{
    audit_invoke_delete_sql, audit_invoke_insert_sql, binding_delete_sql, binding_upsert_sql,
    catalog_offer_delete_sql, catalog_offer_upsert_sql,
};
pub use sql_page::{
    audit_invokes_list_limit_offset_sql, bindings_list_limit_offset_sql,
    catalog_offers_list_limit_offset_sql, schema_migrations_list_limit_offset_sql,
};
pub use sql_page_filtered::{
    audit_by_binding_limit_offset_sql, audit_by_offer_limit_offset_sql,
    bindings_by_offer_limit_offset_sql, bindings_by_principal_limit_offset_sql,
    bindings_expired_limit_offset_sql, catalog_offers_by_origin_limit_offset_sql,
};
pub use sql_sketch::{
    audit_by_binding_select_sql, audit_by_offer_select_sql, audit_invoke_by_id_select_sql,
    binding_by_id_select_sql, bindings_by_offer_select_sql, bindings_by_principal_select_sql,
    catalog_offer_by_id_select_sql, catalog_offers_by_origin_select_sql,
    planned_post_apply_statements, schema_version_delete_sql, schema_version_insert_sql,
    schema_version_select_sql, AUDIT_INVOKES_DDL, BINDINGS_DDL, CATALOG_OFFERS_DDL,
    IDX_AUDIT_BINDING_DDL, IDX_AUDIT_OFFER_DDL, IDX_BINDINGS_EXPIRES_DDL, IDX_BINDINGS_OFFER_DDL,
    IDX_BINDINGS_PRINCIPAL_DDL, IDX_CATALOG_ORIGIN_DDL, SCHEMA_MIGRATIONS_DDL, V1_DDL,
};
pub use sql_touches::{
    audit_invoke_touch_binding_id_sql, audit_invoke_touch_code_sql, audit_invoke_touch_detail_sql,
    audit_invoke_touch_offer_id_sql, audit_invoke_touch_status_sql, binding_touch_expires_at_sql,
    binding_touch_offer_id_sql, binding_touch_policy_json_sql, binding_touch_principal_sql,
    catalog_offer_touch_descriptor_sql, catalog_offer_touch_origin_sql,
    catalog_offer_touch_version_sql,
};

#[cfg(feature = "postgres")]
pub mod ports;

#[cfg(feature = "postgres")]
mod postgres {
    //! Compile-gated Postgres adapter (`sak064-b` / sak070 Phase B).

    /// Marker that the `postgres` feature is enabled at compile time.
    pub const FEATURE_ENABLED: bool = true;
}

/// Whether the crate was built with the optional `postgres` feature.
#[must_use]
pub fn postgres_feature_enabled() -> bool {
    #[cfg(feature = "postgres")]
    {
        postgres::FEATURE_ENABLED
    }
    #[cfg(not(feature = "postgres"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_feature_flag_matches_cfg() {
        assert_eq!(postgres_feature_enabled(), cfg!(feature = "postgres"));
    }

    #[test]
    fn ports_module_available_when_postgres_feature_enabled() {
        #[cfg(feature = "postgres")]
        {
            assert!(crate::ports::PORTS_SKETCH_ENABLED);
        }
        #[cfg(not(feature = "postgres"))]
        {
            assert!(!postgres_feature_enabled());
        }
    }
}
