CREATE TABLE citizen_status_records (
    record_hash TEXT PRIMARY KEY CHECK(length(record_hash) = 64),
    record_id INTEGER NOT NULL CHECK(record_id >= 0),
    year INTEGER NOT NULL,
    day INTEGER NOT NULL CHECK(day BETWEEN 0 AND 364),
    game_day INTEGER NOT NULL
) STRICT;

CREATE TABLE citizen_status_facts (
    record_hash TEXT NOT NULL REFERENCES citizen_status_records(record_hash) ON DELETE CASCADE,
    source_index INTEGER NOT NULL CHECK(source_index BETWEEN 0 AND 8),
    metric_id TEXT NOT NULL,
    value_real REAL NOT NULL CHECK(value_real >= 0.0 AND value_real <= 1.0),
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(record_hash, source_index),
    UNIQUE(record_hash, metric_id)
) STRICT;

CREATE TABLE broadcast_status_observation_records (
    payload_hash TEXT NOT NULL REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    record_hash TEXT NOT NULL REFERENCES citizen_status_records(record_hash),
    PRIMARY KEY(payload_hash, ordinal),
    UNIQUE(payload_hash, record_hash)
) STRICT;

CREATE INDEX broadcast_status_records_record
    ON broadcast_status_observation_records(record_hash);

CREATE TABLE broadcast_status_observation_coverage (
    payload_hash TEXT PRIMARY KEY REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    storage_contract_version INTEGER NOT NULL CHECK(storage_contract_version > 0),
    coverage_status TEXT NOT NULL CHECK(coverage_status IN ('complete', 'partial')),
    history_records INTEGER NOT NULL CHECK(history_records >= 0),
    stored_records INTEGER NOT NULL CHECK(stored_records >= 0),
    dropped_records INTEGER NOT NULL CHECK(dropped_records >= 0),
    warnings_json TEXT NOT NULL
) STRICT;

CREATE TABLE broadcast_status_interpretation_variants (
    raw_payload_hash TEXT NOT NULL,
    interpretation_id TEXT NOT NULL REFERENCES observation_sources(interpretation_id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    resolved_profile_hash TEXT NOT NULL,
    indexed_at_ms INTEGER NOT NULL,
    PRIMARY KEY(raw_payload_hash, interpretation_id)
) STRICT;

