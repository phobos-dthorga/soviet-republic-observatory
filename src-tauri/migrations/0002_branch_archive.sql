CREATE TABLE timeline_branches (
    branch_id TEXT PRIMARY KEY,
    branch_kind TEXT NOT NULL
        CHECK (branch_kind IN ('main', 'fork', 'unassigned')),
    created_at_ms INTEGER NOT NULL,
    parent_branch_id TEXT REFERENCES timeline_branches(branch_id),
    fork_record_id INTEGER CHECK (fork_record_id IS NULL OR fork_record_id >= 0),
    CHECK (branch_id <> parent_branch_id),
    CHECK (
        (branch_kind = 'fork' AND parent_branch_id IS NOT NULL) OR
        (branch_kind <> 'fork' AND parent_branch_id IS NULL)
    )
) STRICT;

INSERT INTO timeline_branches(
    branch_id, branch_kind, created_at_ms, parent_branch_id, fork_record_id
) VALUES('unassigned', 'unassigned', 0, NULL, NULL);

CREATE TABLE observation_lineage (
    payload_hash TEXT PRIMARY KEY
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    parent_payload_hash TEXT
        REFERENCES observation_sources(payload_hash) ON DELETE SET NULL,
    relationship TEXT NOT NULL CHECK (
        relationship IN (
            'root',
            'successor',
            'equivalent_history',
            'rollback_fork',
            'divergent_fork',
            'ambiguous'
        )
    ),
    shared_record_count INTEGER NOT NULL CHECK (shared_record_count >= 0),
    resolved_at_ms INTEGER NOT NULL,
    CHECK (payload_hash <> parent_payload_hash)
) STRICT;

CREATE TABLE observation_history_signatures (
    payload_hash TEXT PRIMARY KEY
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    record_count INTEGER NOT NULL CHECK (record_count > 0),
    tip_fingerprint TEXT NOT NULL CHECK (length(tip_fingerprint) = 64)
) STRICT;

CREATE TABLE archive_observations (
    observation_id INTEGER PRIMARY KEY,
    payload_hash TEXT NOT NULL
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    source_file_name TEXT NOT NULL,
    source_file_size INTEGER NOT NULL CHECK (source_file_size >= 0),
    source_modified_ms INTEGER NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    UNIQUE (
        payload_hash,
        source_file_name,
        source_file_size,
        source_modified_ms
    )
) STRICT;

CREATE TABLE archive_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    selected_branch_id TEXT NOT NULL
        REFERENCES timeline_branches(branch_id)
) STRICT;

INSERT INTO archive_state(singleton_id, selected_branch_id)
VALUES(1, 'unassigned');

INSERT INTO archive_observations(
    payload_hash,
    source_file_name,
    source_file_size,
    source_modified_ms,
    observed_at_ms
)
SELECT
    payload_hash,
    source_file_name,
    source_file_size,
    source_modified_ms,
    imported_at_ms
FROM observation_sources;

CREATE INDEX observation_sources_branch_imported_at
    ON observation_sources(branch_id, imported_at_ms DESC);

CREATE INDEX archive_observations_payload_observed_at
    ON archive_observations(payload_hash, observed_at_ms DESC);

CREATE INDEX timeline_branches_parent
    ON timeline_branches(parent_branch_id);

CREATE INDEX observation_history_signature_tip
    ON observation_history_signatures(record_count, tip_fingerprint);
