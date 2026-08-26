CREATE TABLE observation_sources (
    payload_hash TEXT PRIMARY KEY,
    source_file_name TEXT NOT NULL,
    source_file_size INTEGER NOT NULL CHECK (source_file_size >= 0),
    source_modified_ms INTEGER NOT NULL,
    imported_at_ms INTEGER NOT NULL,
    parser_version TEXT NOT NULL,
    format_profile TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    geographic_scope TEXT NOT NULL,
    coverage_status TEXT NOT NULL CHECK (coverage_status IN ('complete', 'partial')),
    history_records INTEGER NOT NULL CHECK (history_records >= 0),
    chartable_records INTEGER NOT NULL CHECK (chartable_records >= 0),
    dropped_records INTEGER NOT NULL CHECK (dropped_records >= 0),
    warnings_json TEXT NOT NULL
) STRICT;

CREATE TABLE embedded_records (
    payload_hash TEXT NOT NULL REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    record_id INTEGER NOT NULL CHECK (record_id >= 0),
    year INTEGER NOT NULL,
    day INTEGER NOT NULL CHECK (day BETWEEN 0 AND 364),
    game_day INTEGER NOT NULL,
    classified_total INTEGER NOT NULL CHECK (classified_total >= 0),
    PRIMARY KEY (payload_hash, record_id)
) STRICT;

CREATE TABLE metric_observations (
    payload_hash TEXT NOT NULL,
    record_id INTEGER NOT NULL,
    metric_id TEXT NOT NULL,
    value_integer INTEGER NOT NULL CHECK (value_integer >= 0),
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK (source_line > 0),
    evidence_kind TEXT NOT NULL CHECK (evidence_kind = 'save_fact'),
    coverage TEXT NOT NULL CHECK (coverage = 'complete'),
    PRIMARY KEY (payload_hash, record_id, metric_id),
    FOREIGN KEY (payload_hash, record_id)
        REFERENCES embedded_records(payload_hash, record_id)
        ON DELETE CASCADE
) STRICT;

CREATE INDEX observation_sources_imported_at
    ON observation_sources(imported_at_ms DESC);

CREATE INDEX metric_observations_metric_date
    ON metric_observations(metric_id, payload_hash, record_id);

CREATE TABLE private_settings (
    setting_key TEXT PRIMARY KEY,
    setting_value TEXT NOT NULL
) STRICT;
