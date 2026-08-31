import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ThemeManifest, ThemeValidationReport } from "../theme/types";

export type UiReviewContext = {
  enabled: boolean;
  run_id: string | null;
  data_state: "fixture" | "live" | null;
  background_work_suppressed: boolean;
  validator_boundary_theme: ThemeManifest | null;
  validator_boundary_report: ThemeValidationReport | null;
};

export function getUiReviewContext(): Promise<UiReviewContext> {
  if (!isTauri()) {
    return Promise.resolve({
      enabled: false,
      run_id: null,
      data_state: null,
      background_work_suppressed: false,
      validator_boundary_theme: null,
      validator_boundary_report: null,
    });
  }
  return invoke<UiReviewContext>("get_ui_review_context");
}
