//! Bulk-delete SQL sketches (`sak070-dw` / `sak070-dx` / `sak070-dz` / `sak070-ea` /
//! `sak070-ec` / `sak070-ed` / `refactor:persist-postgres-sql-bulk`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` = offer_id.
#[must_use]
pub fn bindings_delete_by_offer_sql() -> &'static str {
    "DELETE FROM bindings WHERE offer_id = $1"
}

/// `$1` = cutoff timestamptz (rows with `expires_at IS NOT NULL AND expires_at < $1`).
#[must_use]
pub fn bindings_delete_expired_sql() -> &'static str {
    "DELETE FROM bindings WHERE expires_at IS NOT NULL AND expires_at < $1"
}

/// `$1` = binding_id.
#[must_use]
pub fn audit_invokes_delete_by_binding_sql() -> &'static str {
    "DELETE FROM audit_invokes WHERE binding_id = $1"
}

/// `$1` = principal.
#[must_use]
pub fn bindings_delete_by_principal_sql() -> &'static str {
    "DELETE FROM bindings WHERE principal = $1"
}

/// `$1` = offer_id.
#[must_use]
pub fn audit_invokes_delete_by_offer_sql() -> &'static str {
    "DELETE FROM audit_invokes WHERE offer_id = $1"
}

/// `$1` = origin.
#[must_use]
pub fn catalog_offers_delete_by_origin_sql() -> &'static str {
    "DELETE FROM catalog_offers WHERE origin = $1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_delete_by_offer_ok() {
        let sql = bindings_delete_by_offer_sql();
        assert!(sql.contains("DELETE FROM bindings"));
        assert!(sql.contains("offer_id = $1"));
    }

    #[test]
    fn bindings_delete_expired_ok() {
        let sql = bindings_delete_expired_sql();
        assert!(sql.contains("DELETE FROM bindings"));
        assert!(sql.contains("expires_at IS NOT NULL"));
        assert!(sql.contains("expires_at < $1"));
    }

    #[test]
    fn audit_invokes_delete_by_binding_ok() {
        let sql = audit_invokes_delete_by_binding_sql();
        assert!(sql.contains("DELETE FROM audit_invokes"));
        assert!(sql.contains("binding_id = $1"));
    }

    #[test]
    fn bindings_delete_by_principal_ok() {
        let sql = bindings_delete_by_principal_sql();
        assert!(sql.contains("DELETE FROM bindings"));
        assert!(sql.contains("principal = $1"));
    }

    #[test]
    fn audit_invokes_delete_by_offer_ok() {
        let sql = audit_invokes_delete_by_offer_sql();
        assert!(sql.contains("DELETE FROM audit_invokes"));
        assert!(sql.contains("offer_id = $1"));
    }

    #[test]
    fn catalog_offers_delete_by_origin_ok() {
        let sql = catalog_offers_delete_by_origin_sql();
        assert!(sql.contains("DELETE FROM catalog_offers"));
        assert!(sql.contains("origin = $1"));
    }
}
