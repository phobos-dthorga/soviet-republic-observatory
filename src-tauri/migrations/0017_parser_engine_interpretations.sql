DROP INDEX IF EXISTS observation_sources_raw_profile;
DROP INDEX IF EXISTS observation_sources_raw_engine_profile;

CREATE UNIQUE INDEX observation_sources_raw_engine_profile
    ON observation_sources(
        raw_payload_hash,
        parser_engine_version,
        resolved_profile_hash
    );
