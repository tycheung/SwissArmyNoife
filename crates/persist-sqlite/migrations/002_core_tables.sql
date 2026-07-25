-- Core control-plane tables (stubs; richer columns land with Phase 1 repos).

CREATE TABLE catalog_offers (
    offer_id TEXT NOT NULL PRIMARY KEY,
    version TEXT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'core',
    descriptor_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE bindings (
    binding_id TEXT NOT NULL PRIMARY KEY,
    offer_id TEXT NOT NULL,
    principal TEXT NOT NULL DEFAULT 'local',
    policy_json TEXT NOT NULL DEFAULT '{}',
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (offer_id) REFERENCES catalog_offers(offer_id)
);

CREATE TABLE audit_invokes (
    invoke_id TEXT NOT NULL PRIMARY KEY,
    binding_id TEXT NOT NULL,
    offer_id TEXT,
    status TEXT NOT NULL,
    code TEXT,
    detail_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (binding_id) REFERENCES bindings(binding_id)
);

CREATE INDEX idx_bindings_offer ON bindings(offer_id);
CREATE INDEX idx_audit_binding ON audit_invokes(binding_id);
