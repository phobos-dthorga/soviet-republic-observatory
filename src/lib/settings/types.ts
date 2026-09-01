import type { SetupState } from "../observations/types";

export type StoragePatiencePreset = "short" | "balanced" | "patient" | "custom";

export type BackgroundWorkPriority = "gentle" | "balanced" | "finish_sooner";

export type MotionPreference = "system" | "reduced";

export type ApplicationPreferencesDraft = {
  storage_patience_preset: StoragePatiencePreset;
  custom_storage_patience_seconds: number | null;
  background_work_priority: BackgroundWorkPriority;
  text_scale_percent: 100 | 125 | 150 | 175 | 200;
  motion_preference: MotionPreference;
  automatic_observation_enabled: boolean;
};

export type ApplicationPreferences = ApplicationPreferencesDraft & {
  schema_version: number;
  effective_storage_patience_seconds: number;
};

export type ApplicationSettingsView = {
  preferences: ApplicationPreferences;
  setup: SetupState;
  maintenance: MaintenanceDiagnostics;
};

export type MaintenanceDiagnostics = {
  market_storage_contract_version: number;
  cached_market_records: number;
  cached_market_fact_rows: number;
  market_interpretation_memberships: number;
  latest_indexing_phase:
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
  latest_cache_records_reused: number;
  latest_cache_rows_avoided: number;
  latest_contention_retries: number;
  latest_contention_wait_ms: number;
  latest_resume_count: number;
};
