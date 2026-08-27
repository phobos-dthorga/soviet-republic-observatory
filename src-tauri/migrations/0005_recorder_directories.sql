CREATE TABLE IF NOT EXISTS recorder_directories (
    source_directory_identity TEXT PRIMARY KEY
        CHECK (length(source_directory_identity) = 64),
    initialised_at_ms INTEGER NOT NULL
) STRICT;
