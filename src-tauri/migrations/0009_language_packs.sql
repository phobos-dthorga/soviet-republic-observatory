CREATE TABLE language_pack_manifests (
    pack_id TEXT PRIMARY KEY NOT NULL
        CHECK(length(pack_id) BETWEEN 3 AND 64),
    content_hash TEXT NOT NULL
        CHECK(length(content_hash) = 64),
    manifest_json TEXT NOT NULL
        CHECK(length(CAST(manifest_json AS BLOB)) <= 262144),
    locale TEXT NOT NULL
        CHECK(length(locale) BETWEEN 2 AND 64),
    display_name TEXT NOT NULL
        CHECK(length(display_name) BETWEEN 1 AND 80),
    author TEXT
        CHECK(author IS NULL OR length(author) BETWEEN 1 AND 80),
    source_catalog_version INTEGER NOT NULL
        CHECK(source_catalog_version >= 1),
    source_catalog_revision INTEGER NOT NULL
        CHECK(source_catalog_revision >= 1),
    direction TEXT NOT NULL
        CHECK(direction IN ('left_to_right', 'right_to_left')),
    translated_message_count INTEGER NOT NULL
        CHECK(translated_message_count BETWEEN 1 AND 2048),
    installed_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE language_preferences (
    singleton_id INTEGER PRIMARY KEY NOT NULL CHECK(singleton_id = 1),
    selected_pack_id TEXT NOT NULL DEFAULT 'observatory-en-au',
    legacy_handover_completed_at_ms INTEGER,
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT INTO language_preferences(
    singleton_id,
    selected_pack_id,
    legacy_handover_completed_at_ms,
    updated_at_ms
) VALUES(1, 'observatory-en-au', NULL, 0);
