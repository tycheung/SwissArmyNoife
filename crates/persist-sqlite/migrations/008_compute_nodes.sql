-- sak425-a: durable compute node registry
CREATE TABLE IF NOT EXISTS compute_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    caps_json TEXT NOT NULL DEFAULT '[]',
    last_heartbeat_unix INTEGER NOT NULL,
    session_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_compute_nodes_session
    ON compute_nodes (session_id);

CREATE INDEX IF NOT EXISTS idx_compute_nodes_heartbeat
    ON compute_nodes (last_heartbeat_unix);
