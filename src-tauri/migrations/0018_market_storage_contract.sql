ALTER TABLE market_observation_coverage
    ADD COLUMN storage_contract_version INTEGER NOT NULL DEFAULT 1
    CHECK(storage_contract_version > 0);
