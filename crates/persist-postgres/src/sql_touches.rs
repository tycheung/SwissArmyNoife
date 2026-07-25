//! Field-touch SQL sketches (`sak070-dh` … `sak070-du` / `sak070-ef` /
//! `sak070-eg` / `sak070-ei` / `refactor:persist-postgres-sql-touches`).
//!
//! Clear helpers live in [`crate::sql_clears`].
//! Sketches are **not executed** until a live pool lands.

/// `$1` = binding_id, `$2` = expires_at (nullable `TIMESTAMPTZ`).
#[must_use]
pub fn binding_touch_expires_at_sql() -> &'static str {
    "UPDATE bindings SET expires_at = $2 WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = offer_id, `$2` = descriptor_json.
#[must_use]
pub fn catalog_offer_touch_descriptor_sql() -> &'static str {
    "UPDATE catalog_offers SET descriptor_json = $2 WHERE offer_id = $1 \
     RETURNING offer_id"
}

/// `$1` = invoke_id, `$2` = detail_json.
#[must_use]
pub fn audit_invoke_touch_detail_sql() -> &'static str {
    "UPDATE audit_invokes SET detail_json = $2 WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = binding_id, `$2` = policy_json.
#[must_use]
pub fn binding_touch_policy_json_sql() -> &'static str {
    "UPDATE bindings SET policy_json = $2 WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = offer_id, `$2` = version.
#[must_use]
pub fn catalog_offer_touch_version_sql() -> &'static str {
    "UPDATE catalog_offers SET version = $2 WHERE offer_id = $1 \
     RETURNING offer_id"
}

/// `$1` = invoke_id, `$2` = status.
#[must_use]
pub fn audit_invoke_touch_status_sql() -> &'static str {
    "UPDATE audit_invokes SET status = $2 WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = offer_id, `$2` = origin.
#[must_use]
pub fn catalog_offer_touch_origin_sql() -> &'static str {
    "UPDATE catalog_offers SET origin = $2 WHERE offer_id = $1 \
     RETURNING offer_id"
}

/// `$1` = invoke_id, `$2` = code.
#[must_use]
pub fn audit_invoke_touch_code_sql() -> &'static str {
    "UPDATE audit_invokes SET code = $2 WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = binding_id, `$2` = principal.
#[must_use]
pub fn binding_touch_principal_sql() -> &'static str {
    "UPDATE bindings SET principal = $2 WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = binding_id, `$2` = offer_id.
#[must_use]
pub fn binding_touch_offer_id_sql() -> &'static str {
    "UPDATE bindings SET offer_id = $2 WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = invoke_id, `$2` = binding_id.
#[must_use]
pub fn audit_invoke_touch_binding_id_sql() -> &'static str {
    "UPDATE audit_invokes SET binding_id = $2 WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = invoke_id, `$2` = offer_id.
#[must_use]
pub fn audit_invoke_touch_offer_id_sql() -> &'static str {
    "UPDATE audit_invokes SET offer_id = $2 WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_touch_expires_at_ok() {
        let sql = binding_touch_expires_at_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("expires_at = $2"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn catalog_offer_touch_descriptor_ok() {
        let sql = catalog_offer_touch_descriptor_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("descriptor_json = $2"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn audit_invoke_touch_detail_ok() {
        let sql = audit_invoke_touch_detail_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("detail_json = $2"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn binding_touch_policy_json_ok() {
        let sql = binding_touch_policy_json_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("policy_json = $2"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn catalog_offer_touch_version_ok() {
        let sql = catalog_offer_touch_version_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("version = $2"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn audit_invoke_touch_status_ok() {
        let sql = audit_invoke_touch_status_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("status = $2"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn catalog_offer_touch_origin_ok() {
        let sql = catalog_offer_touch_origin_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("origin = $2"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn audit_invoke_touch_code_ok() {
        let sql = audit_invoke_touch_code_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("code = $2"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn binding_touch_principal_ok() {
        let sql = binding_touch_principal_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("principal = $2"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn binding_touch_offer_id_ok() {
        let sql = binding_touch_offer_id_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("offer_id = $2"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn audit_invoke_touch_binding_id_ok() {
        let sql = audit_invoke_touch_binding_id_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("binding_id = $2"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn audit_invoke_touch_offer_id_ok() {
        let sql = audit_invoke_touch_offer_id_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("offer_id = $2"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }
}
