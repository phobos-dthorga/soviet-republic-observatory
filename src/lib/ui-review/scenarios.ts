export const UI_REVIEW_SCENARIOS = [
  "workspace-briefing",
  "workspace-monitor",
  "workspace-broadcast",
  "workspace-extensions",
  "workspace-materials",
  "materials-warehouse-attention",
  "production-pathway",
  "workspace-population",
  "population-probe-missing",
  "archive-latest",
  "archive-historical",
  "critical-task-loading",
  "critical-task-failed",
  "dialog-language",
  "dialog-theme",
  "dialog-observation",
  "dialog-diagnostics",
  "dialog-legal",
  "dialog-research",
  "notification-error",
  "tooltip-contextual",
  "attention-cue",
  "keyboard-focus",
  "native-dropdown",
] as const;

export type UiReviewScenarioId = (typeof UI_REVIEW_SCENARIOS)[number];
export type UiReviewThemeId = "classic" | "high-contrast" | "boundary";
