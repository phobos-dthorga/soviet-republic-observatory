CREATE TABLE warehouse_metadata (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    current_catalogue_generation_id VARCHAR,
    active_overlay_profile_id VARCHAR,
    active_overlay_revision BIGINT,
    last_catalogue_check_ms BIGINT,
    last_catalogue_refresh_ms BIGINT,
    last_catalogue_error_code VARCHAR,
    last_projection_ms BIGINT,
    observation_watermark VARCHAR
);

INSERT INTO warehouse_metadata(singleton_id) VALUES(1);

CREATE TABLE projection_receipts (
    projection_id VARCHAR PRIMARY KEY,
    projection_kind VARCHAR NOT NULL,
    source_identity VARCHAR NOT NULL,
    applied_at_ms BIGINT NOT NULL
);

CREATE TABLE catalogue_generations (
    generation_id VARCHAR PRIMARY KEY,
    game_build_id VARCHAR,
    parser_version VARCHAR NOT NULL,
    created_at_ms BIGINT NOT NULL,
    source_count BIGINT NOT NULL,
    file_count BIGINT NOT NULL,
    entity_count BIGINT NOT NULL,
    property_count BIGINT NOT NULL,
    relation_count BIGINT NOT NULL,
    warning_count BIGINT NOT NULL
);

CREATE TABLE catalogue_sources (
    generation_id VARCHAR NOT NULL,
    source_id VARCHAR NOT NULL,
    source_kind VARCHAR NOT NULL,
    package_name VARCHAR NOT NULL,
    package_version VARCHAR,
    content_hash VARCHAR NOT NULL,
    file_count BIGINT NOT NULL,
    PRIMARY KEY (generation_id, source_id)
);

CREATE TABLE catalogue_files (
    generation_id VARCHAR NOT NULL,
    source_id VARCHAR NOT NULL,
    logical_path VARCHAR NOT NULL,
    content_hash VARCHAR NOT NULL,
    byte_size BIGINT NOT NULL,
    parser_profile VARCHAR NOT NULL,
    warning_count BIGINT NOT NULL,
    PRIMARY KEY (generation_id, source_id, logical_path)
);

CREATE TABLE definition_entity_revisions (
    revision_hash VARCHAR PRIMARY KEY,
    entity_kind VARCHAR NOT NULL,
    source_id VARCHAR NOT NULL,
    source_object_id VARCHAR NOT NULL,
    display_name VARCHAR NOT NULL,
    coverage VARCHAR NOT NULL
);

CREATE TABLE catalogue_generation_entities (
    generation_id VARCHAR NOT NULL,
    entity_id VARCHAR NOT NULL,
    revision_hash VARCHAR NOT NULL,
    PRIMARY KEY (generation_id, entity_id)
);

CREATE TABLE definition_properties (
    revision_hash VARCHAR NOT NULL,
    field_id VARCHAR NOT NULL,
    occurrence BIGINT NOT NULL,
    value_kind VARCHAR NOT NULL,
    value_number DOUBLE,
    value_text VARCHAR,
    unit VARCHAR,
    source_directive VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    raw_arguments VARCHAR NOT NULL,
    evidence_kind VARCHAR NOT NULL,
    resolution VARCHAR NOT NULL,
    PRIMARY KEY (revision_hash, field_id, occurrence)
);

CREATE TABLE definition_relations (
    revision_hash VARCHAR NOT NULL,
    relation_kind VARCHAR NOT NULL,
    occurrence BIGINT NOT NULL,
    target_id VARCHAR NOT NULL,
    quantity DOUBLE,
    unit VARCHAR,
    phase_id VARCHAR,
    source_directive VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    raw_arguments VARCHAR NOT NULL,
    resolution VARCHAR NOT NULL,
    PRIMARY KEY (revision_hash, relation_kind, occurrence)
);

CREATE TABLE definition_unknown_directives (
    revision_hash VARCHAR NOT NULL,
    directive VARCHAR NOT NULL,
    occurrence_count BIGINT NOT NULL,
    PRIMARY KEY (revision_hash, directive)
);

CREATE TABLE observation_metrics (
    payload_hash VARCHAR NOT NULL,
    branch_id VARCHAR NOT NULL,
    record_id BIGINT NOT NULL,
    year INTEGER NOT NULL,
    day INTEGER NOT NULL,
    game_day BIGINT NOT NULL,
    metric_id VARCHAR NOT NULL,
    value BIGINT NOT NULL,
    PRIMARY KEY (payload_hash, record_id, metric_id)
);

CREATE TABLE active_overlay_operations (
    profile_id VARCHAR NOT NULL,
    revision BIGINT NOT NULL,
    operation_index BIGINT NOT NULL,
    operation VARCHAR NOT NULL,
    entity_id VARCHAR NOT NULL,
    field_id VARCHAR NOT NULL,
    occurrence BIGINT,
    expected_revision_hash VARCHAR,
    value_kind VARCHAR,
    value_number DOUBLE,
    value_text VARCHAR,
    unit VARCHAR,
    reason VARCHAR NOT NULL,
    conflict_code VARCHAR,
    PRIMARY KEY (profile_id, revision, operation_index)
);

CREATE TABLE active_overlay_entities (
    profile_id VARCHAR NOT NULL,
    revision BIGINT NOT NULL,
    entity_id VARCHAR NOT NULL,
    entity_kind VARCHAR NOT NULL,
    display_name VARCHAR NOT NULL,
    reason VARCHAR NOT NULL,
    properties_json VARCHAR NOT NULL,
    PRIMARY KEY (profile_id, revision, entity_id)
);

CREATE VIEW current_catalogue_entities AS
SELECT membership.entity_id, revisions.*
FROM catalogue_generation_entities membership
JOIN definition_entity_revisions revisions USING (revision_hash)
JOIN warehouse_metadata metadata ON metadata.singleton_id = 1
WHERE membership.generation_id = metadata.current_catalogue_generation_id;

CREATE VIEW production_edges AS
SELECT membership.generation_id, membership.entity_id, relations.target_id,
       relations.quantity, relations.unit, relations.relation_kind
FROM catalogue_generation_entities membership
JOIN definition_relations relations USING (revision_hash)
WHERE relations.relation_kind IN ('production_output', 'production_input');

CREATE VIEW construction_demand AS
SELECT membership.generation_id, membership.entity_id, relations.target_id,
       relations.quantity, relations.unit, relations.phase_id,
       relations.relation_kind, relations.resolution
FROM catalogue_generation_entities membership
JOIN definition_relations relations USING (revision_hash)
WHERE relations.relation_kind IN ('construction_material_explicit',
                                  'construction_material_auto');

CREATE VIEW fleet_capabilities AS
SELECT membership.generation_id, membership.entity_id, properties.field_id,
       properties.value_number, properties.value_text, properties.unit
FROM catalogue_generation_entities membership
JOIN definition_properties properties USING (revision_hash)
WHERE properties.field_id LIKE 'vehicle.%';
