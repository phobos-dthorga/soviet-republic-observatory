CREATE TABLE attention_cue_dismissals (
    cue_id TEXT NOT NULL
        CHECK (length(cue_id) BETWEEN 3 AND 96),
    content_revision INTEGER NOT NULL
        CHECK (content_revision BETWEEN 1 AND 1000000),
    dismissed_at_ms INTEGER NOT NULL,
    PRIMARY KEY (cue_id, content_revision)
) STRICT;

CREATE TABLE research_setup_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    tesmio_checkout_path TEXT,
    accepted_notice_revision INTEGER NOT NULL DEFAULT 0
        CHECK (accepted_notice_revision BETWEEN 0 AND 1000000),
    last_probe_hash TEXT CHECK (
        last_probe_hash IS NULL OR (
            length(last_probe_hash) = 64 AND
            last_probe_hash NOT GLOB '*[^0-9a-f]*'
        )
    ),
    last_built_at_ms INTEGER,
    updated_at_ms INTEGER NOT NULL
) STRICT;

INSERT INTO research_setup_state(
    singleton_id, tesmio_checkout_path, accepted_notice_revision,
    last_probe_hash, last_built_at_ms, updated_at_ms
) VALUES(1, NULL, 0, NULL, NULL, 0);
