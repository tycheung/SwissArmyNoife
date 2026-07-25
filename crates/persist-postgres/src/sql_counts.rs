//! Filtered and table-wide COUNT SQL sketches (`sak070-ce` / `sak070-cv`–`cz` /
//! `refactor:persist-postgres-sql-counts`).
//!
//! Sketches are **not executed** until a live pool lands.

#[must_use]
pub fn catalog_offers_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM catalog_offers"
}

#[must_use]
pub fn bindings_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM bindings"
}

#[must_use]
pub fn audit_invokes_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM audit_invokes"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn bindings_by_offer_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM bindings WHERE offer_id = $1"
}

/// `$1` is the principal bind parameter.
#[must_use]
pub fn bindings_by_principal_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM bindings WHERE principal = $1"
}

/// `$1` is the origin bind parameter.
#[must_use]
pub fn catalog_offers_by_origin_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM catalog_offers WHERE origin = $1"
}

/// `$1` is the binding_id bind parameter.
#[must_use]
pub fn audit_by_binding_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM audit_invokes WHERE binding_id = $1"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn audit_by_offer_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM audit_invokes WHERE offer_id = $1"
}

/// `$1` is the cutoff `TIMESTAMPTZ` bind parameter.
#[must_use]
pub fn bindings_expired_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM bindings \
     WHERE expires_at IS NOT NULL AND expires_at < $1"
}

#[must_use]
pub fn schema_migrations_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM schema_migrations"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_offers_count_ok() {
        let sql = catalog_offers_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("catalog_offers"));
    }

    #[test]
    fn bindings_count_ok() {
        let sql = bindings_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("bindings"));
    }

    #[test]
    fn audit_invokes_count_ok() {
        let sql = audit_invokes_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("audit_invokes"));
    }

    #[test]
    fn bindings_by_offer_count_ok() {
        let sql = bindings_by_offer_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("offer_id = $1"));
    }

    #[test]
    fn bindings_by_principal_count_ok() {
        let sql = bindings_by_principal_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("principal = $1"));
    }

    #[test]
    fn catalog_offers_by_origin_count_ok() {
        let sql = catalog_offers_by_origin_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("catalog_offers"));
    }

    #[test]
    fn audit_by_binding_count_ok() {
        let sql = audit_by_binding_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("audit_invokes"));
    }

    #[test]
    fn audit_by_offer_count_ok() {
        let sql = audit_by_offer_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("audit_invokes"));
    }

    #[test]
    fn bindings_expired_count_ok() {
        let sql = bindings_expired_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("expires_at < $1"));
        assert!(sql.contains("expires_at IS NOT NULL"));
    }

    #[test]
    fn schema_migrations_count_ok() {
        let sql = schema_migrations_count_sql();
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("schema_migrations"));
    }
}
