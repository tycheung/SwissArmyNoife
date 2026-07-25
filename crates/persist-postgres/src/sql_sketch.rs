//! Postgres DDL sketch constants (`sak070-o`).
//!
//! Hand-ported shapes from `persist-sqlite` core tables — **not executed**.
//! Real `try_apply` will run these (or equivalent) against a live pool later.

/// Sketch `catalog_offers` DDL for Postgres (`sak070-o`).
pub const CATALOG_OFFERS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS catalog_offers (
    offer_id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'core',
    descriptor_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
"#;

/// Sketch `bindings` DDL for Postgres (`sak070-o`).
pub const BINDINGS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS bindings (
    binding_id TEXT PRIMARY KEY,
    offer_id TEXT NOT NULL REFERENCES catalog_offers(offer_id),
    principal TEXT NOT NULL DEFAULT 'local',
    policy_json TEXT NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
"#;

/// Sketch `audit_invokes` DDL for Postgres (`sak070-o`).
pub const AUDIT_INVOKES_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS audit_invokes (
    invoke_id TEXT PRIMARY KEY,
    binding_id TEXT NOT NULL REFERENCES bindings(binding_id),
    offer_id TEXT,
    status TEXT NOT NULL,
    code TEXT,
    detail_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
"#;

/// Sketch `schema_migrations` DDL for Postgres (`sak070-s`).
pub const SCHEMA_MIGRATIONS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
"#;

/// Sketch index on `bindings(offer_id)` (`sak070-u`).
pub const IDX_BINDINGS_OFFER_DDL: &str =
    "CREATE INDEX IF NOT EXISTS idx_bindings_offer ON bindings(offer_id)";

/// Sketch index on `audit_invokes(binding_id)` (`sak070-u`).
pub const IDX_AUDIT_BINDING_DDL: &str =
    "CREATE INDEX IF NOT EXISTS idx_audit_binding ON audit_invokes(binding_id)";

/// Sketch index on `catalog_offers(origin)` (`sak070-av`).
pub const IDX_CATALOG_ORIGIN_DDL: &str =
    "CREATE INDEX IF NOT EXISTS idx_catalog_origin ON catalog_offers(origin)";

/// Sketch index on `bindings(expires_at)` for expiry sweeps (`sak070-bw`).
pub const IDX_BINDINGS_EXPIRES_DDL: &str =
    "CREATE INDEX IF NOT EXISTS idx_bindings_expires ON bindings(expires_at) \
     WHERE expires_at IS NOT NULL";

/// Sketch index on `bindings(principal)` (`sak070-cf`).
pub const IDX_BINDINGS_PRINCIPAL_DDL: &str =
    "CREATE INDEX IF NOT EXISTS idx_bindings_principal ON bindings(principal)";

/// Sketch index on `audit_invokes(offer_id)` (`sak070-ck`).
pub const IDX_AUDIT_OFFER_DDL: &str =
    "CREATE INDEX IF NOT EXISTS idx_audit_offer ON audit_invokes(offer_id)";

/// Ordered DDL sketches for schema v1 (documentation / future apply).
///
/// Includes version table first, then core control-plane tables + indexes.
pub const V1_DDL: &[&str] = &[
    SCHEMA_MIGRATIONS_DDL,
    CATALOG_OFFERS_DDL,
    BINDINGS_DDL,
    AUDIT_INVOKES_DDL,
    IDX_BINDINGS_OFFER_DDL,
    IDX_AUDIT_BINDING_DDL,
    IDX_CATALOG_ORIGIN_DDL,
    IDX_BINDINGS_EXPIRES_DDL,
    IDX_BINDINGS_PRINCIPAL_DDL,
    IDX_AUDIT_OFFER_DDL,
];

/// DML statements to run after successful DDL apply (`sak070-ac`).
#[must_use]
pub fn planned_post_apply_statements(version: u32) -> Vec<String> {
    vec![schema_version_insert_sql(version)]
}

/// Not part of [`V1_DDL`] — run after successful DDL apply in a later slice.
#[must_use]
pub fn schema_version_insert_sql(version: u32) -> String {
    format!(
        "INSERT INTO schema_migrations (version) VALUES ({version}) \
         ON CONFLICT (version) DO NOTHING"
    )
}

/// Used for future migrate-if-needed checks.
#[must_use]
pub fn schema_version_select_sql() -> &'static str {
    "SELECT MAX(version) FROM schema_migrations"
}

/// `sak070-cq`).
///
/// `$1` is the origin bind parameter.
#[must_use]
pub fn catalog_offers_by_origin_select_sql() -> &'static str {
    "SELECT offer_id, version, origin, descriptor_json, created_at \
     FROM catalog_offers WHERE origin = $1 ORDER BY offer_id"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn bindings_by_offer_select_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings WHERE offer_id = $1 ORDER BY binding_id"
}

/// `$1` is the binding_id bind parameter.
#[must_use]
pub fn audit_by_binding_select_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes WHERE binding_id = $1 ORDER BY created_at DESC"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn catalog_offer_by_id_select_sql() -> &'static str {
    "SELECT offer_id, version, origin, descriptor_json, created_at \
     FROM catalog_offers WHERE offer_id = $1"
}

/// `$1` is the binding_id bind parameter.
#[must_use]
pub fn binding_by_id_select_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, policy_json, expires_at \
     FROM bindings WHERE binding_id = $1"
}

/// `$1` is the principal bind parameter.
#[must_use]
pub fn bindings_by_principal_select_sql() -> &'static str {
    "SELECT binding_id, offer_id, principal, expires_at \
     FROM bindings WHERE principal = $1 ORDER BY binding_id"
}

/// `$1` is the offer_id bind parameter.
#[must_use]
pub fn audit_by_offer_select_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes WHERE offer_id = $1 ORDER BY created_at DESC"
}

/// `$1` is the invoke_id bind parameter.
#[must_use]
pub fn audit_invoke_by_id_select_sql() -> &'static str {
    "SELECT invoke_id, binding_id, offer_id, status, code, detail_json, created_at \
     FROM audit_invokes WHERE invoke_id = $1"
}

/// `$1` is the version bind parameter.
/// Intended for repair / rollback tooling — not part of normal apply.
#[must_use]
pub fn schema_version_delete_sql() -> &'static str {
    "DELETE FROM schema_migrations WHERE version = $1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_ddl_mentions_core_tables() {
        assert_eq!(V1_DDL.len(), 10);
        assert!(SCHEMA_MIGRATIONS_DDL.contains("schema_migrations"));
        assert!(CATALOG_OFFERS_DDL.contains("catalog_offers"));
        assert!(BINDINGS_DDL.contains("bindings"));
        assert!(AUDIT_INVOKES_DDL.contains("audit_invokes"));
        assert!(CATALOG_OFFERS_DDL.contains("TIMESTAMPTZ"));
        assert!(IDX_BINDINGS_OFFER_DDL.contains("idx_bindings_offer"));
        assert!(IDX_AUDIT_BINDING_DDL.contains("idx_audit_binding"));
        assert!(IDX_CATALOG_ORIGIN_DDL.contains("idx_catalog_origin"));
        assert!(IDX_BINDINGS_EXPIRES_DDL.contains("idx_bindings_expires"));
        assert!(IDX_BINDINGS_EXPIRES_DDL.contains("WHERE expires_at IS NOT NULL"));
        assert!(IDX_BINDINGS_PRINCIPAL_DDL.contains("idx_bindings_principal"));
        assert!(IDX_AUDIT_OFFER_DDL.contains("idx_audit_offer"));
    }

    #[test]
    fn schema_version_insert_mentions_version() {
        let sql = schema_version_insert_sql(1);
        assert!(sql.contains("schema_migrations"));
        assert!(sql.contains("VALUES (1)"));
        assert!(sql.contains("ON CONFLICT"));
    }

    #[test]
    fn planned_post_apply_includes_version_insert() {
        let stmts = planned_post_apply_statements(1);
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], schema_version_insert_sql(1));
    }

    #[test]
    fn schema_version_select_mentions_max() {
        let sql = schema_version_select_sql();
        assert!(sql.contains("schema_migrations"));
        assert!(sql.contains("MAX(version)"));
    }

    #[test]
    fn catalog_offers_by_origin_select_ok() {
        let sql = catalog_offers_by_origin_select_sql();
        assert!(sql.contains("catalog_offers"));
        assert!(sql.contains("origin = $1"));
        assert!(sql.contains("descriptor_json"));
        assert!(sql.contains("created_at"));
        assert!(sql.contains("ORDER BY offer_id"));
    }

    #[test]
    fn bindings_by_offer_select_ok() {
        let sql = bindings_by_offer_select_sql();
        assert!(sql.contains("bindings"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("expires_at"));
        assert!(sql.contains("ORDER BY binding_id"));
    }

    #[test]
    fn audit_by_binding_select_ok() {
        let sql = audit_by_binding_select_sql();
        assert!(sql.contains("audit_invokes"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("offer_id"));
        assert!(sql.contains("detail_json"));
        assert!(sql.contains("created_at"));
        assert!(sql.contains("ORDER BY created_at DESC"));
    }

    #[test]
    fn catalog_offer_by_id_select_ok() {
        let sql = catalog_offer_by_id_select_sql();
        assert!(sql.contains("catalog_offers"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("descriptor_json"));
        assert!(sql.contains("created_at"));
    }

    #[test]
    fn binding_by_id_select_ok() {
        let sql = binding_by_id_select_sql();
        assert!(sql.contains("bindings"));
        assert!(sql.contains("binding_id = $1"));
        assert!(sql.contains("policy_json"));
        assert!(sql.contains("expires_at"));
    }

    #[test]
    fn bindings_by_principal_select_ok() {
        let sql = bindings_by_principal_select_sql();
        assert!(sql.contains("bindings"));
        assert!(sql.contains("principal = $1"));
        assert!(sql.contains("expires_at"));
        assert!(sql.contains("ORDER BY binding_id"));
    }

    #[test]
    fn audit_by_offer_select_ok() {
        let sql = audit_by_offer_select_sql();
        assert!(sql.contains("audit_invokes"));
        assert!(sql.contains("offer_id = $1"));
        assert!(sql.contains("detail_json"));
        assert!(sql.contains("created_at"));
        assert!(sql.contains("ORDER BY created_at DESC"));
    }

    #[test]
    fn audit_invoke_by_id_select_ok() {
        let sql = audit_invoke_by_id_select_sql();
        assert!(sql.contains("audit_invokes"));
        assert!(sql.contains("invoke_id = $1"));
        assert!(sql.contains("detail_json"));
        assert!(sql.contains("created_at"));
    }

    #[test]
    fn schema_version_delete_ok() {
        let sql = schema_version_delete_sql();
        assert!(sql.contains("DELETE FROM schema_migrations"));
        assert!(sql.contains("version = $1"));
    }
}
