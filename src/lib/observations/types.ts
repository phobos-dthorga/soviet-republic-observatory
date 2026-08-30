export type CoverageStatus = "complete" | "partial";

export type CoverageWarning = {
  code: string;
  count: number;
};

export type CoverageReport = {
  status: CoverageStatus;
  history_records: number;
  chartable_records: number;
  dropped_records: number;
  warnings: CoverageWarning[];
};

export type MetricEvidence = {
  metric_id: string;
  source_field: string;
  latest_source_line: number;
};

export type ReceiverHistoryPoint = {
  record_id: number;
  year: number;
  day: number;
  game_day: number;
  none: number;
  radio: number;
  television: number;
  computer: number;
  classified_total: number;
};

export type ReceiverDataset = {
  payload_hash: string;
  interpretation_id: string;
  source_file_name: string;
  source_file_size: number;
  source_modified_ms: number;
  imported_at_ms: number;
  parser_version: string;
  format_profile: string;
  compatibility: CompatibilityProvenance;
  branch_id: string;
  original_branch_id: string;
  analysis_context_id: string | null;
  geographic_scope: string;
  coverage: CoverageReport;
  source_fields: MetricEvidence[];
  points: ReceiverHistoryPoint[];
};

export type TimelineBranch = {
  branch_id: string;
  branch_kind: "main" | "fork" | "unassigned";
  parent_branch_id: string | null;
  fork_record_id: number | null;
  observation_count: number;
  latest_year: number | null;
  latest_day: number | null;
  selected: boolean;
  origin: "automatic" | "manual_continuation";
  short_identity: string;
  player_label: string | null;
  anchor_interpretation_id: string | null;
  membership_revision: number;
};

export type ArchiveObservation = {
  payload_hash: string;
  interpretation_id: string;
  mapping_classification: string;
  profile_id: string;
  profile_version: string;
  resolved_profile_hash: string;
  source_file_name: string;
  imported_at_ms: number;
  branch_id: string;
  relationship:
    | "root"
    | "successor"
    | "equivalent_history"
    | "rollback_fork"
    | "divergent_fork"
    | "ambiguous"
    | "continuation_anchor";
  parent_payload_hash: string | null;
  shared_record_count: number;
  latest_year: number | null;
  latest_day: number | null;
  history_records: number;
  coverage_status: CoverageStatus;
  file_observation_count: number;
  republic_snapshot_fields: number;
  city_snapshot_count: number;
  city_snapshot_fields: number;
  included_in_context: boolean;
  active_head: boolean;
  context_sequence: number | null;
};

export type AnalysisContext = {
  context_id: string;
  selected_branch_id: string;
  head_interpretation_id: string | null;
  original_branch_id: string | null;
  mode: "latest" | "historical_preview";
  origin: "automatic" | "manual_continuation";
  is_tip: boolean;
  membership_revision: number;
  compatibility_profile_id: string | null;
  compatibility_profile_hash: string | null;
  observation_watermark: string | null;
  catalogue_generation_id: string | null;
  overlay_revision: string | null;
};

export type ArchiveOverview = {
  selected_branch_id: string;
  file_observation_count: number;
  distinct_state_count: number;
  unresolved_state_count: number;
  branches: TimelineBranch[];
  observations: ArchiveObservation[];
  analysis_context: AnalysisContext;
};

export type AnalysisContextResult = {
  archive: ArchiveOverview;
  context: AnalysisContext;
  dataset: ReceiverDataset | null;
};

export type PopulationFact = {
  fact_id: string;
  value: number;
  source_field: string;
  source_line: number;
};

export type PopulationObservation = {
  interpretation_id: string;
  source_file_name: string;
  membership_revision: number;
  sampled_year: number;
  sampled_day: number;
  sampled_game_day: number;
  coverage_status: CoverageStatus;
  mapping_classification: string;
  profile_id: string;
  profile_version: string;
  resolved_profile_hash: string;
  facts: PopulationFact[];
};

export type PopulationCitySnapshot = {
  scope_id: string;
  sampled_year: number;
  sampled_day: number;
  sampled_game_day: number;
  coverage_status: CoverageStatus;
  facts: PopulationFact[];
};

export type PopulationDataset = {
  analysis_context: AnalysisContext;
  observations: PopulationObservation[];
  cities: PopulationCitySnapshot[];
  observation_limit: number;
  city_limit: number;
  tesmio_probe: TesmioProbeStatus;
};

export type TesmioProbeState =
  "not_configured" | "missing" | "available" | "warning" | "invalid";

export type TesmioProbeStatus = {
  state: TesmioProbeState;
  read_only: boolean;
  optional: boolean;
  persisted: boolean;
  probe_id: string | null;
  probe_version: string | null;
  loader_api_version: number | null;
  target_game_version: string | null;
  executable_timestamp: number | null;
  content_hash: string | null;
  snapshot_count: number;
  sample_count: number;
  latest_year: number | null;
  latest_day: number | null;
  latest_population_count: number | null;
  warnings: string[];
};

export type BranchSelectionResult = AnalysisContextResult;

export type ComparisonObservation = {
  payload_hash: string;
  interpretation_id: string;
  source_file_name: string;
  branch_id: string;
  year: number;
  day: number;
  game_day: number;
  coverage_status: CoverageStatus;
  republic_snapshot_fields: number;
  city_snapshot_count: number;
  city_snapshot_fields: number;
};

export type ReceiverClassChange = {
  metric_id: string;
  from_value: number;
  to_value: number;
  delta: number;
};

export type ArchiveComparison = {
  branch_id: string;
  elapsed_game_days: number;
  from: ComparisonObservation;
  to: ComparisonObservation;
  receiver_changes: ReceiverClassChange[];
  classified_total_change: ReceiverClassChange;
};

export type ConfiguredDirectorySummary = {
  name: string;
};

export type GameVocabularySource = {
  source_id: string;
  file_name: string;
  locale_hint?: string;
  format: string;
  readable: boolean;
};

export type SetupState = {
  save_directory?: ConfiguredDirectorySummary;
  game_directory?: ConfiguredDirectorySummary;
  workshop_directory?: ConfiguredDirectorySummary;
  save_candidates: number;
  observed_saves: number;
  distinct_states: number;
  game_vocabularies: GameVocabularySource[];
  automatic_observer: AutomaticObserverStatus;
  compatibility: CompatibilityStatus;
};

export type CompatibilityProvenance = {
  profile_id: string;
  profile_version: string;
  profile_content_hash: string;
  resolved_profile_hash: string;
  base_profile_hash: string | null;
  profile_source: "reviewed_builtin" | "local_override";
  mapping_classification: "reviewed_mapping" | "player_mapped";
  parser_engine_version: string;
};

export type CompatibilityProfileSummary = {
  id: string;
  version: string;
  content_hash: string;
  resolved_hash: string;
  source: "reviewed_builtin" | "local_override";
  mapping_classification: "reviewed_mapping" | "player_mapped";
  base_profile_id: string | null;
  base_profile_version: string | null;
  base_profile_hash: string | null;
  target_game_versions: string[];
  target_build_ids: string[];
  target_stats_formats: number[];
};

export type CompatibilityStatus = {
  active: CompatibilityProfileSummary;
  reviewed_base: CompatibilityProfileSummary;
  local_file_path: string;
  local_file_exists: boolean;
  local_validation: "missing" | "valid" | "invalid";
  last_validation_error: string | null;
  last_validated_at_ms: number | null;
  detected_game_version: string | null;
  detected_build_id: string | null;
  coverage: {
    stats_markers: number;
    stats_fields: number;
    definition_operations: number;
    binary_layouts: number;
    catalogue_scopes: number;
  };
  catalogue_scopes: CompatibilityCatalogueScopeStatus[];
};

export type CompatibilityCatalogueScopeStatus = {
  id: string;
  source_id: string;
  package_name: string | null;
  update_policy: "exact" | "track_updates";
  acknowledged_content_hash: string;
  current_content_hash: string | null;
  mapping_count: number;
  state: "matched" | "dormant" | "updated_unreviewed" | "conflict";
};

export type CompatibilityUpdate = {
  status: CompatibilityStatus;
  profile_changed: boolean;
  definition_mapping_changed: boolean;
};

export type ReinterpretationProgress = {
  phase:
    | "idle"
    | "reading"
    | "parsing"
    | "persisting"
    | "queueing_warehouse"
    | "complete"
    | "failed";
  progress_percent: number | null;
  started_at_ms: number | null;
  updated_at_ms: number | null;
  current_file: string | null;
  interpretation_id: string | null;
  error_code: string | null;
};

export type ImportOutcome = "imported" | "duplicate";

export type ObservationImportResult = {
  outcome: ImportOutcome;
  recorded_interpretation_id: string;
  active_context_id: string;
  dataset: ReceiverDataset;
};

export type AutomaticObserverPhase =
  | "disabled"
  | "not_configured"
  | "watching"
  | "waiting_for_stability"
  | "retrying"
  | "observed"
  | "failed";

export type AutomaticObserverStatus = {
  enabled: boolean;
  phase: AutomaticObserverPhase;
  candidate_file_name: string | null;
  retry_attempt: number;
  error_code: string | null;
  last_observed_file_name: string | null;
  last_observed_at_ms: number | null;
};

export type RecorderCandidateStatus =
  | "discovered"
  | "stabilising"
  | "ready"
  | "reading"
  | "imported"
  | "duplicate"
  | "retryable_failure"
  | "terminal_failure"
  | "superseded";

export type RecorderDiscoverySource =
  "migration" | "initial_scan" | "filesystem_event" | "reconciliation";

export type RecorderLedgerEntry = {
  candidate_id: number;
  file_name: string;
  file_size: number;
  source_modified_ms: number;
  status: RecorderCandidateStatus;
  discovery_source: RecorderDiscoverySource;
  discovered_at_ms: number;
  first_stable_at_ms: number | null;
  last_attempt_at_ms: number | null;
  completed_at_ms: number | null;
  attempt_count: number;
  error_code: string | null;
  import_outcome: ImportOutcome | null;
  payload_hash: string | null;
  processing_latency_ms: number | null;
};

export type RecorderHealth = {
  observer: AutomaticObserverStatus;
  last_scan_ms: number | null;
  last_filesystem_event_ms: number | null;
  last_completed_at_ms: number | null;
  last_completed_file_name: string | null;
  last_processing_latency_ms: number | null;
  queue_depth: number;
  attention_count: number;
  completed_count: number;
  latest_entries: RecorderLedgerEntry[];
};

export type RecorderUpdate = {
  health: RecorderHealth;
  import_result: ObservationImportResult | null;
};

export type DirectoryKind = "save" | "game" | "workshop";

export type ObserverErrorCode =
  | "invalid_directory"
  | "invalid_game_directory"
  | "save_directory_not_configured"
  | "no_save_candidate"
  | "invalid_save_candidate"
  | "save_changed_during_read"
  | "invalid_archive"
  | "missing_stats_payload"
  | "duplicate_stats_payload"
  | "stats_payload_too_large"
  | "invalid_stats_encoding"
  | "stats_line_too_long"
  | "unsupported_stats_format"
  | "malformed_receiver_history"
  | "malformed_snapshot"
  | "receiver_history_unavailable"
  | "storage_unavailable"
  | "unknown_branch"
  | "incompatible_comparison"
  | "same_observation_comparison"
  | "unknown_observation"
  | "warehouse_write_limit"
  | "invalid_compatibility_profile"
  | "binary_compatibility_mismatch"
  | "critical_task_busy"
  | "unknown";

export type WarehousePhase = "ready" | "lagging" | "rebuilding" | "attention";

export type WarehouseWriteActivity = {
  kind:
    | "catalogue_publication"
    | "observation_projection"
    | "overlay_projection"
    | "branch_membership_projection"
    | "observation_rebuild";
  stage: "staging" | "merging" | "committing" | "rebuilding";
  started_at_ms: number;
  updated_at_ms: number;
  rows_processed: number;
  rows_total: number;
};

export type WarehouseHealth = {
  phase: WarehousePhase;
  schema_version: number;
  pending_jobs: number;
  failed_jobs: number;
  lag_ms: number | null;
  last_projected_at_ms: number | null;
  observation_watermark: string | null;
  database_size_bytes: number;
  active_write: WarehouseWriteActivity | null;
  consecutive_write_failures: number;
  retry_after_ms: number | null;
};

export type CatalogueGenerationSummary = {
  generation_id: string;
  game_build_id: string | null;
  parser_version: string;
  created_at_ms: number;
  source_count: number;
  file_count: number;
  entity_count: number;
  property_count: number;
  relation_count: number;
  warning_count: number;
  compatibility_profile_id: string;
  compatibility_profile_version: string;
  compatibility_profile_hash: string;
  mapping_classification: string;
};

export type CatalogueRefreshPhase =
  | "idle"
  | "discovering"
  | "scanning"
  | "publishing"
  | "finalising"
  | "complete"
  | "failed";

export type CatalogueRefreshProgress = {
  phase: CatalogueRefreshPhase;
  trigger: "startup" | "filesystem" | "manual";
  progress_percent: number | null;
  started_at_ms: number | null;
  updated_at_ms: number | null;
  current_source: string | null;
  current_file: string | null;
  current_file_index: number | null;
  sources_discovered: number;
  sources_total: number;
  files_discovered: number;
  files_processed: number;
  files_reused: number;
  files_parsed: number;
  entities_prepared: number;
  rows_written: number;
  rows_total: number;
  error_code: string | null;
};

export type OverlayProfileSummary = {
  profile_id: string;
  display_name: string;
  active_revision: number | null;
  latest_revision: number;
  revision_count: number;
  semantic_version: string;
  content_hash: string;
  conflict_count: number;
  active: boolean;
};

export type CatalogueStatus = {
  warehouse: WarehouseHealth;
  generation: CatalogueGenerationSummary | null;
  last_checked_at_ms: number | null;
  last_refreshed_at_ms: number | null;
  last_filesystem_event_ms: number | null;
  error_code: string | null;
  active_overlay: OverlayProfileSummary | null;
  refresh: CatalogueRefreshProgress;
};

export type DiagnosticEntry = {
  occurred_at_ms: number;
  level: string;
  code: string;
  operation: string;
  message: string;
};

export type DiagnosticLogView = {
  language: string;
  storage: string;
  entries: DiagnosticEntry[];
};

export type CatalogueSearchFilter = {
  query?: string;
  entity_kind?: "resource" | "building" | "vehicle" | "recipe";
  source_kind?: string;
  package_query?: string;
  coverage?: "complete" | "partial";
  available_year?: number;
  limit?: number;
  offset?: number;
};

export type DefinitionSummary = {
  entity_id: string;
  revision_hash: string;
  entity_kind: string;
  source_id: string;
  source_kind: string;
  package_name: string;
  display_name: string;
  coverage: string;
  property_count: number;
  relation_count: number;
};

export type CataloguePage = {
  total: number;
  limit: number;
  offset: number;
  items: DefinitionSummary[];
};

export type DefinitionValue = {
  value_kind: string;
  number: number | null;
  text: string | null;
  unit: string | null;
};

export type DefinitionFact = {
  field_id: string;
  occurrence: number;
  original: DefinitionValue | null;
  override_value: DefinitionValue | null;
  effective: DefinitionValue | null;
  source_directive: string;
  source_line: number;
  raw_arguments: string;
  evidence_kind: string;
  resolution: string;
  conflict_code: string | null;
  mapping: DefinitionMappingProvenance;
};

export type DefinitionRelation = {
  relation_kind: string;
  occurrence: number;
  target_id: string;
  quantity: number | null;
  unit: string | null;
  phase_id: string | null;
  source_directive: string;
  source_line: number;
  raw_arguments: string;
  resolution: string;
  mapping: DefinitionMappingProvenance;
};

export type DefinitionMappingProvenance = {
  mapping_id: string;
  catalogue_scope_id: string | null;
  mapping_classification: string;
  scope_state: "matched" | "dormant" | "updated_unreviewed" | "conflict" | null;
  update_policy: "exact" | "track_updates" | null;
  acknowledged_content_hash: string | null;
  current_content_hash: string | null;
};

export type DefinitionDossier = {
  summary: DefinitionSummary;
  facts: DefinitionFact[];
  relations: DefinitionRelation[];
  unknown_directives: Array<{ directive: string; occurrence_count: number }>;
};

export type ProductionRouteStatus =
  | "ready"
  | "ready_with_auxiliary"
  | "too_complex"
  | "no_output"
  | "no_input"
  | "missing_quantity"
  | "invalid_quantity"
  | "missing_unit"
  | "no_comparable_input"
  | "duplicate_endpoint";

export type ProductionRouteRequest = {
  entity_id: string;
  output_resource_id?: string;
  target_quantity?: number;
};

export type ProductionRouteFlow = {
  id: string;
  direction: "production_input" | "production_output" | "waste_input";
  resource_id: string;
  display_name: string;
  source_quantity: number | null;
  scaled_quantity: number | null;
  unit: string | null;
  basis_role: "primary" | "auxiliary";
  basis_exclusion: "different_unit" | "missing_unit" | null;
  resolution: string;
  source_directive: string;
  source_line: number;
  mapping: DefinitionMappingProvenance;
};

export type ProductionRouteModel = {
  schema_version: number;
  route_id: string;
  revision_hash: string;
  building_entity_id: string | null;
  display_name: string;
  package_name: string;
  coverage: string;
  status: ProductionRouteStatus;
  relation_count: number;
  primary_flow_count: number;
  auxiliary_flow_count: number;
  unit: string | null;
  selected_output_resource_id: string | null;
  target_quantity: number | null;
  scale_factor: number | null;
  mapping_classification: string;
  flows: ProductionRouteFlow[];
  snapshot: WarehouseSnapshot;
};

export type ProductionRouteCoverage = {
  schema_version: number;
  route_count: number;
  diagrammable_count: number;
  routes_with_auxiliary: number;
  unavailable_count: number;
  relation_count: number;
  auxiliary_relation_count: number;
  unresolved_basis_relation_count: number;
  unquantified_relation_count: number;
  snapshot: WarehouseSnapshot;
};

export type OverlayInspection = {
  valid: boolean;
  code: string | null;
  profile: OverlayProfileSummary | null;
  operation_count: number;
  supplement_count: number;
  document: unknown | null;
};

export type WarehouseSnapshot = {
  catalogue_generation_id: string;
  compatibility_profile_id: string;
  compatibility_profile_version: string;
  compatibility_profile_hash: string;
  mapping_classification: string;
  overlay_profile_id: string | null;
  overlay_revision: number | null;
  observation_watermark: string | null;
  warehouse_schema_version: number;
  projector_version: string;
};
