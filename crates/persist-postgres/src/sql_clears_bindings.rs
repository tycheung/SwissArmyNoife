//! Binding field-clear SQL sketches (`sak070-dk` / `sak070-ey` / `sak070-fe` /
//! `sak070-fj` / `sak070-fq` / `refactor:persist-postgres-sql-clears-bindings`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` = binding_id.
#[must_use]
pub fn binding_clear_expires_at_sql() -> &'static str {
    "UPDATE bindings SET expires_at = NULL WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = binding_id.
#[must_use]
pub fn binding_clear_policy_json_sql() -> &'static str {
    "UPDATE bindings SET policy_json = NULL WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = binding_id.
#[must_use]
pub fn binding_clear_principal_sql() -> &'static str {
    "UPDATE bindings SET principal = NULL WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = binding_id.
#[must_use]
pub fn binding_clear_offer_id_sql() -> &'static str {
    "UPDATE bindings SET offer_id = NULL WHERE binding_id = $1 \
     RETURNING binding_id"
}

/// `$1` = binding_id.
#[must_use]
pub fn binding_clear_created_at_sql() -> &'static str {
    "UPDATE bindings SET created_at = NULL WHERE binding_id = $1 \
     RETURNING binding_id"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_clear_expires_at_ok() {
        let sql = binding_clear_expires_at_sql();
        assert!(sql.contains("UPDATE bindings"));
        assert!(sql.contains("expires_at = NULL"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }

    #[test]
    fn binding_clear_policy_json_ok() {
        let sql = binding_clear_policy_json_sql();
        assert!(sql.contains("policy_json = NULL"));
    }

    #[test]
    fn binding_clear_principal_ok() {
        let sql = binding_clear_principal_sql();
        assert!(sql.contains("principal = NULL"));
    }

    #[test]
    fn binding_clear_offer_id_ok() {
        let sql = binding_clear_offer_id_sql();
        assert!(sql.contains("offer_id = NULL"));
        assert!(sql.contains("binding_id = $1"));
    }

    #[test]
    fn binding_clear_created_at_ok() {
        let sql = binding_clear_created_at_sql();
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("RETURNING binding_id"));
    }
}
