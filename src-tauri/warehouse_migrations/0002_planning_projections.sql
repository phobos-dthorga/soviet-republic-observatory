CREATE VIEW original_value_projection AS
SELECT membership.generation_id, membership.entity_id, properties.field_id,
       properties.occurrence, properties.value_kind, properties.value_number,
       properties.value_text, properties.unit, properties.evidence_kind,
       properties.resolution
FROM catalogue_generation_entities membership
JOIN definition_properties properties USING (revision_hash);

CREATE VIEW overlay_value_projection AS
SELECT metadata.current_catalogue_generation_id AS generation_id,
       operations.entity_id, operations.field_id,
       COALESCE(operations.occurrence, 0) AS occurrence,
       operations.operation, operations.value_kind, operations.value_number,
       operations.value_text, operations.unit, operations.reason,
       operations.conflict_code
FROM active_overlay_operations operations
CROSS JOIN warehouse_metadata metadata
WHERE metadata.singleton_id = 1;

CREATE VIEW effective_value_projection AS
SELECT originals.generation_id, originals.entity_id, originals.field_id,
       originals.occurrence,
       CASE WHEN overlays.operation = 'unset' THEN NULL
            WHEN overlays.operation = 'set' THEN overlays.value_kind
            ELSE originals.value_kind END AS value_kind,
       CASE WHEN overlays.operation = 'unset' THEN NULL
            WHEN overlays.operation = 'set' THEN overlays.value_number
            ELSE originals.value_number END AS value_number,
       CASE WHEN overlays.operation = 'unset' THEN NULL
            WHEN overlays.operation = 'set' THEN overlays.value_text
            ELSE originals.value_text END AS value_text,
       CASE WHEN overlays.operation = 'unset' THEN NULL
            WHEN overlays.operation = 'set' THEN overlays.unit
            ELSE originals.unit END AS unit,
       CASE WHEN overlays.operation IN ('set', 'unset') THEN 'player_override'
            ELSE originals.evidence_kind END AS evidence_kind
FROM original_value_projection originals
LEFT JOIN overlay_value_projection overlays
  ON overlays.generation_id = originals.generation_id
 AND overlays.entity_id = originals.entity_id
 AND overlays.field_id = originals.field_id
 AND overlays.occurrence = originals.occurrence
 AND overlays.conflict_code IS NULL
 AND overlays.operation IN ('set', 'unset')
UNION ALL
SELECT overlays.generation_id, overlays.entity_id, overlays.field_id,
       overlays.occurrence, overlays.value_kind, overlays.value_number,
       overlays.value_text, overlays.unit, 'player_override'
FROM overlay_value_projection overlays
WHERE overlays.operation = 'add' AND overlays.conflict_code IS NULL;

CREATE VIEW material_flows AS
SELECT * FROM production_edges;

CREATE VIEW observation_time_series AS
SELECT payload_hash, branch_id, record_id, year, day, game_day, metric_id, value
FROM observation_metrics;
