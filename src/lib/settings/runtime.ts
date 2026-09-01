import type { ApplicationPreferences } from "./types";
import { applyWordingMode } from "../i18n/runtime";

export function applyApplicationPreferences(
  preferences: ApplicationPreferences,
): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.style.fontSize = `${preferences.text_scale_percent}%`;
  root.dataset.motionPreference = preferences.motion_preference;
  applyWordingMode(preferences.wording_mode);
}
