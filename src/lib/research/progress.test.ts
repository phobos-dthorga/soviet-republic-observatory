import { describe, expect, it } from "vitest";
import { translate } from "../i18n/runtime";
import {
  researchBuildProgressView,
  researchDownloadProgressView,
  researchSessionProgressView,
} from "./progress";
import type {
  ResearchBuildProgress,
  ResearchSourceDownloadProgress,
  ResearchSessionProgress,
} from "./types";

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
    failed_stage: null,
    compiler_exit_code: null,
    remediation_code: null,
    ...values,
  };
}

function sessionProgress(
  values: Partial<ResearchSessionProgress>,
): ResearchSessionProgress {
  return {
    task_id: "research_session_preparation",
    run_id: "session-1",
    state: "running",
    phase: "preflight",
    progress_percent: 10,
    started_at_ms: 1,
    updated_at_ms: 2,
    current_item: "consent_and_paths",
    log_lines: [],
    error_code: null,
    ...values,
  };
}

function downloadProgress(
  values: Partial<ResearchSourceDownloadProgress>,
): ResearchSourceDownloadProgress {
  return {
    task_id: "research_source_download",
    run_id: "download-1",
    state: "running",
    phase: "connecting",
    progress_percent: 5,
    transferred_bytes: 0,
    expected_bytes: null,
    started_at_ms: 1,
    updated_at_ms: 2,
    current_item: "github_connection",
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
    expect(view.notice?.text).toBe(
      "The build stopped safely. No probe was installed or run.",
    );
    expect(view.notice?.technicalDetails).toEqual({
      code: "research_artifact_invalid",
      operation: "research_probe_build",
    });
  });
});

describe("research source download progress presentation", () => {
  it("shows every native phase and byte count in the foreground task panel", () => {
    const view = researchDownloadProgressView(
      downloadProgress({
        phase: "checking_archive",
        progress_percent: 76,
        transferred_bytes: 1_450_928,
        expected_bytes: 1_450_928,
        current_item: "archive_safety_checks",
      }),
      translate,
      (value) => `${value} B`,
    );

    expect(view.heading).toBe("Checking archive safety and contents");
    expect(view.stages.map((stage) => stage.state)).toEqual([
      "complete",
      "complete",
      "active",
      "pending",
      "pending",
    ]);
    expect(view.metrics.map((metric) => metric.value)).toEqual([
      "1450928 B",
      "1450928 B",
    ]);
  });

  it("keeps a failed verification visible with an expandable error code", () => {
    const view = researchDownloadProgressView(
      downloadProgress({
        state: "failed",
        phase: "failed",
        progress_percent: 96,
        error_code: "research_source_install_failed",
        current_item: "download_stopped",
      }),
      translate,
      (value) => `${value} B`,
    );

    expect(view.state).toBe("failed");
    expect(view.stages.at(-1)?.state).toBe("failed");
    expect(view.notice?.technicalDetails?.code).toBe(
      "research_source_install_failed",
    );
  });
});

describe("checked-session progress presentation", () => {
  it("shows the dedicated game-folder step without implying save writes", () => {
    const view = researchSessionProgressView(
      sessionProgress({
        phase: "installing",
        progress_percent: 70,
        current_item: "isolated_game_folder",
      }),
      translate,
    );
    expect(view.heading).toBe("Preparing the dedicated game folder");
    expect(view.currentItem).toBe("Dedicated Observatory game folder");
    expect(view.stages.map((stage) => stage.state)).toEqual([
      "complete",
      "complete",
      "active",
      "pending",
    ]);
  });

  it("keeps a preparation failure visible with its exact code", () => {
    const view = researchSessionProgressView(
      sessionProgress({
        state: "failed",
        phase: "failed",
        progress_percent: 92,
        error_code: "research_session_preparation_failed",
      }),
      translate,
    );
    expect(view.state).toBe("failed");
    expect(view.stages.at(-1)?.state).toBe("failed");
    expect(view.notice?.technicalDetails?.code).toBe(
      "research_session_preparation_failed",
    );
  });
});
