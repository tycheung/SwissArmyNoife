-- Vault-backed provider connections (ciphertext only; plaintext never persisted).

CREATE TABLE vault_connections (
    connection_id TEXT NOT NULL PRIMARY KEY,
    provider TEXT NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    secret_blob BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_vault_connections_provider ON vault_connections(provider);
