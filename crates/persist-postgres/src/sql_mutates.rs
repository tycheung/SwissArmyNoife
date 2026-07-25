//! Upsert / insert / single-row delete SQL sketches (`sak070-bk`–`bt` / `sak070-cg` /
//! `refactor:persist-postgres-sql-mutates`).
//!
//! Field touches live in [`crate::sql_touches`]. Bulk deletes in [`crate::sql_bulk`].
//! Sketches are **not executed** until a live pool lands.

/// Bind params: `$1` offer_id, `$2` version, `$3` origin, `$4` descriptor_json.
#[must_use]
pub fn catalog_offer_upsert_sql() -> &'static str {
    "INSERT INTO catalog_offers (offer_id, version, origin, descriptor_json) \
     VALUES ($1, $2, $3, $4) \
     ON CONFLICT (offer_id) DO UPDATE SET \
       version = EXCLUDED.version, \
       origin = EXCLUDED.origin, \
       descriptor_json = EXCLUDED.descriptor_json"
}

/// Bind params: `$1` binding_id, `$2` offer_id, `$3` principal, `$4` policy_json,
/// `$5` expires_at (nullable `TIMESTAMPTZ`).
#[must_use]
pub fn binding_upsert_sql() -> &'static str {
    "INSERT INTO bindings (binding_id, offer_id, principal, policy_json, expires_at) \
     VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (binding_id) DO UPDATE SET \
       offer_id = EXCLUDED.offer_id, \
       principal = EXCLUDED.principal, \
       policy_json = EXCLUDED.policy_json, \
       expires_at = EXCLUDED.expires_at"
}

/// Bind params: `$1` invoke_id, `$2` binding_id, `$3` offer_id, `$4` status,
/// `$5` code, `$6` detail_json.
#[must_use]
pub fn audit_invoke_insert_sql() -> &'static str {
    "INSERT INTO audit_invokes (invoke_id, binding_id, offer_id, status, code, detail_json) \
     VALUES ($1, $2, $3, $4, $5, $6)"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn catalog_offer_delete_sql() -> &'static str {
    "DELETE FROM catalog_offers WHERE offer_id = $1"
}

/// `$1` is the binding_id bind parameter.
#[must_use]
pub fn binding_delete_sql() -> &'static str {
    "DELETE FROM bindings WHERE binding_id = $1"
}

/// `$1` is the invoke_id bind parameter.
#[must_use]
pub fn audit_invoke_delete_sql() -> &'static str {
    "DELETE FROM audit_invokes WHERE invoke_id = $1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_offer_upsert_ok() {
        let sql = catalog_offer_upsert_sql();
        assert!(sql.contains("INSERT INTO catalog_offers"));
        assert!(sql.contains("ON CONFLICT (offer_id)"));
        assert!(sql.contains("EXCLUDED.descriptor_json"));
    }

    #[test]
    fn binding_upsert_ok() {
        let sql = binding_upsert_sql();
        assert!(sql.contains("INSERT INTO bindings"));
        assert!(sql.contains("ON CONFLICT (binding_id)"));
        assert!(sql.contains("EXCLUDED.policy_json"));
        assert!(sql.contains("expires_at"));
        assert!(sql.contains("EXCLUDED.expires_at"));
        assert!(sql.contains("$5"));
    }

    #[test]
    fn audit_invoke_insert_ok() {
        let sql = audit_invoke_insert_sql();
        assert!(sql.contains("INSERT INTO audit_invokes"));
        assert!(sql.contains("detail_json"));
        assert!(sql.contains("$6"));
    }

    #[test]
    fn catalog_offer_delete_ok() {
        let sql = catalog_offer_delete_sql();
        assert!(sql.contains("DELETE FROM catalog_offers"));
        assert!(sql.contains("offer_id = $1"));
    }

    #[test]
    fn binding_delete_ok() {
        let sql = binding_delete_sql();
        assert!(sql.contains("DELETE FROM bindings"));
        assert!(sql.contains("binding_id = $1"));
    }

    #[test]
    fn audit_invoke_delete_ok() {
        let sql = audit_invoke_delete_sql();
        assert!(sql.contains("DELETE FROM audit_invokes"));
        assert!(sql.contains("invoke_id = $1"));
    }
}
