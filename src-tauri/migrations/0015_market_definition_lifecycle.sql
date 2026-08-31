CREATE TABLE market_active_selections (
    selection_kind TEXT PRIMARY KEY CHECK(selection_kind IN ('basket', 'scenario')),
    definition_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    selected_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE market_definition_lifecycle (
    definition_kind TEXT NOT NULL CHECK(definition_kind IN ('basket', 'scenario')),
    definition_id TEXT NOT NULL,
    removed_at_ms INTEGER,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(definition_kind, definition_id)
) STRICT;
