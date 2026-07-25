//! Schema-migration field-clear SQL sketches (`sak070-fp` /
//! `refactor:persist-postgres-sql-clears-schema`).
//!
//! Sketches are **not executed** until a live pool lands.

/// `$1` = version.
#[must_use]
pub fn schema_migrations_clear_applied_at_sql() -> &'static str {
    "UPDATE schema_migrations SET applied_at = NULL WHERE version = $1 \
     RETURNING version"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_migrations_clear_applied_at_ok() {
        let sql = schema_migrations_clear_applied_at_sql();
        assert!(sql.contains("UPDATE schema_migrations"));
        assert!(sql.contains("applied_at = NULL"));
        assert!(sql.contains("version = $1"));
        assert!(sql.contains("RETURNING version"));
    }
}
