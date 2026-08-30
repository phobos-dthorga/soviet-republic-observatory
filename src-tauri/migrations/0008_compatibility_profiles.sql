CREATE TABLE compatibility_profile_revisions (
    profile_id TEXT NOT NULL CHECK (length(profile_id) BETWEEN 3 AND 96),
    semantic_version TEXT NOT NULL CHECK (length(semantic_version) BETWEEN 5 AND 32),
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64),
    resolved_hash TEXT NOT NULL UNIQUE CHECK (length(resolved_hash) = 64),
    base_profile_hash TEXT CHECK (base_profile_hash IS NULL OR length(base_profile_hash) = 64),
    profile_source TEXT NOT NULL CHECK (
        profile_source IN ('reviewed_builtin', 'local_override')
    ),
    mapping_classification TEXT NOT NULL CHECK (
        mapping_classification IN ('reviewed_mapping', 'player_mapped')
    ),
    parser_engine_version TEXT NOT NULL,
    document_json TEXT NOT NULL CHECK (length(document_json) <= 1048576),
    validated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (profile_id, semantic_version, content_hash)
) STRICT;

CREATE TABLE compatibility_runtime_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    active_resolved_hash TEXT,
    local_file_exists INTEGER NOT NULL DEFAULT 0 CHECK (local_file_exists IN (0, 1)),
    local_validation TEXT NOT NULL DEFAULT 'missing' CHECK (
        local_validation IN ('missing', 'valid', 'invalid')
    ),
    last_validation_error TEXT,
    last_validated_at_ms INTEGER
) STRICT;

INSERT INTO compatibility_runtime_state(
    singleton_id, active_resolved_hash, local_file_exists, local_validation,
    last_validation_error, last_validated_at_ms
) VALUES(1, NULL, 0, 'missing', NULL, NULL);

ALTER TABLE observation_sources ADD COLUMN raw_payload_hash TEXT;
ALTER TABLE observation_sources ADD COLUMN interpretation_id TEXT;
ALTER TABLE observation_sources ADD COLUMN profile_id TEXT;
ALTER TABLE observation_sources ADD COLUMN profile_semantic_version TEXT;
ALTER TABLE observation_sources ADD COLUMN profile_content_hash TEXT;
ALTER TABLE observation_sources ADD COLUMN resolved_profile_hash TEXT;
ALTER TABLE observation_sources ADD COLUMN base_profile_hash TEXT;
ALTER TABLE observation_sources ADD COLUMN profile_source TEXT;
ALTER TABLE observation_sources ADD COLUMN mapping_classification TEXT;
ALTER TABLE observation_sources ADD COLUMN parser_engine_version TEXT;

CREATE UNIQUE INDEX observation_sources_interpretation_id
    ON observation_sources(interpretation_id);

CREATE UNIQUE INDEX observation_sources_raw_profile
    ON observation_sources(raw_payload_hash, resolved_profile_hash);

CREATE INDEX observation_sources_mapping_classification
    ON observation_sources(mapping_classification, imported_at_ms DESC);

CREATE TABLE binary_mapped_facts (
    payload_hash TEXT NOT NULL
        REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    layout_id TEXT NOT NULL CHECK (length(layout_id) BETWEEN 3 AND 64),
    record_index INTEGER NOT NULL CHECK (record_index >= 0),
    host_slot TEXT NOT NULL CHECK (length(host_slot) BETWEEN 3 AND 96),
    value_real REAL,
    available INTEGER NOT NULL CHECK (available IN (0, 1)),
    source_offset INTEGER NOT NULL CHECK (source_offset >= 0),
    evidence_kind TEXT NOT NULL CHECK (evidence_kind = 'save_fact'),
    PRIMARY KEY (payload_hash, layout_id, record_index, host_slot),
    CHECK ((available = 0 AND value_real IS NULL) OR available = 1)
) STRICT;
