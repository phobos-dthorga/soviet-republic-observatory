import type { ApplicationPreferences } from "./types";

export function applyApplicationPreferences(
  preferences: ApplicationPreferences,
): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.style.fontSize = `${preferences.text_scale_percent}%`;
  root.dataset.motionPreference = preferences.motion_preference;
}
