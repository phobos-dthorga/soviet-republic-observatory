CREATE TABLE analysis_pack_profiles (
    pack_id TEXT PRIMARY KEY
        CHECK (length(pack_id) BETWEEN 3 AND 128),
    display_name TEXT NOT NULL
        CHECK (length(display_name) BETWEEN 1 AND 80),
    active_revision INTEGER,
    removed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE analysis_pack_revisions (
    pack_id TEXT NOT NULL
        REFERENCES analysis_pack_profiles(pack_id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    content_hash TEXT NOT NULL UNIQUE CHECK (length(content_hash) = 64),
    semantic_version TEXT NOT NULL CHECK (length(semantic_version) BETWEEN 5 AND 64),
    host_api_version INTEGER NOT NULL CHECK (host_api_version > 0),
    author TEXT NOT NULL CHECK (length(author) BETWEEN 1 AND 120),
    default_locale TEXT NOT NULL CHECK (length(default_locale) BETWEEN 2 AND 64),
    description TEXT NOT NULL CHECK (length(description) BETWEEN 1 AND 500),
    derived_metric_count INTEGER NOT NULL CHECK (derived_metric_count BETWEEN 0 AND 64),
    chart_count INTEGER NOT NULL CHECK (chart_count BETWEEN 0 AND 16),
    document_json TEXT NOT NULL CHECK (length(document_json) <= 524288),
    installed_at_ms INTEGER NOT NULL,
    PRIMARY KEY (pack_id, revision)
) STRICT;

CREATE INDEX analysis_pack_revisions_profile
    ON analysis_pack_revisions(pack_id, revision DESC);

CREATE INDEX analysis_pack_profiles_enabled
    ON analysis_pack_profiles(active_revision, removed_at_ms);
