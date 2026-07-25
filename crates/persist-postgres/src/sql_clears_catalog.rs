//! Catalog-offer field-clear SQL sketches (`sak070-fa` / `sak070-fh` / `sak070-fm` /
//! `refactor:persist-postgres-sql-clears-catalog`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` = offer_id.
#[must_use]
pub fn catalog_offer_clear_descriptor_sql() -> &'static str {
    "UPDATE catalog_offers SET descriptor_json = NULL WHERE offer_id = $1 \
     RETURNING offer_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn catalog_offer_clear_origin_sql() -> &'static str {
    "UPDATE catalog_offers SET origin = NULL WHERE offer_id = $1 \
     RETURNING offer_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn catalog_offer_clear_version_sql() -> &'static str {
    "UPDATE catalog_offers SET version = NULL WHERE offer_id = $1 \
     RETURNING offer_id"
}

/// `$1` = offer_id.
#[must_use]
pub fn catalog_offer_clear_created_at_sql() -> &'static str {
    "UPDATE catalog_offers SET created_at = NULL WHERE offer_id = $1 \
     RETURNING offer_id"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_offer_clear_descriptor_ok() {
        let sql = catalog_offer_clear_descriptor_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("descriptor_json = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn catalog_offer_clear_origin_ok() {
        let sql = catalog_offer_clear_origin_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("origin = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn catalog_offer_clear_version_ok() {
        let sql = catalog_offer_clear_version_sql();
        assert!(sql.contains("UPDATE catalog_offers"));
        assert!(sql.contains("version = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }

    #[test]
    fn catalog_offer_clear_created_at_ok() {
        let sql = catalog_offer_clear_created_at_sql();
        assert!(sql.contains("created_at = NULL"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("RETURNING offer_id"));
    }
}
