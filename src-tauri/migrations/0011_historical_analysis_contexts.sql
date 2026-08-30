CREATE TABLE timeline_branch_metadata (
    branch_id TEXT PRIMARY KEY
        REFERENCES timeline_branches(branch_id) ON DELETE CASCADE,
    origin TEXT NOT NULL CHECK (origin IN ('automatic', 'manual_continuation')),
    short_identity TEXT NOT NULL UNIQUE
        CHECK (length(short_identity) BETWEEN 3 AND 24),
    player_label TEXT CHECK (
        player_label IS NULL OR length(player_label) BETWEEN 1 AND 120
    ),
    anchor_interpretation_id TEXT,
    membership_revision INTEGER NOT NULL DEFAULT 0
        CHECK (membership_revision >= 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT INTO timeline_branch_metadata(
    branch_id, origin, short_identity, player_label, anchor_interpretation_id,
    membership_revision, created_at_ms, updated_at_ms
)
SELECT branch_id, 'automatic',
       CASE
           WHEN branch_id = 'main' THEN 'main'
           WHEN branch_id = 'unassigned' THEN 'unassigned'
           ELSE substr(branch_id, 1, 24)
       END,
       NULL, NULL,
       (SELECT COUNT(*) FROM observation_sources source
        WHERE source.branch_id = timeline_branches.branch_id),
       created_at_ms, created_at_ms
FROM timeline_branches;

CREATE TABLE timeline_branch_memberships (
    branch_id TEXT NOT NULL
        REFERENCES timeline_branches(branch_id) ON DELETE CASCADE,
    interpretation_id TEXT NOT NULL,
    payload_hash TEXT NOT NULL
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    parent_interpretation_id TEXT,
    relationship TEXT NOT NULL CHECK (
        relationship IN (
            'root', 'successor', 'equivalent_history', 'rollback_fork',
            'divergent_fork', 'ambiguous', 'continuation_anchor'
        )
    ),
    shared_record_count INTEGER NOT NULL CHECK (shared_record_count >= 0),
    membership_revision INTEGER NOT NULL CHECK (membership_revision > 0),
    added_at_ms INTEGER NOT NULL,
    PRIMARY KEY (branch_id, interpretation_id)
) STRICT;

INSERT INTO timeline_branch_memberships(
    branch_id, interpretation_id, payload_hash, parent_interpretation_id,
    relationship, shared_record_count, membership_revision, added_at_ms
)
SELECT source.branch_id, source.interpretation_id, source.payload_hash,
       parent.interpretation_id, lineage.relationship,
       lineage.shared_record_count,
       ROW_NUMBER() OVER (
           PARTITION BY source.branch_id
           ORDER BY signature.record_count, source.interpretation_id
       ),
       lineage.resolved_at_ms
FROM observation_sources source
JOIN observation_lineage lineage ON lineage.payload_hash = source.payload_hash
JOIN observation_history_signatures signature
  ON signature.payload_hash = source.payload_hash
LEFT JOIN observation_sources parent
  ON parent.payload_hash = lineage.parent_payload_hash
WHERE source.interpretation_id IS NOT NULL;

CREATE INDEX timeline_branch_memberships_interpretation
    ON timeline_branch_memberships(interpretation_id, branch_id);
CREATE INDEX timeline_branch_memberships_revision
    ON timeline_branch_memberships(branch_id, membership_revision);

CREATE TABLE analysis_context_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    selected_branch_id TEXT NOT NULL
        REFERENCES timeline_branches(branch_id),
    head_interpretation_id TEXT,
    mode TEXT NOT NULL CHECK (mode IN ('latest', 'historical_preview')),
    origin TEXT NOT NULL CHECK (origin IN ('automatic', 'manual_continuation')),
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT INTO analysis_context_state(
    singleton_id, selected_branch_id, head_interpretation_id, mode, origin,
    updated_at_ms
)
SELECT 1, archive.selected_branch_id,
       (SELECT membership.interpretation_id
        FROM timeline_branch_memberships membership
        JOIN observation_history_signatures signature
          ON signature.payload_hash = membership.payload_hash
        WHERE membership.branch_id = archive.selected_branch_id
        ORDER BY signature.record_count DESC, membership.interpretation_id DESC
        LIMIT 1),
       'latest', 'automatic', 0
FROM archive_state archive WHERE archive.singleton_id = 1;

ALTER TABLE warehouse_projection_jobs RENAME TO warehouse_projection_jobs_v10;

CREATE TABLE warehouse_projection_jobs (
    projection_id TEXT PRIMARY KEY
        CHECK (length(projection_id) BETWEEN 3 AND 160),
    projection_kind TEXT NOT NULL CHECK (
        projection_kind IN (
            'observation', 'overlay_state', 'branch_membership', 'rebuild'
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

INSERT INTO warehouse_projection_jobs
SELECT * FROM warehouse_projection_jobs_v10;
DROP TABLE warehouse_projection_jobs_v10;

CREATE INDEX warehouse_projection_jobs_queue
    ON warehouse_projection_jobs(status, requested_at_ms, projection_id);

INSERT INTO warehouse_projection_jobs(
    projection_id, projection_kind, source_identity, status, requested_at_ms
)
SELECT 'branch_membership:' || branch_id || ':' || membership_revision,
       'branch_membership', branch_id, 'pending', updated_at_ms
FROM timeline_branch_metadata
WHERE membership_revision > 0;
