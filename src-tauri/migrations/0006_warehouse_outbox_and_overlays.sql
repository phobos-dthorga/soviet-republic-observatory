CREATE TABLE warehouse_projection_jobs (
    projection_id TEXT PRIMARY KEY
        CHECK (length(projection_id) BETWEEN 3 AND 160),
    projection_kind TEXT NOT NULL CHECK (
        projection_kind IN ('observation', 'overlay_state', 'rebuild')
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
    CHECK (
        (status = 'applied' AND applied_at_ms IS NOT NULL) OR
        status <> 'applied'
    )
) STRICT;

CREATE INDEX warehouse_projection_jobs_queue
    ON warehouse_projection_jobs(status, requested_at_ms, projection_id);

INSERT OR IGNORE INTO warehouse_projection_jobs(
    projection_id,
    projection_kind,
    source_identity,
    status,
    requested_at_ms
)
SELECT
    'observation:' || payload_hash,
    'observation',
    payload_hash,
    'pending',
    imported_at_ms
FROM observation_sources;

CREATE TABLE planning_overlay_profiles (
    profile_id TEXT PRIMARY KEY
        CHECK (length(profile_id) BETWEEN 3 AND 96),
    display_name TEXT NOT NULL
        CHECK (length(display_name) BETWEEN 1 AND 120),
    active_revision INTEGER,
    removed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE planning_overlay_revisions (
    profile_id TEXT NOT NULL
        REFERENCES planning_overlay_profiles(profile_id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    content_hash TEXT NOT NULL UNIQUE CHECK (length(content_hash) = 64),
    semantic_version TEXT NOT NULL CHECK (length(semantic_version) BETWEEN 5 AND 32),
    author TEXT NOT NULL CHECK (length(author) BETWEEN 1 AND 120),
    default_locale TEXT NOT NULL CHECK (length(default_locale) BETWEEN 2 AND 32),
    description TEXT NOT NULL CHECK (length(description) BETWEEN 1 AND 500),
    document_json TEXT NOT NULL CHECK (length(document_json) <= 1048576),
    installed_at_ms INTEGER NOT NULL,
    PRIMARY KEY (profile_id, revision)
) STRICT;

CREATE INDEX planning_overlay_revisions_profile
    ON planning_overlay_revisions(profile_id, revision DESC);

CREATE TABLE planning_overlay_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    active_profile_id TEXT REFERENCES planning_overlay_profiles(profile_id),
    active_revision INTEGER,
    CHECK (
        (active_profile_id IS NULL AND active_revision IS NULL) OR
        (active_profile_id IS NOT NULL AND active_revision IS NOT NULL)
    )
) STRICT;

INSERT INTO planning_overlay_state(singleton_id, active_profile_id, active_revision)
VALUES(1, NULL, NULL);

CREATE TABLE catalogue_runtime_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    last_filesystem_event_ms INTEGER,
    last_refresh_requested_ms INTEGER,
    last_refresh_error_code TEXT
) STRICT;

INSERT INTO catalogue_runtime_state(
    singleton_id, last_filesystem_event_ms, last_refresh_requested_ms,
    last_refresh_error_code
) VALUES(1, NULL, NULL, NULL);
