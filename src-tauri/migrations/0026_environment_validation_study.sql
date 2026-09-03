CREATE TABLE environment_validation_snapshots (
    snapshot_id TEXT PRIMARY KEY CHECK(length(snapshot_id) = 64),
    checked_session_id TEXT NOT NULL CHECK(length(checked_session_id) BETWEEN 1 AND 96),
    candidate_contract_version INTEGER NOT NULL CHECK(candidate_contract_version > 0),
    probe_version TEXT NOT NULL CHECK(length(probe_version) BETWEEN 1 AND 32),
    game_build_id TEXT NOT NULL CHECK(length(game_build_id) BETWEEN 1 AND 128),
    year INTEGER NOT NULL,
    day INTEGER NOT NULL CHECK(day BETWEEN 0 AND 365),
    game_day INTEGER NOT NULL,
    captured_at_ms INTEGER NOT NULL,
    collection_fingerprint TEXT NOT NULL CHECK(length(collection_fingerprint) = 16),
    facility_count INTEGER NOT NULL CHECK(facility_count BETWEEN 1 AND 25000),
    inserted_at_ms INTEGER NOT NULL
) STRICT;

CREATE INDEX environment_validation_snapshot_recency
    ON environment_validation_snapshots(captured_at_ms DESC, snapshot_id);

CREATE TABLE environment_validation_facilities (
    snapshot_id TEXT NOT NULL REFERENCES environment_validation_snapshots(snapshot_id) ON DELETE CASCADE,
    facility_index INTEGER NOT NULL CHECK(facility_index BETWEEN 0 AND 24999),
    building_type INTEGER NOT NULL CHECK(building_type BETWEEN 0 AND 255),
    building_subtype INTEGER NOT NULL CHECK(building_subtype BETWEEN -1 AND 4096),
    finished INTEGER NOT NULL CHECK(finished IN (0, 1)),
    going_away INTEGER NOT NULL CHECK(going_away IN (0, 1)),
    position_x REAL,
    position_z REAL,
    production REAL,
    pollution REAL,
    radiation REAL,
    water_amount REAL,
    water_capacity REAL,
    water_quality REAL,
    sewage_amount REAL,
    sewage_capacity REAL,
    sewage_quality REAL,
    PRIMARY KEY(snapshot_id, facility_index)
) STRICT;

CREATE TABLE environment_validation_comparisons (
    comparison_id TEXT PRIMARY KEY CHECK(length(comparison_id) = 64),
    snapshot_id TEXT NOT NULL,
    facility_index INTEGER NOT NULL,
    field TEXT NOT NULL CHECK(field IN (
        'production', 'pollution', 'water_amount', 'water_capacity', 'water_quality',
        'sewage_amount', 'sewage_capacity', 'sewage_quality'
    )),
    research_value REAL NOT NULL,
    wr_value REAL NOT NULL,
    control_kind TEXT NOT NULL CHECK(control_kind IN (
        'positive_value', 'zero_value', 'disconnected_facility',
        'consecutive_frame_stability', 'save_reload', 'application_restart'
    )),
    result TEXT NOT NULL CHECK(result IN ('matches', 'does_not_match', 'uncertain')),
    note TEXT CHECK(note IS NULL OR length(note) BETWEEN 1 AND 500),
    game_build_id TEXT NOT NULL,
    probe_version TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(snapshot_id, facility_index)
        REFERENCES environment_validation_facilities(snapshot_id, facility_index)
) STRICT;

CREATE INDEX environment_validation_comparison_recency
    ON environment_validation_comparisons(created_at_ms DESC, comparison_id);
