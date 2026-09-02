CREATE TABLE environment_activity_records (
    record_hash VARCHAR PRIMARY KEY,
    record_id UINTEGER NOT NULL,
    year INTEGER NOT NULL,
    day USMALLINT NOT NULL,
    game_day BIGINT NOT NULL
);

CREATE TABLE environment_activity_observation_records (
    interpretation_id VARCHAR NOT NULL,
    raw_payload_hash VARCHAR NOT NULL,
    branch_id VARCHAR NOT NULL,
    record_hash VARCHAR NOT NULL,
    ordinal UINTEGER NOT NULL,
    profile_id VARCHAR NOT NULL,
    profile_version VARCHAR NOT NULL,
    resolved_profile_hash VARCHAR NOT NULL,
    mapping_classification VARCHAR NOT NULL,
    PRIMARY KEY(interpretation_id, ordinal)
);

CREATE TABLE environment_activity_facts (
    record_hash VARCHAR NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line UBIGINT NOT NULL,
    row_ordinal UINTEGER NOT NULL,
    resource_token VARCHAR NOT NULL,
    activity_channel VARCHAR NOT NULL,
    primary_value DOUBLE NOT NULL,
    secondary_value DOUBLE NOT NULL,
    quantity_is_publishable BOOLEAN NOT NULL,
    mapping_id VARCHAR NOT NULL,
    PRIMARY KEY(record_hash, source_field, row_ordinal)
);

CREATE INDEX environment_activity_resource
    ON environment_activity_facts(resource_token, activity_channel);

CREATE OR REPLACE VIEW environment_activity_history AS
SELECT membership.interpretation_id,
       membership.raw_payload_hash,
       membership.branch_id,
       membership.ordinal,
       record.record_id,
       record.year,
       record.day,
       record.game_day,
       fact.source_field,
       fact.source_line,
       fact.row_ordinal,
       fact.resource_token,
       fact.activity_channel,
       fact.primary_value,
       fact.secondary_value,
       fact.quantity_is_publishable,
       fact.mapping_id,
       membership.profile_id,
       membership.profile_version,
       membership.resolved_profile_hash,
       membership.mapping_classification
FROM environment_activity_observation_records membership
JOIN environment_activity_records record USING(record_hash)
JOIN environment_activity_facts fact USING(record_hash);
