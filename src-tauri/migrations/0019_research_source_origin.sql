ALTER TABLE research_setup_state
ADD COLUMN tesmio_source_origin TEXT
    CHECK (tesmio_source_origin IS NULL OR tesmio_source_origin IN (
        'manual_checkout',
        'observatory_downloaded'
    ));

UPDATE research_setup_state
SET tesmio_source_origin = 'manual_checkout'
WHERE tesmio_checkout_path IS NOT NULL
  AND tesmio_source_origin IS NULL;
