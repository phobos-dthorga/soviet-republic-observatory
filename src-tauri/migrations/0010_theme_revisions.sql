CREATE TABLE theme_revisions (
    theme_id TEXT NOT NULL,
    semantic_version TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    display_name TEXT NOT NULL,
    author TEXT,
    description TEXT,
    installed_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY(theme_id, semantic_version),
    UNIQUE(content_hash)
) STRICT;

CREATE TABLE theme_preferences (
    singleton_id INTEGER PRIMARY KEY NOT NULL CHECK(singleton_id = 1),
    selected_theme_id TEXT NOT NULL,
    selected_version TEXT NOT NULL,
    selected_content_hash TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT INTO theme_preferences(
    singleton_id,
    selected_theme_id,
    selected_version,
    selected_content_hash,
    updated_at_ms
) VALUES(
    1,
    'org.republic-observatory.classic',
    '1.0.0',
    '',
    0
);
