//! List-all / list-by-cutoff SQL sketches (`sak070-bz` / `sak070-cc` / `sak070-cs` /
//! `sak070-ct` / `sak070-bv` / `refactor:persist-postgres-sql-lists`).
//!
//! LIMIT/OFFSET helpers live in [`crate::sql_page`].
//! Sketches are **not executed** until a live pool lands.

/// `$1` is the cutoff `TIMESTAMPTZ` bind parameter.
#[must_use]
pub fn bindings_expired_select_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings WHERE expires_at IS NOT NULL AND expires_at < $1 \
     ORDER BY expires_at"
}

#[must_use]
pub fn schema_migrations_list_select_sql() -> &'static str {
    "SELECT version, applied_at FROM schema_migrations ORDER BY version"
}

#[must_use]
pub fn catalog_offers_list_select_sql() -> &'static str {
    "SELECT offer_id, version, origin, descriptor_json, created_at \
     FROM catalog_offers ORDER BY offer_id"
}

#[must_use]
pub fn bindings_list_select_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings ORDER BY binding_id"
}

#[must_use]
pub fn audit_invokes_list_select_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes ORDER BY created_at DESC"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_expired_select_ok() {
        let sql = bindings_expired_select_sql();
        assert!(sql.contains("bindings"));
        assert!(sql.contains("expires_at < $1"));
        assert!(sql.contains("expires_at IS NOT NULL"));
        assert!(sql.contains("ORDER BY expires_at"));
    }

    #[test]
    fn schema_migrations_list_select_ok() {
        let sql = schema_migrations_list_select_sql();
        assert!(sql.contains("schema_migrations"));
        assert!(sql.contains("ORDER BY version"));
        assert!(sql.contains("applied_at"));
    }

    #[test]
    fn catalog_offers_list_select_ok() {
        let sql = catalog_offers_list_select_sql();
        assert!(sql.contains("catalog_offers"));
        assert!(sql.contains("ORDER BY offer_id"));
        assert!(sql.contains("descriptor_json"));
    }

    #[test]
    fn bindings_list_select_ok() {
        let sql = bindings_list_select_sql();
        assert!(sql.contains("FROM bindings"));
        assert!(sql.contains("ORDER BY binding_id"));
        assert!(sql.contains("expires_at"));
    }

    #[test]
    fn audit_invokes_list_select_ok() {
        let sql = audit_invokes_list_select_sql();
        assert!(sql.contains("FROM audit_invokes"));
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("detail_json"));
    }
}
