CREATE TABLE recorder_candidates (
    candidate_id INTEGER PRIMARY KEY,
    source_directory_identity TEXT NOT NULL
        CHECK (length(source_directory_identity) = 64),
    source_file_name TEXT NOT NULL
        CHECK (length(source_file_name) BETWEEN 1 AND 512),
    source_file_size INTEGER NOT NULL CHECK (source_file_size >= 0),
    source_modified_ms INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN (
            'discovered',
            'stabilising',
            'ready',
            'reading',
            'imported',
            'duplicate',
            'retryable_failure',
            'terminal_failure',
            'superseded'
        )
    ),
    discovery_source TEXT NOT NULL CHECK (
        discovery_source IN (
            'migration',
            'initial_scan',
            'filesystem_event',
            'reconciliation'
        )
    ),
    discovered_at_ms INTEGER NOT NULL,
    first_stable_at_ms INTEGER,
    last_attempt_at_ms INTEGER,
    completed_at_ms INTEGER,
    last_seen_at_ms INTEGER NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    error_code TEXT,
    import_outcome TEXT CHECK (import_outcome IN ('imported', 'duplicate')),
    payload_hash TEXT REFERENCES observation_sources(payload_hash),
    UNIQUE (
        source_directory_identity,
        source_file_name,
        source_file_size,
        source_modified_ms
    ),
    CHECK (
        (status IN ('imported', 'duplicate') AND completed_at_ms IS NOT NULL
            AND import_outcome = status AND payload_hash IS NOT NULL)
        OR status NOT IN ('imported', 'duplicate')
    ),
    CHECK (
        (status IN ('terminal_failure', 'superseded') AND completed_at_ms IS NOT NULL)
        OR status NOT IN ('terminal_failure', 'superseded')
    )
) STRICT;

CREATE INDEX recorder_candidates_queue
    ON recorder_candidates(status, discovered_at_ms, candidate_id);

CREATE INDEX recorder_candidates_directory_seen
    ON recorder_candidates(source_directory_identity, last_seen_at_ms DESC);

CREATE TABLE recorder_runtime_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    last_scan_ms INTEGER,
    last_filesystem_event_ms INTEGER
) STRICT;

INSERT INTO recorder_runtime_state(
    singleton_id, last_scan_ms, last_filesystem_event_ms
) VALUES(1, NULL, NULL);

INSERT OR IGNORE INTO recorder_candidates(
    source_directory_identity,
    source_file_name,
    source_file_size,
    source_modified_ms,
    status,
    discovery_source,
    discovered_at_ms,
    first_stable_at_ms,
    last_attempt_at_ms,
    completed_at_ms,
    last_seen_at_ms,
    attempt_count,
    error_code,
    import_outcome,
    payload_hash
)
SELECT
    source_directory_identity,
    source_file_name,
    source_file_size,
    source_modified_ms,
    'imported',
    'migration',
    MIN(observed_at_ms),
    MIN(observed_at_ms),
    MIN(observed_at_ms),
    MIN(observed_at_ms),
    MAX(observed_at_ms),
    1,
    NULL,
    'imported',
    MIN(payload_hash)
FROM archive_observations
WHERE source_directory_identity IS NOT NULL
GROUP BY
    source_directory_identity,
    source_file_name,
    source_file_size,
    source_modified_ms;
