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
  source_file_name: string;
  source_file_size: number;
  source_modified_ms: number;
  imported_at_ms: number;
  parser_version: string;
  format_profile: string;
  branch_id: string;
  geographic_scope: string;
  coverage: CoverageReport;
  source_fields: MetricEvidence[];
  points: ReceiverHistoryPoint[];
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
  save_candidates: number;
  observed_saves: number;
  game_vocabularies: GameVocabularySource[];
};

export type ImportOutcome = "imported" | "duplicate";

export type ObservationImportResult = {
  outcome: ImportOutcome;
  dataset: ReceiverDataset;
};

export type DirectoryKind = "save" | "game";

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
  | "receiver_history_unavailable"
  | "storage_unavailable"
  | "unknown";
