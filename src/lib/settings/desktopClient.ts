import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  ApplicationPreferencesDraft,
  ApplicationSettingsView,
} from "./types";

export function applicationSettingsHostAvailable(): boolean {
  return isTauri();
}

export function getApplicationSettings(): Promise<ApplicationSettingsView> {
  return invoke<ApplicationSettingsView>("get_application_settings");
}

export function updateApplicationPreferences(
  preferences: ApplicationPreferencesDraft,
): Promise<ApplicationSettingsView> {
  return invoke<ApplicationSettingsView>("update_application_preferences", {
    preferences,
  });
}

export function resetApplicationPreferences(): Promise<ApplicationSettingsView> {
  return invoke<ApplicationSettingsView>("reset_application_preferences");
}

export function replayAllAttentionCues(): Promise<number> {
  return invoke<number>("replay_all_attention_cues");
}
