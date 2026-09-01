import type { Translator } from "../i18n/runtime";
import type { TranslationKey } from "../i18n/catalog";
import type {
  TaskProgressStage,
  TaskProgressView,
  TaskStageState,
} from "../tasks/progress";
import type { ResearchBuildPhase, ResearchBuildProgress } from "./types";

const phaseIndex: Record<ResearchBuildPhase, number> = {
  idle: -1,
  preflight: 0,
  toolchain: 1,
  compiling: 2,
  verifying: 3,
  complete: 4,
  failed: -1,
};

const phaseMessage: Record<ResearchBuildPhase, TranslationKey> = {
  idle: "research-setup-progress-idle",
  preflight: "research-setup-progress-preflight",
  toolchain: "research-setup-progress-toolchain",
  compiling: "research-setup-progress-compiling",
  verifying: "research-setup-progress-verifying",
  complete: "research-setup-progress-complete",
  failed: "research-setup-progress-failed",
};

const itemMessage: Record<string, TranslationKey> = {
  reviewed_contract: "research-setup-item-reviewed-contract",
  visual_cpp_toolchain: "research-setup-item-visual-cpp-toolchain",
  observatory_probe_cpp: "research-setup-item-observatory-probe-cpp",
  probe_artifact: "research-setup-item-probe-artifact",
  build_complete: "research-setup-item-build-complete",
};

const stageStateMessage: Record<TaskStageState, TranslationKey> = {
  pending: "task-progress-stage-pending",
  active: "task-progress-stage-active",
  complete: "task-progress-stage-complete",
  failed: "task-progress-stage-failed",
};

export function researchBuildProgressView(
  progress: ResearchBuildProgress,
  translate: Translator,
): TaskProgressView {
  return {
    taskId: "research.probe.build",
    runId: progress.run_id,
    state:
      progress.state === "idle"
        ? "complete"
        : progress.state === "failed"
          ? "failed"
          : progress.state,
    eyebrow: translate("research-setup-progress-eyebrow"),
    heading: translate(phaseMessage[progress.phase]),
    progressPercent: progress.progress_percent,
    stages: stages(progress, translate),
    metrics: [],
    meters: [],
    currentItemLabel: progress.current_item
      ? translate("research-setup-progress-current")
      : null,
    currentItem: progress.current_item
      ? translate(
          itemMessage[progress.current_item] ??
            "research-setup-progress-current",
        )
      : null,
    currentItemContext: null,
    notice:
      progress.state === "failed"
        ? {
            tone: "error",
            text: translate("research-setup-progress-error-summary"),
            technicalDetails: {
              code: progress.error_code ?? "unknown",
              operation: "research_probe_build",
            },
          }
        : null,
  };
}

function stages(
  progress: ResearchBuildProgress,
  translate: Translator,
): TaskProgressStage[] {
  const active = failedStage(progress);
  const definitions: Array<[string, TranslationKey]> = [
    ["preflight", "research-setup-stage-preflight"],
    ["toolchain", "research-setup-stage-toolchain"],
    ["compile", "research-setup-stage-compile"],
    ["verify", "research-setup-stage-verify"],
  ];
  return definitions.map(([id, key], index) => {
    const state = stageState(progress, index, active);
    return {
      id,
      label: translate(key),
      state,
      stateLabel: translate(stageStateMessage[state]),
    };
  });
}

function failedStage(progress: ResearchBuildProgress): number {
  if (progress.phase !== "failed") return phaseIndex[progress.phase];
  const percent = progress.progress_percent ?? 0;
  if (percent >= 90) return 3;
  if (percent >= 55) return 2;
  if (percent >= 30) return 1;
  return 0;
}

function stageState(
  progress: ResearchBuildProgress,
  index: number,
  active: number,
): TaskStageState {
  if (progress.phase === "complete") return "complete";
  if (progress.phase === "idle") return "pending";
  if (progress.phase === "failed" && index === active) return "failed";
  if (index < active) return "complete";
  if (index === active) return "active";
  return "pending";
}
