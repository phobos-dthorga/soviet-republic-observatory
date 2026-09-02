CREATE TABLE resource_registry_snapshots (
    snapshot_id VARCHAR PRIMARY KEY,
    assurance VARCHAR NOT NULL,
    game_build_id VARCHAR NOT NULL,
    probe_version VARCHAR NOT NULL,
    loader_api_version INTEGER NOT NULL,
    captured_year INTEGER NOT NULL,
    captured_day INTEGER NOT NULL,
    captured_at_ms BIGINT NOT NULL,
    resource_count INTEGER NOT NULL
);

CREATE TABLE resource_registry_entries (
    snapshot_id VARCHAR NOT NULL,
    live_index INTEGER NOT NULL,
    source_token VARCHAR NOT NULL,
    display_name VARCHAR NOT NULL,
    label_source VARCHAR NOT NULL,
    caption_id BIGINT,
    resource_kind INTEGER,
    transport_class_mask INTEGER NOT NULL,
    material_family INTEGER,
    runtime_extension BOOLEAN NOT NULL,
    PRIMARY KEY(snapshot_id, live_index)
);

CREATE TABLE resource_registry_prices (
    snapshot_id VARCHAR NOT NULL,
    live_index INTEGER NOT NULL,
    currency VARCHAR NOT NULL,
    finished_price DOUBLE NOT NULL,
    base_price DOUBLE NOT NULL,
    buy_multiplier DOUBLE NOT NULL,
    sell_multiplier DOUBLE NOT NULL,
    buy_quote DOUBLE NOT NULL,
    sell_quote DOUBLE NOT NULL,
    PRIMARY KEY(snapshot_id, live_index, currency)
);

CREATE VIEW latest_resource_registry_entries AS
SELECT entry.*, snapshot.assurance, snapshot.game_build_id,
       snapshot.captured_year, snapshot.captured_day, snapshot.captured_at_ms
FROM resource_registry_entries entry
JOIN resource_registry_snapshots snapshot USING(snapshot_id)
QUALIFY row_number() OVER (
    PARTITION BY entry.source_token
    ORDER BY snapshot.captured_at_ms DESC, snapshot.snapshot_id DESC
) = 1;
