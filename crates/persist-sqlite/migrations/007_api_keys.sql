CREATE TABLE IF NOT EXISTS api_keys (
    key_id TEXT PRIMARY KEY NOT NULL,
    hash_hex TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_api_keys_hash_hex ON api_keys (hash_hex);
