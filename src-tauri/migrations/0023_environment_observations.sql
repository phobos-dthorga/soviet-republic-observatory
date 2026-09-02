CREATE TABLE environment_records (
    record_hash TEXT PRIMARY KEY CHECK(length(record_hash) = 64),
    record_id INTEGER NOT NULL CHECK(record_id >= 0),
    year INTEGER NOT NULL,
    day INTEGER NOT NULL CHECK(day BETWEEN 0 AND 364),
    game_day INTEGER NOT NULL
) STRICT;

CREATE TABLE environment_activity_facts (
    record_hash TEXT NOT NULL REFERENCES environment_records(record_hash) ON DELETE CASCADE,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    row_ordinal INTEGER NOT NULL CHECK(row_ordinal >= 0),
    resource_token TEXT NOT NULL CHECK(length(resource_token) BETWEEN 1 AND 128),
    activity_channel TEXT NOT NULL CHECK(activity_channel IN (
        'production', 'construction_use', 'factory_use', 'shop_use', 'vehicle_use',
        'factory_waste', 'citizen_waste', 'demolition_waste'
    )),
    primary_value REAL NOT NULL,
    secondary_value REAL NOT NULL,
    quantity_is_publishable INTEGER NOT NULL CHECK(quantity_is_publishable IN (0, 1)),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(record_hash, source_field, row_ordinal)
) STRICT;

CREATE INDEX environment_activity_resource
    ON environment_activity_facts(resource_token, activity_channel);

CREATE TABLE environment_observation_records (
    payload_hash TEXT NOT NULL REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    record_hash TEXT NOT NULL REFERENCES environment_records(record_hash),
    PRIMARY KEY(payload_hash, ordinal),
    UNIQUE(payload_hash, record_hash)
) STRICT;

CREATE INDEX environment_observation_record
    ON environment_observation_records(record_hash);

CREATE TABLE environment_observation_coverage (
    payload_hash TEXT PRIMARY KEY REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    storage_contract_version INTEGER NOT NULL CHECK(storage_contract_version > 0),
    coverage_status TEXT NOT NULL CHECK(coverage_status IN ('complete', 'partial')),
    history_records INTEGER NOT NULL CHECK(history_records >= 0),
    stored_records INTEGER NOT NULL CHECK(stored_records >= 0),
    row_count INTEGER NOT NULL CHECK(row_count >= 0),
    warnings_json TEXT NOT NULL
) STRICT;

CREATE TABLE environment_interpretation_variants (
    raw_payload_hash TEXT NOT NULL,
    interpretation_id TEXT NOT NULL REFERENCES observation_sources(interpretation_id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    resolved_profile_hash TEXT NOT NULL,
    indexed_at_ms INTEGER NOT NULL,
    PRIMARY KEY(raw_payload_hash, interpretation_id)
) STRICT;

CREATE TABLE environment_recording_state (
    singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
    enabled INTEGER NOT NULL CHECK(enabled IN (0, 1)),
    interval_game_days INTEGER NOT NULL CHECK(interval_game_days BETWEEN 1 AND 365),
    accepted_notice_revision INTEGER NOT NULL CHECK(accepted_notice_revision >= 0),
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT INTO environment_recording_state(
    singleton_id, enabled, interval_game_days, accepted_notice_revision, updated_at_ms
) VALUES(1, 0, 7, 0, 0);

CREATE TABLE environment_live_sessions (
    session_id TEXT PRIMARY KEY,
    executable_identity TEXT NOT NULL,
    probe_version TEXT NOT NULL,
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER
) STRICT;

CREATE TABLE environment_live_snapshots (
    snapshot_id TEXT PRIMARY KEY CHECK(length(snapshot_id) = 64),
    session_id TEXT NOT NULL REFERENCES environment_live_sessions(session_id),
    game_day INTEGER NOT NULL,
    facility_count INTEGER NOT NULL CHECK(facility_count BETWEEN 0 AND 25000),
    captured_at_ms INTEGER NOT NULL,
    storage_contract_version INTEGER NOT NULL CHECK(storage_contract_version > 0)
) STRICT;

CREATE TABLE environment_facility_readings (
    snapshot_id TEXT NOT NULL REFERENCES environment_live_snapshots(snapshot_id) ON DELETE CASCADE,
    facility_index INTEGER NOT NULL CHECK(facility_index BETWEEN 0 AND 24999),
    position_x REAL NOT NULL,
    position_z REAL NOT NULL,
    definition_identity TEXT,
    pollution_value REAL,
    radiation_value REAL,
    water_amount REAL,
    water_capacity REAL,
    water_quality REAL,
    sewage_amount REAL,
    sewage_capacity REAL,
    sewage_quality REAL,
    PRIMARY KEY(snapshot_id, facility_index)
) STRICT;

CREATE TABLE carbon_factor_revisions (
    factor_set_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    accounting_boundary TEXT NOT NULL CHECK(length(accounting_boundary) BETWEEN 1 AND 240),
    reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 500),
    entries_json TEXT NOT NULL,
    content_hash TEXT NOT NULL CHECK(length(content_hash) = 64),
    created_at_ms INTEGER NOT NULL,
    removed_at_ms INTEGER,
    PRIMARY KEY(factor_set_id, revision),
    UNIQUE(content_hash)
) STRICT;

CREATE TABLE carbon_factor_selection (
    singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
    factor_set_id TEXT,
    revision INTEGER,
    selected_at_ms INTEGER,
    FOREIGN KEY(factor_set_id, revision)
        REFERENCES carbon_factor_revisions(factor_set_id, revision)
) STRICT;

INSERT INTO carbon_factor_selection(singleton_id, factor_set_id, revision, selected_at_ms)
VALUES(1, NULL, NULL, NULL);
