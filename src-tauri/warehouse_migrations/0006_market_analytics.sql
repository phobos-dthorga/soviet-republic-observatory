CREATE TABLE market_observation_records (
    interpretation_id VARCHAR NOT NULL,
    raw_payload_hash VARCHAR NOT NULL,
    branch_id VARCHAR NOT NULL,
    record_hash VARCHAR NOT NULL,
    ordinal BIGINT NOT NULL,
    record_id BIGINT NOT NULL,
    year INTEGER NOT NULL,
    day INTEGER NOT NULL,
    game_day BIGINT NOT NULL,
    profile_id VARCHAR NOT NULL,
    profile_version VARCHAR NOT NULL,
    resolved_profile_hash VARCHAR NOT NULL,
    mapping_classification VARCHAR NOT NULL,
    PRIMARY KEY (interpretation_id, record_hash)
);

CREATE TABLE market_price_facts (
    interpretation_id VARCHAR NOT NULL,
    record_hash VARCHAR,
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

CREATE TABLE market_trade_facts (
    interpretation_id VARCHAR NOT NULL,
    record_hash VARCHAR,
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

CREATE TABLE market_scalar_facts (
    interpretation_id VARCHAR NOT NULL,
    record_hash VARCHAR,
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

CREATE VIEW market_trade_history AS
SELECT record.interpretation_id, record.branch_id, record.record_hash,
       record.year, record.day, record.game_day,
       trade.currency, trade.direction, trade.channel, trade.resource_token,
       trade.quantity, trade.account_value
FROM market_observation_records record
JOIN market_trade_facts trade
  ON trade.interpretation_id = record.interpretation_id
 AND trade.record_hash = record.record_hash
WHERE trade.scope_kind IS NULL;

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
SELECT record.interpretation_id, record.branch_id, record.record_hash,
       record.year, record.day, record.game_day,
       price.currency, price.price_side, price.resource_token,
       price.value, price.modifier
FROM market_observation_records record
JOIN market_price_facts price
  ON price.interpretation_id = record.interpretation_id
 AND price.record_hash = record.record_hash
WHERE price.scope_kind IS NULL;
