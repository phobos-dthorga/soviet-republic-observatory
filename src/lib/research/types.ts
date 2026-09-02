export type ResearchCheckoutState =
  "not_selected" | "missing" | "reviewed" | "unsupported";
export type ResearchArtifactState =
  "absent" | "unrecorded" | "verified" | "changed" | "missing";
export type ResearchSourceOrigin = "manual_checkout" | "observatory_downloaded";
export type ResearchSourceDownloadState =
  "idle" | "running" | "complete" | "failed";
export type ResearchSourceDownloadPhase =
  | "idle"
  | "connecting"
  | "downloading"
  | "checking_archive"
  | "installing"
  | "verifying"
  | "complete"
  | "failed";

export type ResearchSourceDownloadProgress = {
  task_id: string;
  run_id: string;
  state: ResearchSourceDownloadState;
  phase: ResearchSourceDownloadPhase;
  progress_percent: number | null;
  transferred_bytes: number;
  expected_bytes: number | null;
  started_at_ms: number | null;
  updated_at_ms: number | null;
  current_item: string | null;
  error_code: string | null;
};

export type ResearchBuildState = "idle" | "running" | "complete" | "failed";
export type ResearchBuildPhase =
  | "idle"
  | "preflight"
  | "toolchain"
  | "compiling"
  | "verifying"
  | "complete"
  | "failed";

export type ResearchBuildProgress = {
  task_id: string;
  run_id: string;
  state: ResearchBuildState;
  phase: ResearchBuildPhase;
  progress_percent: number | null;
  started_at_ms: number | null;
  updated_at_ms: number | null;
  current_item: string | null;
  log_lines: string[];
  error_code: string | null;
  failed_stage: string | null;
  compiler_exit_code: number | null;
  remediation_code: string | null;
};

export type ResearchSessionState =
  | "game_not_configured"
  | "prerequisites_required"
  | "ready_to_prepare"
  | "prepared"
  | "report_available"
  | "invalid";
export type ResearchSessionTaskState =
  "idle" | "running" | "complete" | "failed";
export type ResearchSessionPhase =
  | "idle"
  | "preflight"
  | "building_host"
  | "installing"
  | "verifying"
  | "complete"
  | "failed";

export type ResearchSessionProgress = {
  task_id: string;
  run_id: string;
  state: ResearchSessionTaskState;
  phase: ResearchSessionPhase;
  progress_percent: number | null;
  started_at_ms: number | null;
  updated_at_ms: number | null;
  current_item: string | null;
  log_lines: string[];
  error_code: string | null;
};

export type ResearchSessionStatus = {
  state: ResearchSessionState;
  game_configured: boolean;
  reviewed_loader_source_available: boolean;
  probe_ready: boolean;
  report_snapshot_count: number;
  report_collection_stage: string | null;
  managed_folder: string;
  can_prepare: boolean;
  can_launch: boolean;
  writes_game_directory: boolean;
  writes_save_data: boolean;
  changes_running_game_memory: boolean;
  progress: ResearchSessionProgress;
};

export type ResearchSetupStatus = {
  notice_revision: number;
  notice_accepted: boolean;
  source_available: boolean;
  compiler_available: boolean;
  checkout_state: ResearchCheckoutState;
  source_origin: ResearchSourceOrigin | null;
  checkout_name: string | null;
  reviewed_tesmio_revision: string;
  probe_built: boolean;
  artifact_state: ResearchArtifactState;
  probe_content_hash: string | null;
  probe_size_bytes: number | null;
  output_display_path: string | null;
  last_built_at_ms: number | null;
  can_build: boolean;
  can_download: boolean;
  blockers: string[];
  warnings: string[];
  progress: ResearchBuildProgress;
  download_progress: ResearchSourceDownloadProgress;
  session: ResearchSessionStatus;
};
