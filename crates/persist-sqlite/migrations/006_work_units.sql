-- sak291-b: durable compute work units
CREATE TABLE IF NOT EXISTS work_units (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL,
    claimed_by TEXT,
    result_json TEXT,
    created_at INTEGER NOT NULL,
    seq INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_work_units_status_seq
    ON work_units (status, seq);
