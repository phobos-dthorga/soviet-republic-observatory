DROP VIEW market_trade_totals;
DROP VIEW market_positive_export_concentration;
DROP VIEW market_trade_history;
DROP VIEW market_price_history;

ALTER TABLE market_observation_records RENAME TO market_observation_records_v6;
ALTER TABLE market_price_facts RENAME TO market_price_facts_v6;
ALTER TABLE market_trade_facts RENAME TO market_trade_facts_v6;
ALTER TABLE market_scalar_facts RENAME TO market_scalar_facts_v6;

CREATE TABLE market_records (
    record_hash VARCHAR PRIMARY KEY,
    record_id BIGINT NOT NULL,
    year INTEGER NOT NULL,
    day INTEGER NOT NULL,
    game_day BIGINT NOT NULL
);

CREATE TABLE market_observation_records (
    interpretation_id VARCHAR NOT NULL,
    raw_payload_hash VARCHAR NOT NULL,
    branch_id VARCHAR NOT NULL,
    record_hash VARCHAR NOT NULL,
    ordinal BIGINT NOT NULL,
    profile_id VARCHAR NOT NULL,
    profile_version VARCHAR NOT NULL,
    resolved_profile_hash VARCHAR NOT NULL,
    mapping_classification VARCHAR NOT NULL,
    PRIMARY KEY (interpretation_id, record_hash)
);

CREATE TABLE market_price_facts (
    record_hash VARCHAR NOT NULL,
    currency VARCHAR NOT NULL,
    price_side VARCHAR NOT NULL,
    resource_token VARCHAR NOT NULL,
    value DOUBLE NOT NULL,
    modifier DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL
);

CREATE TABLE market_trade_facts (
    record_hash VARCHAR NOT NULL,
    currency VARCHAR NOT NULL,
    direction VARCHAR NOT NULL,
    channel VARCHAR NOT NULL,
    resource_token VARCHAR NOT NULL,
    quantity DOUBLE NOT NULL,
    account_value DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL
);

CREATE TABLE market_scalar_facts (
    record_hash VARCHAR NOT NULL,
    fact_id VARCHAR NOT NULL,
    currency VARCHAR,
    category INTEGER,
    value DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL
);

CREATE TABLE market_snapshot_price_facts (
    interpretation_id VARCHAR NOT NULL,
    scope_kind VARCHAR,
    scope_id VARCHAR,
    currency VARCHAR NOT NULL,
    price_side VARCHAR NOT NULL,
    resource_token VARCHAR NOT NULL,
    value DOUBLE NOT NULL,
    modifier DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL
);

CREATE TABLE market_snapshot_trade_facts (
    interpretation_id VARCHAR NOT NULL,
    scope_kind VARCHAR,
    scope_id VARCHAR,
    currency VARCHAR NOT NULL,
    direction VARCHAR NOT NULL,
    channel VARCHAR NOT NULL,
    resource_token VARCHAR NOT NULL,
    quantity DOUBLE NOT NULL,
    account_value DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL
);

CREATE TABLE market_snapshot_scalar_facts (
    interpretation_id VARCHAR NOT NULL,
    scope_kind VARCHAR,
    scope_id VARCHAR,
    fact_id VARCHAR NOT NULL,
    currency VARCHAR,
    category INTEGER,
    value DOUBLE NOT NULL,
    source_field VARCHAR NOT NULL,
    source_line BIGINT NOT NULL,
    mapping_id VARCHAR NOT NULL
);

INSERT INTO market_records
SELECT DISTINCT record_hash, record_id, year, day, game_day
FROM market_observation_records_v6;

INSERT INTO market_observation_records
SELECT interpretation_id, raw_payload_hash, branch_id, record_hash, ordinal,
       profile_id, profile_version, resolved_profile_hash, mapping_classification
FROM market_observation_records_v6;

INSERT INTO market_price_facts
SELECT DISTINCT record_hash, currency, price_side, resource_token, value, modifier,
       source_field, source_line, mapping_id
FROM market_price_facts_v6 WHERE record_hash IS NOT NULL;
INSERT INTO market_snapshot_price_facts
SELECT interpretation_id, scope_kind, scope_id, currency, price_side,
       resource_token, value, modifier, source_field, source_line, mapping_id
FROM market_price_facts_v6 WHERE record_hash IS NULL;

INSERT INTO market_trade_facts
SELECT DISTINCT record_hash, currency, direction, channel, resource_token,
       quantity, account_value, source_field, source_line, mapping_id
FROM market_trade_facts_v6 WHERE record_hash IS NOT NULL;
INSERT INTO market_snapshot_trade_facts
SELECT interpretation_id, scope_kind, scope_id, currency, direction, channel,
       resource_token, quantity, account_value, source_field, source_line, mapping_id
FROM market_trade_facts_v6 WHERE record_hash IS NULL;

INSERT INTO market_scalar_facts
SELECT DISTINCT record_hash, fact_id, currency, category, value, source_field,
       source_line, mapping_id
FROM market_scalar_facts_v6 WHERE record_hash IS NOT NULL;
INSERT INTO market_snapshot_scalar_facts
SELECT interpretation_id, scope_kind, scope_id, fact_id, currency, category,
       value, source_field, source_line, mapping_id
FROM market_scalar_facts_v6 WHERE record_hash IS NULL;

DROP TABLE market_observation_records_v6;
DROP TABLE market_price_facts_v6;
DROP TABLE market_trade_facts_v6;
DROP TABLE market_scalar_facts_v6;

CREATE VIEW market_trade_history AS
SELECT membership.interpretation_id, membership.branch_id, record.record_hash,
       record.year, record.day, record.game_day,
       trade.currency, trade.direction, trade.channel, trade.resource_token,
       trade.quantity, trade.account_value
FROM market_observation_records membership
JOIN market_records record USING(record_hash)
JOIN market_trade_facts trade USING(record_hash);

CREATE VIEW market_trade_totals AS
SELECT interpretation_id, branch_id, record_hash, year, day, game_day,
       currency, channel, direction, SUM(account_value) AS account_value
FROM market_trade_history
GROUP BY interpretation_id, branch_id, record_hash, year, day, game_day,
         currency, channel, direction;

CREATE VIEW market_positive_export_concentration AS
SELECT interpretation_id, branch_id, record_hash, year, day, game_day,
       currency, channel, resource_token, account_value
FROM market_trade_history
WHERE direction = 'export' AND account_value > 0;

CREATE VIEW market_price_history AS
SELECT membership.interpretation_id, membership.branch_id, record.record_hash,
       record.year, record.day, record.game_day,
       price.currency, price.price_side, price.resource_token,
       price.value, price.modifier
FROM market_observation_records membership
JOIN market_records record USING(record_hash)
JOIN market_price_facts price USING(record_hash);
