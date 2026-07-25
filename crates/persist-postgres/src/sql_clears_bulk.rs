//! Multi-row field-clear SQL sketches (`sak070-fv` / `sak070-fw` / `sak070-fy` /
//! `sak070-fz` / `sak070-gb` / `sak070-gc` / `sak070-ge` / `sak070-gf` /
//! `sak070-gh` / `sak070-gi` / `sak070-gj` / `sak070-gk` /
//! `refactor:persist-postgres-sql-clears-bulk`).
//!
//! Single-row clears live in `sql_clears_*` modules.
//! Sketches are **not executed** until a live pool lands.
//! Finite single→bulk mirror for nullable columns is **closed** after `sak070-gk`.

/// `$1` = origin.
#[must_use]
pub fn catalog_offers_clear_descriptor_by_origin_sql() -> &'static str {
    "UPDATE catalog_offers SET descriptor_json = NULL WHERE origin = $1 \
     RETURNING offer_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn bindings_clear_expires_at_by_offer_sql() -> &'static str {
    "UPDATE bindings SET expires_at = NULL WHERE offer_id = $1 \
     RETURNING binding_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn audit_invokes_clear_detail_by_offer_sql() -> &'static str {
    "UPDATE audit_invokes SET detail_json = NULL WHERE offer_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn bindings_clear_principal_by_offer_sql() -> &'static str {
    "UPDATE bindings SET principal = NULL WHERE offer_id = $1 \
     RETURNING binding_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn audit_invokes_clear_code_by_offer_sql() -> &'static str {
    "UPDATE audit_invokes SET code = NULL WHERE offer_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = origin.
#[must_use]
pub fn catalog_offers_clear_version_by_origin_sql() -> &'static str {
    "UPDATE catalog_offers SET version = NULL WHERE origin = $1 \
     RETURNING offer_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn audit_invokes_clear_status_by_offer_sql() -> &'static str {
    "UPDATE audit_invokes SET status = NULL WHERE offer_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn bindings_clear_policy_json_by_offer_sql() -> &'static str {
    "UPDATE bindings SET policy_json = NULL WHERE offer_id = $1 \
     RETURNING binding_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn audit_invokes_clear_binding_id_by_offer_sql() -> &'static str {
    "UPDATE audit_invokes SET binding_id = NULL WHERE offer_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn bindings_clear_created_at_by_offer_sql() -> &'static str {
    "UPDATE bindings SET created_at = NULL WHERE offer_id = $1 \
     RETURNING binding_id"
}

/// `$1` = origin.
#[must_use]
pub fn catalog_offers_clear_created_at_by_origin_sql() -> &'static str {
    "UPDATE catalog_offers SET created_at = NULL WHERE origin = $1 \
     RETURNING offer_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn audit_invokes_clear_created_at_by_offer_sql() -> &'static str {
    "UPDATE audit_invokes SET created_at = NULL WHERE offer_id = $1 \
     RETURNING invoke_id"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_offers_clear_descriptor_by_origin_ok() {
        let sql = catalog_offers_clear_descriptor_by_origin_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("descriptor_json = NULL"));
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn bindings_clear_expires_at_by_offer_ok() {
        let sql = bindings_clear_expires_at_by_offer_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("expires_at = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn audit_invokes_clear_detail_by_offer_ok() {
        let sql = audit_invokes_clear_detail_by_offer_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("detail_json = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn bindings_clear_principal_by_offer_ok() {
        let sql = bindings_clear_principal_by_offer_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("principal = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn audit_invokes_clear_code_by_offer_ok() {
        let sql = audit_invokes_clear_code_by_offer_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("code = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn catalog_offers_clear_version_by_origin_ok() {
        let sql = catalog_offers_clear_version_by_origin_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("version = NULL"));
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn audit_invokes_clear_status_by_offer_ok() {
        let sql = audit_invokes_clear_status_by_offer_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("status = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn bindings_clear_policy_json_by_offer_ok() {
        let sql = bindings_clear_policy_json_by_offer_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("policy_json = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn audit_invokes_clear_binding_id_by_offer_ok() {
        let sql = audit_invokes_clear_binding_id_by_offer_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("binding_id = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn bindings_clear_created_at_by_offer_ok() {
        let sql = bindings_clear_created_at_by_offer_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn catalog_offers_clear_created_at_by_origin_ok() {
        let sql = catalog_offers_clear_created_at_by_origin_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn audit_invokes_clear_created_at_by_offer_ok() {
        let sql = audit_invokes_clear_created_at_by_offer_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }
}
