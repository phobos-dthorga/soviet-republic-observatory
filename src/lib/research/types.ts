export type ResearchCheckoutState =
  "not_selected" | "missing" | "reviewed" | "unsupported";

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
};

export type ResearchSetupStatus = {
  notice_revision: number;
  notice_accepted: boolean;
  source_available: boolean;
  compiler_available: boolean;
  checkout_state: ResearchCheckoutState;
  checkout_path: string | null;
  reviewed_tesmio_revision: string;
  probe_built: boolean;
  probe_content_hash: string | null;
  probe_size_bytes: number | null;
  output_path: string | null;
  last_built_at_ms: number | null;
  can_build: boolean;
  blockers: string[];
  warnings: string[];
  progress: ResearchBuildProgress;
};
