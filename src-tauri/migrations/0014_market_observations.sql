CREATE TABLE market_observation_coverage (
    payload_hash TEXT PRIMARY KEY REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    coverage_status TEXT NOT NULL CHECK(coverage_status IN ('complete', 'partial')),
    history_records INTEGER NOT NULL CHECK(history_records >= 0),
    snapshot_scopes INTEGER NOT NULL CHECK(snapshot_scopes >= 0),
    row_count INTEGER NOT NULL CHECK(row_count >= 0),
    warnings_json TEXT NOT NULL
) STRICT;

CREATE TABLE market_records (
    record_hash TEXT PRIMARY KEY,
    record_id INTEGER NOT NULL CHECK(record_id >= 0),
    year INTEGER NOT NULL,
    day INTEGER NOT NULL CHECK(day >= 0 AND day < 365),
    game_day INTEGER NOT NULL
) STRICT;

CREATE TABLE market_observation_records (
    payload_hash TEXT NOT NULL REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    record_hash TEXT NOT NULL REFERENCES market_records(record_hash),
    PRIMARY KEY(payload_hash, ordinal),
    UNIQUE(payload_hash, record_hash)
) STRICT;

CREATE INDEX market_observation_records_record
    ON market_observation_records(record_hash);

CREATE TABLE market_price_facts (
    record_hash TEXT NOT NULL REFERENCES market_records(record_hash) ON DELETE CASCADE,
    currency TEXT NOT NULL CHECK(currency IN ('rub', 'usd')),
    price_side TEXT NOT NULL CHECK(price_side IN ('purchase', 'sell', 'base')),
    resource_token TEXT NOT NULL,
    value_real REAL NOT NULL,
    modifier_real REAL NOT NULL,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(record_hash, currency, price_side, resource_token)
) STRICT;

CREATE TABLE market_trade_facts (
    record_hash TEXT NOT NULL REFERENCES market_records(record_hash) ON DELETE CASCADE,
    currency TEXT NOT NULL CHECK(currency IN ('rub', 'usd')),
    direction TEXT NOT NULL CHECK(direction IN ('import', 'export')),
    channel TEXT NOT NULL CHECK(channel IN ('standard', 'international')),
    resource_token TEXT NOT NULL,
    quantity_real REAL NOT NULL,
    account_value_real REAL NOT NULL,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(record_hash, currency, direction, channel, resource_token)
) STRICT;

CREATE TABLE market_scalar_facts (
    record_hash TEXT NOT NULL REFERENCES market_records(record_hash) ON DELETE CASCADE,
    fact_id TEXT NOT NULL,
    currency TEXT CHECK(currency IS NULL OR currency IN ('rub', 'usd')),
    category INTEGER,
    value_real REAL NOT NULL,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(record_hash, fact_id, category, source_line)
) STRICT;

CREATE TABLE market_snapshot_scopes (
    payload_hash TEXT NOT NULL REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    scope_kind TEXT NOT NULL CHECK(scope_kind IN ('republic', 'city')),
    scope_id TEXT NOT NULL,
    PRIMARY KEY(payload_hash, scope_kind, scope_id)
) STRICT;

CREATE TABLE market_snapshot_price_facts (
    payload_hash TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    currency TEXT NOT NULL CHECK(currency IN ('rub', 'usd')),
    price_side TEXT NOT NULL CHECK(price_side IN ('purchase', 'sell', 'base')),
    resource_token TEXT NOT NULL,
    value_real REAL NOT NULL,
    modifier_real REAL NOT NULL,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(payload_hash, scope_kind, scope_id, currency, price_side, resource_token),
    FOREIGN KEY(payload_hash, scope_kind, scope_id)
        REFERENCES market_snapshot_scopes(payload_hash, scope_kind, scope_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE market_snapshot_trade_facts (
    payload_hash TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    currency TEXT NOT NULL CHECK(currency IN ('rub', 'usd')),
    direction TEXT NOT NULL CHECK(direction IN ('import', 'export')),
    channel TEXT NOT NULL CHECK(channel IN ('standard', 'international')),
    resource_token TEXT NOT NULL,
    quantity_real REAL NOT NULL,
    account_value_real REAL NOT NULL,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(payload_hash, scope_kind, scope_id, currency, direction, channel, resource_token),
    FOREIGN KEY(payload_hash, scope_kind, scope_id)
        REFERENCES market_snapshot_scopes(payload_hash, scope_kind, scope_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE market_snapshot_scalar_facts (
    payload_hash TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    fact_id TEXT NOT NULL,
    currency TEXT CHECK(currency IS NULL OR currency IN ('rub', 'usd')),
    category INTEGER,
    value_real REAL NOT NULL,
    source_field TEXT NOT NULL,
    source_line INTEGER NOT NULL CHECK(source_line > 0),
    mapping_id TEXT NOT NULL,
    PRIMARY KEY(payload_hash, scope_kind, scope_id, fact_id, category, source_line),
    FOREIGN KEY(payload_hash, scope_kind, scope_id)
        REFERENCES market_snapshot_scopes(payload_hash, scope_kind, scope_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE market_interpretation_variants (
    raw_payload_hash TEXT NOT NULL,
    interpretation_id TEXT NOT NULL REFERENCES observation_sources(interpretation_id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    resolved_profile_hash TEXT NOT NULL,
    indexed_at_ms INTEGER NOT NULL,
    PRIMARY KEY(raw_payload_hash, interpretation_id)
) STRICT;

CREATE TABLE market_indexing_jobs (
    job_id TEXT PRIMARY KEY,
    state TEXT NOT NULL CHECK(state IN ('pending', 'running', 'complete', 'failed')),
    started_at_ms INTEGER,
    completed_at_ms INTEGER,
    total_archives INTEGER NOT NULL DEFAULT 0 CHECK(total_archives >= 0),
    completed_archives INTEGER NOT NULL DEFAULT 0 CHECK(completed_archives >= 0),
    missing_archives INTEGER NOT NULL DEFAULT 0 CHECK(missing_archives >= 0),
    changed_archives INTEGER NOT NULL DEFAULT 0 CHECK(changed_archives >= 0),
    failed_archives INTEGER NOT NULL DEFAULT 0 CHECK(failed_archives >= 0),
    duplicate_archives INTEGER NOT NULL DEFAULT 0 CHECK(duplicate_archives >= 0),
    last_error_code TEXT
) STRICT;

CREATE TABLE market_indexing_items (
    job_id TEXT NOT NULL REFERENCES market_indexing_jobs(job_id) ON DELETE CASCADE,
    payload_hash TEXT NOT NULL REFERENCES observation_sources(payload_hash) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK(state IN ('pending', 'running', 'complete', 'missing', 'changed', 'failed', 'duplicate')),
    records_processed INTEGER NOT NULL DEFAULT 0 CHECK(records_processed >= 0),
    rows_processed INTEGER NOT NULL DEFAULT 0 CHECK(rows_processed >= 0),
    error_code TEXT,
    PRIMARY KEY(job_id, payload_hash)
) STRICT;

CREATE TABLE market_basket_revisions (
    basket_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    name TEXT NOT NULL,
    currency TEXT NOT NULL CHECK(currency IN ('rub', 'usd')),
    price_side TEXT NOT NULL CHECK(price_side IN ('purchase', 'sell')),
    base_record_hash TEXT NOT NULL REFERENCES market_records(record_hash),
    reason TEXT NOT NULL,
    weights_json TEXT NOT NULL,
    built_in INTEGER NOT NULL CHECK(built_in IN (0, 1)),
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY(basket_id, revision)
) STRICT;

CREATE TABLE market_scenario_revisions (
    scenario_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    name TEXT NOT NULL,
    scenario_kind TEXT NOT NULL CHECK(scenario_kind IN ('break_even', 'debt_stress')),
    reason TEXT NOT NULL,
    assumptions_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY(scenario_id, revision)
) STRICT;
