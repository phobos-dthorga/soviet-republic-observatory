CREATE TABLE IF NOT EXISTS resource_registry_ingestion_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    assurance TEXT CHECK (
        assurance IS NULL OR assurance IN ('verified_observation_only', 'player_managed_modded')
    ),
    acknowledged_notice_revision INTEGER NOT NULL DEFAULT 0
        CHECK (acknowledged_notice_revision BETWEEN 0 AND 1000000),
    last_ingested_snapshot_id TEXT CHECK (
        last_ingested_snapshot_id IS NULL OR (
            length(last_ingested_snapshot_id) = 64 AND
            last_ingested_snapshot_id NOT GLOB '*[^0-9a-f]*'
        )
    ),
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT OR IGNORE INTO resource_registry_ingestion_state(
    singleton_id, enabled, assurance, acknowledged_notice_revision,
    last_ingested_snapshot_id, updated_at_ms
) VALUES(1, 0, NULL, 0, NULL, 0);

CREATE TABLE IF NOT EXISTS resource_registry_snapshots (
    snapshot_id TEXT PRIMARY KEY CHECK (length(snapshot_id) = 64),
    source_content_hash TEXT NOT NULL CHECK (length(source_content_hash) = 64),
    assurance TEXT NOT NULL CHECK (
        assurance IN ('verified_observation_only', 'player_managed_modded')
    ),
    game_build_id TEXT NOT NULL CHECK (length(game_build_id) BETWEEN 3 AND 96),
    probe_version TEXT NOT NULL CHECK (length(probe_version) BETWEEN 1 AND 32),
    loader_api_version INTEGER NOT NULL CHECK (loader_api_version BETWEEN 1 AND 1000),
    executable_timestamp INTEGER NOT NULL CHECK (executable_timestamp > 0),
    executable_size INTEGER NOT NULL CHECK (executable_size > 0),
    captured_year INTEGER NOT NULL CHECK (captured_year BETWEEN 1900 AND 10000),
    captured_day INTEGER NOT NULL CHECK (captured_day BETWEEN 0 AND 365),
    captured_at_ms INTEGER NOT NULL,
    resource_count INTEGER NOT NULL CHECK (resource_count BETWEEN 1 AND 512),
    storage_contract_version INTEGER NOT NULL CHECK (storage_contract_version BETWEEN 1 AND 1000)
) STRICT;

CREATE TABLE IF NOT EXISTS resource_registry_entries (
    snapshot_id TEXT NOT NULL REFERENCES resource_registry_snapshots(snapshot_id) ON DELETE RESTRICT,
    live_index INTEGER NOT NULL CHECK (live_index BETWEEN 0 AND 511),
    source_token TEXT NOT NULL CHECK (length(source_token) BETWEEN 1 AND 128),
    caption_id INTEGER NOT NULL CHECK (caption_id BETWEEN 0 AND 4294967295),
    resolved_caption TEXT CHECK (resolved_caption IS NULL OR length(resolved_caption) BETWEEN 1 AND 320),
    label_source_id TEXT CHECK (label_source_id IS NULL OR length(label_source_id) BETWEEN 1 AND 160),
    resource_kind INTEGER NOT NULL CHECK (resource_kind BETWEEN -64 AND 64),
    transport_class_mask INTEGER NOT NULL CHECK (transport_class_mask BETWEEN 0 AND 262143),
    material_family INTEGER NOT NULL CHECK (material_family BETWEEN -1 AND 255),
    PRIMARY KEY(snapshot_id, live_index),
    UNIQUE(snapshot_id, source_token)
) STRICT;

CREATE TABLE IF NOT EXISTS resource_registry_prices (
    snapshot_id TEXT NOT NULL,
    live_index INTEGER NOT NULL,
    currency TEXT NOT NULL CHECK (currency IN ('RUB', 'USD')),
    finished_price REAL NOT NULL CHECK (finished_price >= 0.0 AND finished_price <= 1000000000000.0),
    base_price REAL NOT NULL CHECK (base_price >= 0.0 AND base_price <= 1000000000000.0),
    buy_multiplier REAL NOT NULL CHECK (buy_multiplier >= 0.0 AND buy_multiplier <= 100.0),
    sell_multiplier REAL NOT NULL CHECK (sell_multiplier >= 0.0 AND sell_multiplier <= 100.0),
    buy_quote REAL NOT NULL CHECK (buy_quote >= 0.0 AND buy_quote <= 100000000000000.0),
    sell_quote REAL NOT NULL CHECK (sell_quote >= 0.0 AND sell_quote <= 100000000000000.0),
    PRIMARY KEY(snapshot_id, live_index, currency),
    FOREIGN KEY(snapshot_id, live_index)
        REFERENCES resource_registry_entries(snapshot_id, live_index) ON DELETE RESTRICT
) STRICT;

CREATE TABLE IF NOT EXISTS resource_vocabulary_revisions (
    snapshot_id TEXT NOT NULL REFERENCES resource_registry_snapshots(snapshot_id) ON DELETE RESTRICT,
    source_id TEXT NOT NULL CHECK (length(source_id) BETWEEN 3 AND 160),
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64),
    entry_count INTEGER NOT NULL CHECK (entry_count BETWEEN 0 AND 100000),
    warning_count INTEGER NOT NULL CHECK (warning_count BETWEEN 0 AND 100000),
    PRIMARY KEY(snapshot_id, source_id)
) STRICT;

CREATE TABLE IF NOT EXISTS resource_registry_ingestion_receipts (
    snapshot_id TEXT PRIMARY KEY REFERENCES resource_registry_snapshots(snapshot_id) ON DELETE RESTRICT,
    source_content_hash TEXT NOT NULL CHECK (length(source_content_hash) = 64),
    ingested_at_ms INTEGER NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS resource_registry_entries_token
    ON resource_registry_entries(source_token, snapshot_id);

ALTER TABLE warehouse_projection_jobs RENAME TO warehouse_projection_jobs_v21;

CREATE TABLE warehouse_projection_jobs (
    projection_id TEXT PRIMARY KEY
        CHECK (length(projection_id) BETWEEN 3 AND 160),
    projection_kind TEXT NOT NULL CHECK (
        projection_kind IN (
            'observation', 'market_observation', 'broadcast_observation',
            'resource_registry_snapshot', 'overlay_state', 'branch_membership', 'rebuild'
        )
    ),
    source_identity TEXT NOT NULL
        CHECK (length(source_identity) BETWEEN 1 AND 256),
    status TEXT NOT NULL CHECK (
        status IN ('pending', 'running', 'applied', 'failed')
    ),
    requested_at_ms INTEGER NOT NULL,
    started_at_ms INTEGER,
    applied_at_ms INTEGER,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    error_code TEXT,
    CHECK ((status = 'applied' AND applied_at_ms IS NOT NULL) OR status <> 'applied')
) STRICT;

INSERT INTO warehouse_projection_jobs(
    projection_id, projection_kind, source_identity, status, requested_at_ms,
    started_at_ms, applied_at_ms, attempt_count, error_code
)
SELECT projection_id, projection_kind, source_identity, status, requested_at_ms,
       started_at_ms, applied_at_ms, attempt_count, error_code
FROM warehouse_projection_jobs_v21;

DROP TABLE warehouse_projection_jobs_v21;

CREATE INDEX warehouse_projection_jobs_queue
    ON warehouse_projection_jobs(status, requested_at_ms, projection_id);
