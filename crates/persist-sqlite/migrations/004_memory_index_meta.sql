CREATE TABLE memory_index_meta (
    scope_key TEXT PRIMARY KEY NOT NULL,
    fingerprint TEXT NOT NULL,
    backend TEXT NOT NULL,
    vector_count INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);
