import { describe, expect, it } from "vitest";
import { translate } from "../i18n/runtime";
import { researchBuildProgressView } from "./progress";
import type { ResearchBuildProgress } from "./types";

function progress(
  values: Partial<ResearchBuildProgress>,
): ResearchBuildProgress {
  return {
    task_id: "research_probe_build",
    run_id: "run-1",
    state: "running",
    phase: "preflight",
    progress_percent: 10,
    started_at_ms: 1,
    updated_at_ms: 2,
    current_item: "reviewed_contract",
    log_lines: [],
    error_code: null,
    ...values,
  };
}

describe("research build progress presentation", () => {
  it("maps native phases onto the shared four-stage task view", () => {
    const view = researchBuildProgressView(
      progress({ phase: "compiling", progress_percent: 55 }),
      translate,
    );
    expect(view.progressPercent).toBe(55);
    expect(view.stages.map((stage) => stage.state)).toEqual([
      "complete",
      "complete",
      "active",
      "pending",
    ]);
    expect(view.currentItem).toBe("Reviewed source contract");
  });

  it("keeps a failed verification visible without inventing completion", () => {
    const view = researchBuildProgressView(
      progress({
        state: "failed",
        phase: "failed",
        progress_percent: 90,
        error_code: "research_artifact_invalid",
      }),
      translate,
    );
    expect(view.state).toBe("failed");
    expect(view.stages.map((stage) => stage.state)).toEqual([
      "complete",
      "complete",
      "complete",
      "failed",
    ]);
    expect(view.notice?.text).toContain("research_artifact_invalid");
  });
});
