CREATE TABLE broadcast_status_records (
    record_hash VARCHAR PRIMARY KEY,
    record_id BIGINT NOT NULL,
    year INTEGER NOT NULL,
    day INTEGER NOT NULL,
    game_day BIGINT NOT NULL
);

CREATE TABLE broadcast_status_observation_records (
    interpretation_id VARCHAR NOT NULL,
    raw_payload_hash VARCHAR NOT NULL,
    branch_id VARCHAR NOT NULL,
    record_hash VARCHAR NOT NULL,
    ordinal BIGINT NOT NULL,
    profile_id VARCHAR NOT NULL,
    profile_version VARCHAR NOT NULL,
    resolved_profile_hash VARCHAR NOT NULL,
    mapping_classification VARCHAR NOT NULL,
    PRIMARY KEY(interpretation_id, ordinal)
);

CREATE TABLE broadcast_status_facts (
    record_hash VARCHAR NOT NULL,
    source_index INTEGER NOT NULL,
    metric_id VARCHAR NOT NULL,
    value_real DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL,
    PRIMARY KEY(record_hash, source_index)
);

CREATE VIEW broadcast_status_history AS
SELECT membership.interpretation_id, membership.raw_payload_hash,
       membership.branch_id, membership.ordinal, membership.profile_id,
       membership.profile_version, membership.resolved_profile_hash,
       membership.mapping_classification, record.record_hash, record.record_id,
       record.year, record.day, record.game_day, fact.source_index,
       fact.metric_id, fact.value_real, fact.source_field, fact.source_line,
       fact.mapping_id
FROM broadcast_status_observation_records membership
JOIN broadcast_status_records record USING(record_hash)
JOIN broadcast_status_facts fact USING(record_hash);
