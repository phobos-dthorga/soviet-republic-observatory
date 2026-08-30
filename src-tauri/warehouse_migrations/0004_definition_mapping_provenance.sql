ALTER TABLE definition_properties ADD COLUMN mapping_id VARCHAR;
ALTER TABLE definition_properties ADD COLUMN catalogue_scope_id VARCHAR;
ALTER TABLE definition_properties ADD COLUMN mapping_classification VARCHAR;

UPDATE definition_properties SET
    mapping_id = 'legacy.reviewed.definition',
    mapping_classification = 'reviewed_mapping'
WHERE mapping_id IS NULL;

ALTER TABLE definition_relations ADD COLUMN mapping_id VARCHAR;
ALTER TABLE definition_relations ADD COLUMN catalogue_scope_id VARCHAR;
ALTER TABLE definition_relations ADD COLUMN mapping_classification VARCHAR;

UPDATE definition_relations SET
    mapping_id = 'legacy.reviewed.definition',
    mapping_classification = 'reviewed_mapping'
WHERE mapping_id IS NULL;

CREATE TABLE catalogue_scope_evaluations (
    generation_id VARCHAR NOT NULL,
    scope_id VARCHAR NOT NULL,
    source_id VARCHAR NOT NULL,
    package_name VARCHAR,
    update_policy VARCHAR NOT NULL,
    acknowledged_content_hash VARCHAR NOT NULL,
    current_content_hash VARCHAR,
    mapping_count BIGINT NOT NULL,
    state VARCHAR NOT NULL,
    PRIMARY KEY (generation_id, scope_id)
);
