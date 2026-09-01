ALTER TABLE warehouse_projection_jobs RENAME TO warehouse_projection_jobs_v20;

CREATE TABLE warehouse_projection_jobs (
    projection_id TEXT PRIMARY KEY
        CHECK (length(projection_id) BETWEEN 3 AND 160),
    projection_kind TEXT NOT NULL CHECK (
        projection_kind IN (
            'observation', 'market_observation', 'broadcast_observation',
            'overlay_state', 'branch_membership', 'rebuild'
        )
    ),
    source_identity TEXT NOT NULL
        CHECK (length(source_identity) BETWEEN 1 AND 256),
    status TEXT NOT NULL CHECK (
        status IN ('pending', 'running', 'applied', 'failed')
    ),
    requested_at_ms INTEGER NOT NULL,
    started_at_ms INTEGER,
    applied_at_ms INTEGER,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    error_code TEXT,
    CHECK (
        (status = 'applied' AND applied_at_ms IS NOT NULL) OR
        status <> 'applied'
    )
) STRICT;

INSERT INTO warehouse_projection_jobs(
    projection_id, projection_kind, source_identity, status, requested_at_ms,
    started_at_ms, applied_at_ms, attempt_count, error_code
)
SELECT projection_id, projection_kind, source_identity, status, requested_at_ms,
       started_at_ms, applied_at_ms, attempt_count, error_code
FROM warehouse_projection_jobs_v20;

DROP TABLE warehouse_projection_jobs_v20;

CREATE INDEX warehouse_projection_jobs_queue
    ON warehouse_projection_jobs(status, requested_at_ms, projection_id);

INSERT OR IGNORE INTO warehouse_projection_jobs(
    projection_id, projection_kind, source_identity, status, requested_at_ms
)
SELECT 'broadcast:' || interpretation_id, 'broadcast_observation', interpretation_id,
       'pending', indexed_at_ms
FROM broadcast_status_interpretation_variants;
