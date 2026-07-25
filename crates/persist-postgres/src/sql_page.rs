//! Unfiltered LIMIT/OFFSET list SQL sketches (`sak070-ej` / `sak070-el` /
//! `sak070-em` / `sak070-ex` / `refactor:persist-postgres-sql-page`).
//!
//! Filtered page helpers live in [`crate::sql_page_filtered`].
//! Sketches are **not executed** until a live pool lands.

/// `$1` = limit, `$2` = offset.
#[must_use]
pub fn bindings_list_limit_offset_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings ORDER BY binding_id LIMIT $1 OFFSET $2"
}

/// `$1` = limit, `$2` = offset.
#[must_use]
pub fn catalog_offers_list_limit_offset_sql() -> &'static str {
    "SELECT offer_id, version, origin, descriptor_json, created_at \
     FROM catalog_offers ORDER BY offer_id LIMIT $1 OFFSET $2"
}

/// `$1` = limit, `$2` = offset.
#[must_use]
pub fn audit_invokes_list_limit_offset_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes ORDER BY created_at DESC LIMIT $1 OFFSET $2"
}

/// `$1` = limit, `$2` = offset.
#[must_use]
pub fn schema_migrations_list_limit_offset_sql() -> &'static str {
    "SELECT version, applied_at FROM schema_migrations \
     ORDER BY version LIMIT $1 OFFSET $2"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_list_limit_offset_ok() {
        let sql = bindings_list_limit_offset_sql();
        assert!(sql.contains("FROM bindings"));
        assert!(sql.contains("ORDER BY binding_id"));
        assert!(sql.contains("LIMIT $1"));
        assert!(sql.contains("OFFSET $2"));
    }

    #[test]
    fn catalog_offers_list_limit_offset_ok() {
        let sql = catalog_offers_list_limit_offset_sql();
        assert!(sql.contains("FROM catalog_offers"));
        assert!(sql.contains("ORDER BY offer_id"));
        assert!(sql.contains("LIMIT $1"));
        assert!(sql.contains("OFFSET $2"));
    }

    #[test]
    fn audit_invokes_list_limit_offset_ok() {
        let sql = audit_invokes_list_limit_offset_sql();
        assert!(sql.contains("FROM audit_invokes"));
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("LIMIT $1"));
        assert!(sql.contains("OFFSET $2"));
    }

    #[test]
    fn schema_migrations_list_limit_offset_ok() {
        let sql = schema_migrations_list_limit_offset_sql();
        assert!(sql.contains("FROM schema_migrations"));
        assert!(sql.contains("ORDER BY version"));
        assert!(sql.contains("LIMIT $1"));
        assert!(sql.contains("OFFSET $2"));
    }
}
