//! Existence-check SQL sketches (`sak070-de` / `sak070-df` /
//! `refactor:persist-postgres-sql-exists`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` is the binding_id bind parameter.
#[must_use]
pub fn binding_exists_by_id_sql() -> &'static str {
    "SELECT 1 FROM bindings WHERE binding_id = $1 LIMIT 1"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn catalog_offer_exists_by_id_sql() -> &'static str {
    "SELECT 1 FROM catalog_offers WHERE offer_id = $1 LIMIT 1"
}

/// `$1` is the invoke_id bind parameter.
#[must_use]
pub fn audit_invoke_exists_by_id_sql() -> &'static str {
    "SELECT 1 FROM audit_invokes WHERE invoke_id = $1 LIMIT 1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_exists_by_id_ok() {
        let sql = binding_exists_by_id_sql();
        assert!(sql.contains("SELECT 1"));
        assert!(sql.contains("bindings"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("LIMIT 1"));
    }

    #[test]
    fn catalog_offer_exists_by_id_ok() {
        let sql = catalog_offer_exists_by_id_sql();
        assert!(sql.contains("SELECT 1"));
        assert!(sql.contains("catalog_offers"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("LIMIT 1"));
    }

    #[test]
    fn audit_invoke_exists_by_id_ok() {
        let sql = audit_invoke_exists_by_id_sql();
        assert!(sql.contains("SELECT 1"));
        assert!(sql.contains("audit_invokes"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("LIMIT 1"));
    }
}
