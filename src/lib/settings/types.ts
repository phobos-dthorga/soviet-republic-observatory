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
};
