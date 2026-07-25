//! Filtered LIMIT/OFFSET list SQL sketches (`sak070-eo` / `sak070-ep` /
//! `sak070-er` / `sak070-es` / `sak070-eu` / `sak070-ev` /
//! `refactor:persist-postgres-sql-page-filtered`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` = offer_id, `$2` = limit, `$3` = offset.
#[must_use]
pub fn bindings_by_offer_limit_offset_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings WHERE offer_id = $1 \
     ORDER BY binding_id LIMIT $2 OFFSET $3"
}

/// `$1` = origin, `$2` = limit, `$3` = offset.
#[must_use]
pub fn catalog_offers_by_origin_limit_offset_sql() -> &'static str {
    "SELECT offer_id, version, origin, descriptor_json, created_at \
     FROM catalog_offers WHERE origin = $1 \
     ORDER BY offer_id LIMIT $2 OFFSET $3"
}

/// `$1` = principal, `$2` = limit, `$3` = offset.
#[must_use]
pub fn bindings_by_principal_limit_offset_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings WHERE principal = $1 \
     ORDER BY binding_id LIMIT $2 OFFSET $3"
}

/// `$1` = offer_id, `$2` = limit, `$3` = offset.
#[must_use]
pub fn audit_by_offer_limit_offset_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes WHERE offer_id = $1 \
     ORDER BY created_at DESC LIMIT $2 OFFSET $3"
}

/// `$1` = binding_id, `$2` = limit, `$3` = offset.
#[must_use]
pub fn audit_by_binding_limit_offset_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes WHERE binding_id = $1 \
     ORDER BY created_at DESC LIMIT $2 OFFSET $3"
}

/// `$1` = cutoff timestamptz, `$2` = limit, `$3` = offset.
#[must_use]
pub fn bindings_expired_limit_offset_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings WHERE expires_at IS NOT NULL AND expires_at < $1 \
     ORDER BY expires_at LIMIT $2 OFFSET $3"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_by_offer_limit_offset_ok() {
        let sql = bindings_by_offer_limit_offset_sql();
        assert!(sql.contains("FROM bindings"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("OFFSET $3"));
    }

    #[test]
    fn catalog_offers_by_origin_limit_offset_ok() {
        let sql = catalog_offers_by_origin_limit_offset_sql();
        assert!(sql.contains("FROM catalog_offers"));
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("OFFSET $3"));
    }

    #[test]
    fn bindings_by_principal_limit_offset_ok() {
        let sql = bindings_by_principal_limit_offset_sql();
        assert!(sql.contains("FROM bindings"));
        assert!(sql.contains("principal = $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("OFFSET $3"));
    }

    #[test]
    fn audit_by_offer_limit_offset_ok() {
        let sql = audit_by_offer_limit_offset_sql();
        assert!(sql.contains("FROM audit_invokes"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("OFFSET $3"));
        assert!(sql.contains("ORDER BY created_at DESC"));
    }

    #[test]
    fn audit_by_binding_limit_offset_ok() {
        let sql = audit_by_binding_limit_offset_sql();
        assert!(sql.contains("FROM audit_invokes"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("OFFSET $3"));
        assert!(sql.contains("ORDER BY created_at DESC"));
    }

    #[test]
    fn bindings_expired_limit_offset_ok() {
        let sql = bindings_expired_limit_offset_sql();
        assert!(sql.contains("FROM bindings"));
        assert!(sql.contains("expires_at IS NOT NULL"));
        assert!(sql.contains("expires_at < $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("OFFSET $3"));
    }
}
