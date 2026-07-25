//! Field-clear SQL sketches facade (`refactor:persist-postgres-sql-clears` /
//! `refactor:persist-postgres-sql-clears-catalog` /
//! `refactor:persist-postgres-sql-clears-audit` /
//! `refactor:persist-postgres-sql-clears-bindings` /
//! `refactor:persist-postgres-sql-clears-schema`).
//!
//! Catalog: [`crate::sql_clears_catalog`]. Audit: [`crate::sql_clears_audit`].
//! Bindings: [`crate::sql_clears_bindings`]. Schema: [`crate::sql_clears_schema`].
//! Sketches are **not executed** until a live pool lands.

pub use crate::sql_clears_audit::{
    audit_invoke_clear_binding_id_sql, audit_invoke_clear_code_sql,
    audit_invoke_clear_created_at_sql, audit_invoke_clear_detail_sql,
    audit_invoke_clear_offer_id_sql, audit_invoke_clear_status_sql,
};
pub use crate::sql_clears_bindings::{
    binding_clear_created_at_sql, binding_clear_expires_at_sql, binding_clear_offer_id_sql,
    binding_clear_policy_json_sql, binding_clear_principal_sql,
};
pub use crate::sql_clears_bulk::{
    audit_invokes_clear_binding_id_by_offer_sql, audit_invokes_clear_code_by_offer_sql,
    audit_invokes_clear_created_at_by_offer_sql, audit_invokes_clear_detail_by_offer_sql,
    audit_invokes_clear_status_by_offer_sql, bindings_clear_created_at_by_offer_sql,
    bindings_clear_expires_at_by_offer_sql, bindings_clear_policy_json_by_offer_sql,
    bindings_clear_principal_by_offer_sql, catalog_offers_clear_created_at_by_origin_sql,
    catalog_offers_clear_descriptor_by_origin_sql, catalog_offers_clear_version_by_origin_sql,
};
pub use crate::sql_clears_catalog::{
    catalog_offer_clear_created_at_sql, catalog_offer_clear_descriptor_sql,
    catalog_offer_clear_origin_sql, catalog_offer_clear_version_sql,
};
pub use crate::sql_clears_schema::schema_migrations_clear_applied_at_sql;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_clear_expires_at_ok() {
        let sql = binding_clear_expires_at_sql();
        assert!(sql.contains("expires_at = NULL"));
    }

    #[test]
    fn binding_clear_policy_json_ok() {
        let sql = binding_clear_policy_json_sql();
        assert!(sql.contains("policy_json = NULL"));
    }

    #[test]
    fn catalog_offer_clear_descriptor_ok() {
        let sql = catalog_offer_clear_descriptor_sql();
        assert!(sql.contains("descriptor_json = NULL"));
    }

    #[test]
    fn audit_invoke_clear_detail_ok() {
        let sql = audit_invoke_clear_detail_sql();
        assert!(sql.contains("detail_json = NULL"));
    }

    #[test]
    fn audit_invoke_clear_code_ok() {
        let sql = audit_invoke_clear_code_sql();
        assert!(sql.contains("code = NULL"));
    }

    #[test]
    fn binding_clear_principal_ok() {
        let sql = binding_clear_principal_sql();
        assert!(sql.contains("principal = NULL"));
    }

    #[test]
    fn audit_invoke_clear_offer_id_ok() {
        let sql = audit_invoke_clear_offer_id_sql();
        assert!(sql.contains("offer_id = NULL"));
    }

    #[test]
    fn catalog_offer_clear_origin_ok() {
        let sql = catalog_offer_clear_origin_sql();
        assert!(sql.contains("origin = NULL"));
    }

    #[test]
    fn binding_clear_offer_id_ok() {
        let sql = binding_clear_offer_id_sql();
        assert!(sql.contains("offer_id = NULL"));
    }

    #[test]
    fn audit_invoke_clear_binding_id_ok() {
        let sql = audit_invoke_clear_binding_id_sql();
        assert!(sql.contains("binding_id = NULL"));
    }

    #[test]
    fn catalog_offer_clear_version_ok() {
        let sql = catalog_offer_clear_version_sql();
        assert!(sql.contains("version = NULL"));
    }

    #[test]
    fn audit_invoke_clear_status_ok() {
        let sql = audit_invoke_clear_status_sql();
        assert!(sql.contains("status = NULL"));
    }

    #[test]
    fn schema_migrations_clear_applied_at_ok() {
        let sql = schema_migrations_clear_applied_at_sql();
        assert!(sql.contains("applied_at = NULL"));
        assert!(sql.contains("version = $1"));
    }

    #[test]
    fn binding_clear_created_at_ok() {
        let sql = binding_clear_created_at_sql();
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("binding_id = $1"));
    }

    #[test]
    fn catalog_offer_clear_created_at_ok() {
        let sql = catalog_offer_clear_created_at_sql();
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("offer_id = $1"));
    }

    #[test]
    fn audit_invoke_clear_created_at_ok() {
        let sql = audit_invoke_clear_created_at_sql();
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("invoke_id = $1"));
    }

    #[test]
    fn catalog_offers_clear_descriptor_by_origin_ok() {
        let sql = catalog_offers_clear_descriptor_by_origin_sql();
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("descriptor_json = NULL"));
    }

    #[test]
    fn bindings_clear_expires_at_by_offer_ok() {
        let sql = bindings_clear_expires_at_by_offer_sql();
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("expires_at = NULL"));
    }
}
