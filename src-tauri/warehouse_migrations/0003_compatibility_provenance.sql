ALTER TABLE observation_metrics ADD COLUMN interpretation_id VARCHAR;
ALTER TABLE observation_metrics ADD COLUMN raw_payload_hash VARCHAR;
ALTER TABLE observation_metrics ADD COLUMN profile_id VARCHAR;
ALTER TABLE observation_metrics ADD COLUMN profile_version VARCHAR;
ALTER TABLE observation_metrics ADD COLUMN profile_content_hash VARCHAR;
ALTER TABLE observation_metrics ADD COLUMN resolved_profile_hash VARCHAR;
ALTER TABLE observation_metrics ADD COLUMN mapping_classification VARCHAR;

UPDATE observation_metrics SET
    interpretation_id = payload_hash,
    raw_payload_hash = payload_hash,
    profile_id = 'org.republic-observatory.wrsr-1.1.1.9',
    profile_version = '1.0.0',
    profile_content_hash = '0f2737d29ddb50aa22a32d6fb1747e7c0ec5aa00227464a38d68b4ae1bac522e',
    resolved_profile_hash = '0f2737d29ddb50aa22a32d6fb1747e7c0ec5aa00227464a38d68b4ae1bac522e',
    mapping_classification = 'reviewed_mapping'
WHERE interpretation_id IS NULL;

ALTER TABLE catalogue_generations ADD COLUMN compatibility_profile_id VARCHAR;
ALTER TABLE catalogue_generations ADD COLUMN compatibility_profile_version VARCHAR;
ALTER TABLE catalogue_generations ADD COLUMN compatibility_profile_hash VARCHAR;
ALTER TABLE catalogue_generations ADD COLUMN mapping_classification VARCHAR;

UPDATE catalogue_generations SET
    compatibility_profile_id = 'org.republic-observatory.wrsr-1.1.1.9',
    compatibility_profile_version = '1.0.0',
    compatibility_profile_hash = '0f2737d29ddb50aa22a32d6fb1747e7c0ec5aa00227464a38d68b4ae1bac522e',
    mapping_classification = 'reviewed_mapping'
WHERE compatibility_profile_id IS NULL;

CREATE OR REPLACE VIEW observation_time_series AS
SELECT interpretation_id, raw_payload_hash, branch_id, record_id, year, day,
       game_day, metric_id, value, profile_id, profile_version,
       resolved_profile_hash, mapping_classification
FROM observation_metrics;
