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
    const browserReview =
      typeof window !== "undefined" &&
      ["127.0.0.1", "localhost"].includes(window.location.hostname) &&
      new URLSearchParams(window.location.search).get("ui-review") ===
        "fixture";
    return Promise.resolve({
      enabled: browserReview,
      run_id: browserReview ? "browser-fixture" : null,
      data_state: browserReview ? "fixture" : null,
      background_work_suppressed: browserReview,
      validator_boundary_theme: null,
      validator_boundary_report: null,
    });
  }
  return invoke<UiReviewContext>("get_ui_review_context");
}
