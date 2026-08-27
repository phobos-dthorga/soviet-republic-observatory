CREATE TABLE receiver_history_nodes (
    node_id INTEGER PRIMARY KEY,
    parent_node_id INTEGER REFERENCES receiver_history_nodes(node_id),
    depth INTEGER NOT NULL CHECK (depth > 0),
    prefix_fingerprint TEXT NOT NULL UNIQUE
        CHECK (length(prefix_fingerprint) = 64),
    record_id INTEGER NOT NULL CHECK (record_id >= 0),
    year INTEGER NOT NULL,
    day INTEGER NOT NULL CHECK (day BETWEEN 0 AND 364),
    game_day INTEGER NOT NULL,
    classified_total INTEGER NOT NULL CHECK (classified_total >= 0),
    none_value INTEGER NOT NULL CHECK (none_value >= 0),
    radio_value INTEGER NOT NULL CHECK (radio_value >= 0),
    television_value INTEGER NOT NULL CHECK (television_value >= 0),
    computer_value INTEGER NOT NULL CHECK (computer_value >= 0),
    CHECK (
        (depth = 1 AND parent_node_id IS NULL) OR
        (depth > 1 AND parent_node_id IS NOT NULL)
    )
) STRICT;

CREATE INDEX receiver_history_nodes_parent
    ON receiver_history_nodes(parent_node_id);

CREATE TABLE observation_history_tips (
    payload_hash TEXT PRIMARY KEY
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    tip_node_id INTEGER NOT NULL
        REFERENCES receiver_history_nodes(node_id),
    record_count INTEGER NOT NULL CHECK (record_count > 0)
) STRICT;

CREATE INDEX observation_history_tips_node
    ON observation_history_tips(tip_node_id);

CREATE TABLE observation_metric_evidence (
    payload_hash TEXT NOT NULL
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    metric_id TEXT NOT NULL,
    source_field TEXT NOT NULL,
    latest_source_line INTEGER NOT NULL CHECK (latest_source_line > 0),
    PRIMARY KEY (payload_hash, metric_id)
) STRICT;

CREATE TABLE snapshot_scopes (
    payload_hash TEXT NOT NULL
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('republic', 'city')),
    scope_id TEXT NOT NULL,
    sampled_year INTEGER NOT NULL,
    sampled_day INTEGER NOT NULL CHECK (sampled_day BETWEEN 0 AND 364),
    sampled_game_day INTEGER NOT NULL,
    coverage_status TEXT NOT NULL
        CHECK (coverage_status IN ('complete', 'partial')),
    supported_fact_count INTEGER NOT NULL CHECK (supported_fact_count >= 0),
    expected_fact_count INTEGER NOT NULL CHECK (expected_fact_count >= 0),
    PRIMARY KEY (payload_hash, scope_kind, scope_id),
    CHECK (
        (scope_kind = 'republic' AND scope_id = 'republic') OR
        (scope_kind = 'city' AND length(scope_id) > 0)
    )
) STRICT;

CREATE TABLE snapshot_scalar_facts (
    payload_hash TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    fact_id TEXT NOT NULL,
    value_integer INTEGER NOT NULL CHECK (value_integer >= 0),
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK (source_line > 0),
    evidence_kind TEXT NOT NULL CHECK (evidence_kind = 'save_fact'),
    coverage TEXT NOT NULL CHECK (coverage = 'complete'),
    PRIMARY KEY (payload_hash, scope_kind, scope_id, fact_id),
    FOREIGN KEY (payload_hash, scope_kind, scope_id)
        REFERENCES snapshot_scopes(payload_hash, scope_kind, scope_id)
        ON DELETE CASCADE
) STRICT;

CREATE INDEX snapshot_scopes_kind_date
    ON snapshot_scopes(scope_kind, scope_id, sampled_game_day);

ALTER TABLE archive_observations
    ADD COLUMN source_directory_identity TEXT;

CREATE INDEX archive_observations_candidate_identity
    ON archive_observations(
        source_directory_identity,
        source_file_name,
        source_file_size,
        source_modified_ms
    );
