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

export type ExactObservationReference = {
  interpretation_id: string;
  branch_id: string;
  year: number;
  day: number;
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
  exact_observation: ExactObservationReference | null;
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

export type BroadcastMetricDefinition = {
  metric_id: string;
  source_index: number;
};

export type CitizenStatusPoint = {
  ordinal: number;
  record_id: number;
  year: number;
  day: number;
  game_day: number;
  values: number[];
  source_fields: string[];
  source_lines: number[];
  exact_observation: ExactObservationReference | null;
};

export type BroadcastReceiverClassPulse = {
  metric_id: string;
  count: number;
  share_percent: number;
  change_from_previous: number | null;
};

export type BroadcastPulse = {
  year: number;
  day: number;
  classified_population: number;
  classes: BroadcastReceiverClassPulse[];
};

export type BroadcastStationRequirement = {
  station_kind: string;
  catalogue_entity_id: string;
  workers: number;
  professors: number;
};

export type BroadcastWorkspaceModel = {
  analysis_context: AnalysisContext;
  receiver: ReceiverDataset | null;
  pulse: BroadcastPulse | null;
  status_metrics: BroadcastMetricDefinition[];
  status_coverage: CoverageReport | null;
  citizen_status_points: CitizenStatusPoint[];
  station_requirements: BroadcastStationRequirement[];
  availability: {
    potential_audience: boolean;
    current_audience: boolean;
    programme_settings: boolean;
    demographic_receiver_join: boolean;
  };
  warehouse_projection_available: boolean;
};

export type BroadcastOutcomeRequest = {
  receiver_metric_id: string;
  status_metric_id: string;
  lag_confirmed_records: 0 | 1 | 2 | 4 | 8;
};

export type BroadcastOutcomeAvailability =
  | "available"
  | "receiver_unavailable"
  | "status_unavailable"
  | "insufficient_pairs"
  | "constant_receiver_changes"
  | "constant_status_changes";

export type BroadcastOutcomePair = {
  receiver_record_id: number;
  receiver_year: number;
  receiver_day: number;
  receiver_game_day: number;
  status_record_id: number;
  status_year: number;
  status_day: number;
  status_game_day: number;
  elapsed_game_days: number;
  receiver_share_change: number;
  status_change: number;
  exact_observation: ExactObservationReference | null;
};

export type BroadcastOutcomeModel = {
  availability: BroadcastOutcomeAvailability;
  receiver_metric_id: string;
  status_metric_id: string;
  lag_confirmed_records: number;
  coefficient: number | null;
  pair_count: number;
  start_year: number | null;
  start_day: number | null;
  end_year: number | null;
  end_day: number | null;
  elapsed_days_median: number | null;
  elapsed_days_min: number | null;
  elapsed_days_max: number | null;
  pairs: BroadcastOutcomePair[];
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
  resource_catalogue_revision_id: string | null;
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
  exact_observation: ExactObservationReference | null;
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

export type BriefMetricRole = "headline" | "education" | "receiver_class";

export type BriefEvidenceKind = "save_fact" | "calculation";

export type BriefSourceEvidence = {
  source_field: string;
  source_line: number;
};

export type MetricPopulationBasis =
  | "all_recorded_citizens"
  | "source_defined_adults"
  | "source_defined_small_children"
  | "source_defined_unemployed"
  | "source_defined_movement_counter"
  | "source_defined_citizen_status"
  | "classified_receiver_population";

export type MetricTimeBasis =
  "exact_selected_observation" | "branch_observations_through_selected_head";

export type MetricGeographicScope = "whole_republic";

export type MetricComparisonBasis =
  "proven_preceding_same_branch_and_profile" | "player_plan_schedule";

export type MetricContextLimitation =
  | "not_employment_count"
  | "not_workers_only"
  | "source_age_boundary_unverified"
  | "source_window_unverified"
  | "excludes_unclassified_citizens"
  | "not_interval_flow";

export type MetricContext = {
  population_basis: MetricPopulationBasis;
  time_basis: MetricTimeBasis;
  geographic_scope: MetricGeographicScope;
  denominator_metric_id: string | null;
  comparison_basis: MetricComparisonBasis;
  limitations: MetricContextLimitation[];
};

export type PublishedMetricContext = {
  metric_id: string;
  exact: MetricContext;
  history: MetricContext;
};

export type PlanScheduleKind = "linear" | "milestone" | "hold_then_change";
export type PlanDirection = "increase" | "decrease" | "maintain";
export type PlanTargetState =
  | "awaiting_start"
  | "ahead"
  | "on_track"
  | "behind"
  | "complete"
  | "unavailable";

export type PlanTargetDraft = {
  metric_id: string;
  target_value: number;
  direction: PlanDirection;
  guardrail_basis_points: number;
};

export type RepublicPlanDraft = {
  plan_id: string | null;
  name: string;
  end_year: number;
  end_day: number;
  schedule: PlanScheduleKind;
  targets: PlanTargetDraft[];
};

export type RepublicPlanTarget = PlanTargetDraft & {
  baseline_value: number;
};

export type RepublicPlanRevision = {
  plan_id: string;
  name: string;
  revision: number;
  branch_id: string;
  start_interpretation_id: string;
  start_profile_hash: string;
  start_year: number;
  start_day: number;
  start_game_day: number;
  end_year: number;
  end_day: number;
  end_game_day: number;
  schedule: PlanScheduleKind;
  created_at_ms: number;
  targets: RepublicPlanTarget[];
};

export type RepublicPlanListItem = {
  plan_id: string;
  name: string;
  branch_id: string;
  active_revision: number;
  latest_revision: number;
  revision_count: number;
  selected: boolean;
};

export type PlanMetricOption = {
  metric_id: string;
  current_value: number | null;
  active_plan_baseline_value: number | null;
  context: MetricContext;
};

export type PlanSeriesPoint = {
  year: number;
  day: number;
  game_day: number;
  observed_value: number;
  scheduled_value: number;
  exact_observation: ExactObservationReference | null;
};

export type PlanTargetEvaluation = {
  target: RepublicPlanTarget;
  current_value: number | null;
  scheduled_value: number | null;
  directional_variance: number | null;
  attainment_basis_points: number | null;
  guardrail_breached: boolean;
  state: PlanTargetState;
  context: MetricContext;
  points: PlanSeriesPoint[];
};

export type RepublicPlanEvaluation = {
  revision: RepublicPlanRevision;
  state: PlanTargetState;
  attainment_basis_points: number | null;
  guardrail_breach_count: number;
  targets: PlanTargetEvaluation[];
};

export type RepublicPlanWorkspace = {
  analysis_context: AnalysisContext;
  current_year: number | null;
  current_day: number | null;
  available_metrics: PlanMetricOption[];
  plans: RepublicPlanListItem[];
  active_plan: RepublicPlanEvaluation | null;
};

export type RepublicPlanBrief = {
  plan_id: string;
  name: string;
  revision: number;
  target_count: number;
  end_year: number;
  end_day: number;
  state: PlanTargetState;
  attainment_basis_points: number | null;
  guardrail_breach_count: number;
};

export type BriefMetric = {
  metric_id: string;
  role: BriefMetricRole;
  value: number;
  previous_value: number | null;
  delta: number | null;
  share_basis_points: number | null;
  evidence_kind: BriefEvidenceKind;
  sources: BriefSourceEvidence[];
  context: MetricContext;
};

export type BriefObservation = {
  interpretation_id: string;
  source_file_name: string;
  year: number;
  day: number;
  game_day: number;
  coverage_status: CoverageStatus;
  mapping_classification: string;
  profile_id: string;
  profile_version: string;
  resolved_profile_hash: string;
};

export type BriefComparisonAnchor = {
  interpretation_id: string;
  source_file_name: string;
  year: number;
  day: number;
  game_day: number;
};

export type BriefFindingSeverity = "information" | "watch" | "attention";

export type BriefFindingCode =
  | "no_observation"
  | "historical_preview"
  | "partial_coverage"
  | "player_mapping"
  | "mapping_changed"
  | "no_prior_observation"
  | "missing_metrics"
  | "recorder_attention"
  | "recorder_queue"
  | "warehouse_attention"
  | "warehouse_lagging"
  | "catalogue_unavailable";

export type BriefFinding = {
  code: BriefFindingCode | string;
  severity: BriefFindingSeverity;
  value: number | null;
  metric_id: string | null;
};

export type BriefOperations = {
  recorder_phase: AutomaticObserverPhase | null;
  recorder_queue_depth: number | null;
  recorder_attention_count: number | null;
  warehouse_phase: WarehousePhase | null;
  warehouse_pending_jobs: number | null;
  warehouse_failed_jobs: number | null;
  warehouse_lag_ms: number | null;
  catalogue_generation_id: string | null;
  catalogue_entity_count: number | null;
  city_scope_count: number;
};

export type RepublicBrief = {
  schema_version: number;
  analysis_context: AnalysisContext;
  observation: BriefObservation | null;
  comparison: BriefComparisonAnchor | null;
  metrics: BriefMetric[];
  findings: BriefFinding[];
  dispatch_code: BriefFindingCode | "observation_ready" | string;
  operations: BriefOperations;
  plan: RepublicPlanBrief | null;
  unavailable_capabilities: string[];
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
  collection_stage: string | null;
  people_readings_ready: boolean;
  resource_readings_ready: boolean;
  environment_readings_ready: boolean;
  facility_contract_version: number | null;
  last_report_at_ms: number | null;
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
  content_hash: string | null;
  entry_count: number | null;
  warning_count: number;
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
  | "storage_busy"
  | "storage_contract_violation"
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
    | "market_projection"
    | "broadcast_projection"
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
  output_resource_id?: string;
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

export type ResourceCatalogueOriginFilter =
  "installed_content" | "recorded_save" | "live_game" | "player_overlay";

export type ResourceCatalogueRequest = {
  query?: string;
  origin?: ResourceCatalogueOriginFilter;
  limit?: number;
  offset?: number;
};

export type ResourceOriginEvidence = {
  installed_content: boolean;
  recorded_save: boolean;
  live_game: boolean;
  runtime_extension: boolean;
  player_overlay: boolean;
  installed_reference_count: number;
};

export type ResourceLivePrice = {
  currency: string;
  finished_price: number;
  base_price: number;
  buy_multiplier: number;
  sell_multiplier: number;
  buy_quote: number;
  sell_quote: number;
};

export type ResourceRegistryAssurance =
  "verified_observation_only" | "player_managed_modded";

export type ResourceRegistryIngestionState =
  "disabled" | "waiting_for_game" | "available" | "invalid";

export type ResourceRegistrySnapshotSummary = {
  snapshot_id: string;
  assurance: ResourceRegistryAssurance;
  game_build_id: string;
  probe_version: string;
  loader_api_version: number;
  captured_year: number;
  captured_day: number;
  captured_at_ms: number;
  resource_count: number;
};

export type ResourceRegistryStatus = {
  enabled: boolean;
  assurance: ResourceRegistryAssurance | null;
  state: ResourceRegistryIngestionState;
  latest_snapshot: ResourceRegistrySnapshotSummary | null;
  latest_probe_content_hash: string | null;
  collection_stage: string | null;
  warning_code: string | null;
};

export type ResourceCatalogueEntry = {
  resource_id: string;
  source_token: string;
  display_name: string;
  label_source: string;
  caption_id: number | null;
  live_index: number | null;
  resource_kind: number | null;
  transport_classes: number[];
  material_family: number | null;
  origin: ResourceOriginEvidence;
  live_prices: ResourceLivePrice[];
  latest_live_snapshot_id: string | null;
};

export type ResourceCatalogueRevision = {
  revision_id: string;
  definition_generation_id: string | null;
  overlay_revision: string | null;
  live_snapshot_id: string | null;
  entry_count: number;
};

export type ResourceCatalogueView = {
  revision: ResourceCatalogueRevision;
  total: number;
  limit: number;
  offset: number;
  entries: ResourceCatalogueEntry[];
};

export type ResourceDetails = {
  revision_id: string;
  entry: ResourceCatalogueEntry;
  installed_sources: string[];
  recorded_profile_count: number;
  live_snapshot: ResourceRegistrySnapshotSummary | null;
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

export type ProductionPathwayStatus =
  | "ready"
  | "ready_with_auxiliary"
  | "needs_selection"
  | "bounded"
  | "too_complex";

export type ProductionPathwaySelection = {
  resource_id: string;
  recipe_entity_id: string;
};

export type ProductionPathwayRequest = {
  root_recipe_entity_id: string;
  output_resource_id: string;
  target_quantity: number;
  max_depth: number;
  selections: ProductionPathwaySelection[];
};

export type ProductionPathwayNode = {
  id: string;
  kind: "resource" | "process";
  display_name: string;
  resource_id: string | null;
  recipe_entity_id: string | null;
  package_name: string | null;
  depth: number;
};

export type ProductionPathwayLink = {
  id: string;
  source: string;
  target: string;
  resource_id: string;
  quantity: number;
  unit: string;
  source_directive: string;
  source_line: number;
  mapping: DefinitionMappingProvenance;
};

export type ProductionPathwayCandidate = {
  recipe_entity_id: string;
  display_name: string;
  package_name: string;
  output_quantity: number;
  unit: string;
};

export type ProductionPathwayChoice = {
  resource_node_id: string;
  resource_id: string;
  display_name: string;
  required_quantity: number;
  unit: string;
  selected_recipe_entity_id: string | null;
  candidates: ProductionPathwayCandidate[];
};

export type ProductionPathwayRequirement = {
  resource_id: string;
  display_name: string;
  quantity: number;
  unit: string;
  reason:
    | "external_input"
    | "route_selection_required"
    | "depth_limit"
    | "cycle"
    | "unsupported_route"
    | "candidate_limit"
    | "node_limit"
    | "link_limit";
};

export type ProductionPathwayAuxiliaryRequirement = {
  stage_id: string;
  recipe_entity_id: string;
  resource_id: string;
  display_name: string;
  quantity: number | null;
  unit: string | null;
  reason: "different_unit" | "missing_unit";
  source_directive: string;
  source_line: number;
  mapping: DefinitionMappingProvenance;
};

export type ProductionPathwayDiagnostic = {
  code:
    | "depth_limit"
    | "cycle"
    | "unsupported_route"
    | "candidate_limit"
    | "node_limit"
    | "link_limit";
  resource_id: string | null;
  recipe_entity_id: string | null;
  depth: number;
};

export type ProductionPathwayModel = {
  schema_version: number;
  status: ProductionPathwayStatus;
  root_recipe_entity_id: string;
  output_resource_id: string;
  target_quantity: number;
  unit: string;
  max_depth: number;
  mapping_classification: string;
  nodes: ProductionPathwayNode[];
  links: ProductionPathwayLink[];
  choices: ProductionPathwayChoice[];
  terminal_requirements: ProductionPathwayRequirement[];
  auxiliary_requirements: ProductionPathwayAuxiliaryRequirement[];
  diagnostics: ProductionPathwayDiagnostic[];
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

export type MarketIndexingPhase =
  | "idle"
  | "discovering"
  | "matching"
  | "reading_archive"
  | "parsing_records"
  | "persisting"
  | "queueing_warehouse"
  | "paused"
  | "complete"
  | "failed";

export type MarketIndexingProgress = {
  job_id: string | null;
  storage_contract_version: number;
  phase: MarketIndexingPhase;
  progress_percent: number | null;
  started_at_ms: number | null;
  updated_at_ms: number | null;
  current_file: string | null;
  current_archive: number;
  total_archives: number;
  records_processed: number;
  rows_processed: number;
  completed_archives: number;
  missing_archives: number;
  changed_archives: number;
  failed_archives: number;
  duplicate_archives: number;
  cache_records_reused: number;
  cache_rows_avoided: number;
  contention_retries: number;
  contention_wait_ms: number;
  resume_count: number;
  error_code: string | null;
};

export type BroadcastIndexingProgress = MarketIndexingProgress;

export type EnvironmentIndexingProgress = MarketIndexingProgress;

export type EnvironmentActivityChannel =
  | "production"
  | "construction_use"
  | "factory_use"
  | "shop_use"
  | "vehicle_use"
  | "factory_waste"
  | "citizen_waste"
  | "demolition_waste";

export type EnvironmentActivityPoint = {
  record_id: number;
  year: number;
  day: number;
  game_day: number;
  activity_channel: EnvironmentActivityChannel;
  resource_token: string;
  primary_value: number;
  secondary_value: number;
  source_field: string;
  source_line: number;
  row_ordinal: number;
  quantity_is_publishable: boolean;
  exact_observation: ExactObservationReference | null;
};

export type EnvironmentActivitySummary = {
  activity_channel: EnvironmentActivityChannel;
  resource_count: number;
  row_count: number;
  latest_recorded_value: number | null;
  quantity_is_publishable: boolean;
};

export type EnvironmentLiveState =
  "disabled" | "waiting_for_reviewed_facility_contract" | "ready";

export type EnvironmentRecordingStatus = {
  enabled: boolean;
  interval_game_days: number;
  state: EnvironmentLiveState;
  notice_revision: number;
  latest_snapshot_id: string | null;
  latest_game_day: number | null;
  captured_facilities: number;
  detail_code: string | null;
};

export type EnvironmentTelemetryState =
  | "checked_session_not_running"
  | "checked_connection_reader_unavailable"
  | "candidate_reader_ready"
  | "reviewed_reader_ready"
  | "waiting_for_next_capture"
  | "snapshot_rejected"
  | "latest_reading_available";

export type EnvironmentValidationField =
  | "production"
  | "pollution"
  | "water_amount"
  | "water_capacity"
  | "water_quality"
  | "sewage_amount"
  | "sewage_capacity"
  | "sewage_quality";

export type EnvironmentValidationControl =
  | "positive_value"
  | "zero_value"
  | "disconnected_facility"
  | "consecutive_frame_stability"
  | "save_reload"
  | "application_restart";

export type EnvironmentValidationResult =
  "matches" | "does_not_match" | "uncertain";

export type EnvironmentValidationFacility = {
  facility_index: number;
  building_type: number;
  building_subtype: number;
  finished: boolean;
  going_away: boolean;
  position_x: number | null;
  position_z: number | null;
  production: number | null;
  pollution: number | null;
  radiation: number | null;
  water_amount: number | null;
  water_capacity: number | null;
  water_quality: number | null;
  sewage_amount: number | null;
  sewage_capacity: number | null;
  sewage_quality: number | null;
};

export type EnvironmentValidationSnapshot = {
  snapshot_id: string;
  checked_session_id: string;
  candidate_contract_version: number;
  probe_version: string;
  game_build_id: string;
  year: number;
  day: number;
  game_day: number;
  captured_at_ms: number;
  collection_fingerprint: string;
  facilities: EnvironmentValidationFacility[];
};

export type EnvironmentValidationComparisonDraft = {
  snapshot_id: string;
  facility_index: number;
  field: EnvironmentValidationField;
  wr_value: number;
  control: EnvironmentValidationControl;
  result: EnvironmentValidationResult;
  note: string | null;
};

export type EnvironmentValidationComparison =
  EnvironmentValidationComparisonDraft & {
    comparison_id: string;
    research_value: number;
    game_build_id: string;
    probe_version: string;
    created_at_ms: number;
  };

export type EnvironmentTelemetryCapability = {
  state: EnvironmentTelemetryState;
  checked_connection: boolean;
  people_readings_ready: boolean;
  resource_readings_ready: boolean;
  candidate_contract_version: number | null;
  reviewed_contract_version: number | null;
  latest_validation_snapshot: EnvironmentValidationSnapshot | null;
  detail_code: string | null;
};

export type EnvironmentSourceAvailability = {
  save_activity: boolean;
  live_pollution: boolean;
  live_radiation: boolean;
  live_water_and_sewage: boolean;
  spatial_pollution_map: boolean;
  pollution_units: string;
  radiation_units: string;
};

export type EnvironmentDefinitionContext = {
  available: boolean;
  building_count: number;
  pollution_class_facts: number;
  sewage_pollution_factors: number;
  water_quality_facts: number;
  connection_capability_facts: number;
};

export type CarbonFactorEntry = {
  resource_token: string;
  activity_channel: EnvironmentActivityChannel;
  grams_co2e_per_unit: number;
  source_name: string;
  source_year: number;
  reason: string;
  reference: string | null;
};

export type CarbonFactorSetDraft = {
  factor_set_id: string | null;
  name: string;
  accounting_boundary: string;
  reason: string;
  entries: CarbonFactorEntry[];
};

export type CarbonFactorRevision = {
  factor_set_id: string;
  name: string;
  accounting_boundary: string;
  reason: string;
  entries: CarbonFactorEntry[];
  revision: number;
  content_hash: string;
  created_at_ms: number;
  selected: boolean;
};

export type CarbonEstimateContribution = {
  resource_token: string;
  activity_channel: EnvironmentActivityChannel;
  recorded_quantity: number;
  grams_co2e_per_unit: number;
  estimated_grams_co2e: number;
};

export type CarbonEstimateModel = {
  available: boolean;
  factor_set_id: string | null;
  factor_set_revision: number | null;
  estimated_grams_co2e: number | null;
  covered_rows: number;
  eligible_rows: number;
  coverage_percent: number;
  missing_factors: string[];
  contributions: CarbonEstimateContribution[];
  limitation: string | null;
};

export type CarbonFactorImportPreview = {
  valid: boolean;
  row_count: number;
  draft: CarbonFactorSetDraft | null;
  errors: string[];
};

export type EnvironmentWorkspaceModel = {
  analysis_context: AnalysisContext;
  coverage_status: CoverageStatus | null;
  history_records: number;
  row_count: number;
  returned_rows: number;
  truncated: boolean;
  warnings: CoverageWarning[];
  source_availability: EnvironmentSourceAvailability;
  definition_context: EnvironmentDefinitionContext;
  recording: EnvironmentRecordingStatus;
  activity: EnvironmentActivityPoint[];
  summaries: EnvironmentActivitySummary[];
  resources: string[];
  factor_sets: CarbonFactorRevision[];
  carbon_estimate: CarbonEstimateModel | null;
};

export type EnvironmentCaptureResult = {
  captured: boolean;
  status: EnvironmentRecordingStatus;
};

export type EnvironmentFacilityReading = {
  facility_index: number;
  position_x: number;
  position_z: number;
  definition_identity: string | null;
  pollution_value: number | null;
  radiation_value: number | null;
  water_amount: number | null;
  water_capacity: number | null;
  water_quality: number | null;
  sewage_amount: number | null;
  sewage_capacity: number | null;
  sewage_quality: number | null;
};

export type EnvironmentSnapshot = {
  snapshot_id: string;
  session_id: string;
  game_day: number;
  facility_count: number;
  captured_at_ms: number;
  readings: EnvironmentFacilityReading[];
};

export type EnvironmentHistoryModel = {
  recording: EnvironmentRecordingStatus;
  snapshots: EnvironmentSnapshot[];
  truncated: boolean;
};

export type MarketMetricContext = {
  metric_id: string;
  formula: string;
  currency: string | null;
  unit: string;
  time_basis: string;
  exclusions: string[];
  evidence_class: string;
  profile_id: string;
  profile_version: string;
  source_fields: string[];
  analytical_head: string;
};

export type MarketCurrencyPulse = {
  currency: string;
  standard_import_value: number;
  standard_export_value: number;
  standard_trade_result: number;
  international_import_value: number;
  international_export_value: number;
  international_trade_result: number;
  positive_export_hhi: number | null;
  positive_export_resource_count: number;
  context: MarketMetricContext;
};

export type MarketTradePoint = {
  record_hash: string;
  year: number;
  day: number;
  game_day: number;
  currency: string;
  channel: string;
  import_value: number;
  export_value: number;
  trade_result: number;
  exact_observation: ExactObservationReference | null;
};

export type MarketResourceLedgerRow = {
  currency: string;
  channel: string;
  resource_token: string;
  import_quantity: number;
  export_quantity: number;
  import_account_value: number;
  export_account_value: number;
  trade_result: number;
  disposal_cost: number | null;
  source_fields: string[];
};

export type MarketPriceLedgerRow = {
  currency: string;
  resource_token: string;
  purchase_price: number | null;
  sell_price: number | null;
  base_price: number | null;
  purchase_index: number | null;
  sell_index: number | null;
  robust_log_volatility: number | null;
  volatility_observations: number;
  source_fields: string[];
};

export type MarketScalarLedgerRow = {
  fact_id: string;
  currency: string | null;
  category: number | null;
  value: number;
  source_field: string;
  source_line: number;
};

export type MarketCityRow = {
  source_id: string;
  currency: string;
  channel: string;
  import_value: number;
  export_value: number;
  trade_result: number;
};

export type MarketBasketSummary = {
  basket_id: string;
  revision: number;
  name: string;
  currency: string;
  price_side: string;
  built_in: boolean;
  selected: boolean;
  base_record_hash: string | null;
  resource_count: number;
  coverage_resources: number;
  index_value: number | null;
  reason: string;
  weights: Array<{ resource_token: string; weight: number }>;
};

export type MarketBasketDraft = {
  basket_id: string;
  name: string;
  currency: string;
  price_side: string;
  base_record_hash: string;
  reason: string;
  weights: Array<{ resource_token: string; weight: number }>;
};

export type MarketTermsOfTradeSummary = {
  currency: string;
  base_record_hash: string;
  import_basket_id: string;
  import_basket_revision: number;
  export_basket_id: string;
  export_basket_revision: number;
  import_index: number;
  export_index: number;
  terms_of_trade_index: number;
  context: MarketMetricContext;
};

export type MarketScenarioSummary = {
  scenario_id: string;
  revision: number;
  name: string;
  scenario_kind: string;
  reason: string;
  assumptions_json: string;
  selected: boolean;
  result_kind: string | null;
  result_value: number | null;
  result_unit: string | null;
  covered_components: number;
};

export type MarketScenarioDraft = {
  scenario_id: string;
  name: string;
  scenario_kind: "break_even" | "debt_stress";
  currency: "rub" | "usd";
  reason: string;
  domestic_unit_cost: number | null;
  delivery_cost: number | null;
  operating_efficiency_percent: number | null;
  exchange_rate: number | null;
  debt_service: number | null;
  export_stress_percent: number | null;
  tourism_stress_percent: number | null;
  included_income_components: string[];
};

export type MarketCoverageFacet = {
  facet_id: string;
  status: "observed" | "partial" | "not_observed";
  observed_slots: number;
  expected_slots: number;
  resource_count: number;
  currencies: string[];
  channels: string[];
  source_fields: string[];
};

export type MarketCommissioningSummary = {
  recorded_save_count: number;
  indexed_save_count: number;
  current_engine_indexed_save_count: number;
  pending_current_engine_save_count: number;
  active_engine_current: boolean;
  active_parser_engine_version: string | null;
  recommended_currency: string | null;
  recommended_channel: string | null;
  recommended_price_resource: string | null;
  facets: MarketCoverageFacet[];
};

export type MarketPriceSeriesPoint = {
  record_hash: string;
  year: number;
  day: number;
  game_day: number;
  purchase_price: number | null;
  sell_price: number | null;
  base_price: number | null;
  exact_observation: ExactObservationReference | null;
};

export type MarketPriceSeries = {
  available: boolean;
  currency: string;
  resource_token: string;
  points: MarketPriceSeriesPoint[];
  context: MarketMetricContext;
  limitation: string | null;
};

export type MarketWorkspace = {
  analysis_context: AnalysisContext;
  available: boolean;
  partial: boolean;
  coverage_status: string | null;
  history_records: number;
  row_count: number;
  city_scope_count: number;
  warehouse_history_available: boolean;
  warnings: CoverageWarning[];
  currencies: MarketCurrencyPulse[];
  trade_history: MarketTradePoint[];
  resource_ledger: MarketResourceLedgerRow[];
  price_ledger: MarketPriceLedgerRow[];
  scalar_ledger: MarketScalarLedgerRow[];
  cities: MarketCityRow[];
  baskets: MarketBasketSummary[];
  scenarios: MarketScenarioSummary[];
  metric_contexts: MarketMetricContext[];
  terms_of_trade: MarketTermsOfTradeSummary[];
  reserves_available: boolean;
  terms_of_trade_available: boolean;
  limitations: string[];
  commissioning: MarketCommissioningSummary;
};
