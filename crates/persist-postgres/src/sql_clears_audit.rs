//! Audit-invoke field-clear SQL sketches (`sak070-fb` / `sak070-fd` / `sak070-fg` /
//! `sak070-fk` / `sak070-fn` / `refactor:persist-postgres-sql-clears-audit`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` = invoke_id.
#[must_use]
pub fn audit_invoke_clear_detail_sql() -> &'static str {
    "UPDATE audit_invokes SET detail_json = NULL WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = invoke_id.
#[must_use]
pub fn audit_invoke_clear_code_sql() -> &'static str {
    "UPDATE audit_invokes SET code = NULL WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = invoke_id.
#[must_use]
pub fn audit_invoke_clear_offer_id_sql() -> &'static str {
    "UPDATE audit_invokes SET offer_id = NULL WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = invoke_id.
#[must_use]
pub fn audit_invoke_clear_binding_id_sql() -> &'static str {
    "UPDATE audit_invokes SET binding_id = NULL WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = invoke_id.
#[must_use]
pub fn audit_invoke_clear_status_sql() -> &'static str {
    "UPDATE audit_invokes SET status = NULL WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

/// `$1` = invoke_id.
#[must_use]
pub fn audit_invoke_clear_created_at_sql() -> &'static str {
    "UPDATE audit_invokes SET created_at = NULL WHERE invoke_id = $1 \
     RETURNING invoke_id"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_invoke_clear_detail_ok() {
        let sql = audit_invoke_clear_detail_sql();
        assert!(sql.contains("UPDATE audit_invokes"));
        assert!(sql.contains("detail_json = NULL"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn audit_invoke_clear_code_ok() {
        let sql = audit_invoke_clear_code_sql();
        assert!(sql.contains("code = NULL"));
        assert!(sql.contains("invoke_id = $1"));
    }

    #[test]
    fn audit_invoke_clear_offer_id_ok() {
        let sql = audit_invoke_clear_offer_id_sql();
        assert!(sql.contains("offer_id = NULL"));
        assert!(sql.contains("invoke_id = $1"));
    }

    #[test]
    fn audit_invoke_clear_binding_id_ok() {
        let sql = audit_invoke_clear_binding_id_sql();
        assert!(sql.contains("binding_id = NULL"));
        assert!(sql.contains("invoke_id = $1"));
    }

    #[test]
    fn audit_invoke_clear_status_ok() {
        let sql = audit_invoke_clear_status_sql();
        assert!(sql.contains("status = NULL"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }

    #[test]
    fn audit_invoke_clear_created_at_ok() {
        let sql = audit_invoke_clear_created_at_sql();
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("RETURNING invoke_id"));
    }
}
