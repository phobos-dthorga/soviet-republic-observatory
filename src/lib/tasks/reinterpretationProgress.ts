import type { Translator } from "../i18n/runtime";
import type { ReinterpretationProgress } from "../observations/types";
import type {
  TaskProgressStage,
  TaskProgressView,
  TaskStageState,
} from "./progress";

const phaseOrder: Record<ReinterpretationProgress["phase"], number> = {
  idle: -1,
  reading: 0,
  parsing: 1,
  persisting: 2,
  queueing_warehouse: 3,
  complete: 4,
  failed: -1,
};

const stageKeys = [
  "reinterpretation-stage-read",
  "reinterpretation-stage-map",
  "reinterpretation-stage-store",
  "reinterpretation-stage-project",
] as const;

function stageState(
  progress: ReinterpretationProgress,
  index: number,
): TaskStageState {
  if (progress.phase === "failed") return index === 0 ? "failed" : "pending";
  const current = phaseOrder[progress.phase];
  if (current > index || progress.phase === "complete") return "complete";
  if (current === index) return "active";
  return "pending";
}

function stateLabel(state: TaskStageState, translate: Translator): string {
  return translate(
    state === "active"
      ? "task-progress-stage-active"
      : state === "complete"
        ? "task-progress-stage-complete"
        : state === "failed"
          ? "task-progress-stage-failed"
          : "task-progress-stage-pending",
  );
}

function heading(
  progress: ReinterpretationProgress,
  translate: Translator,
): string {
  const keys = {
    idle: "reinterpretation-progress-idle",
    reading: "reinterpretation-progress-reading",
    parsing: "reinterpretation-progress-parsing",
    persisting: "reinterpretation-progress-persisting",
    queueing_warehouse: "reinterpretation-progress-queueing",
    complete: "reinterpretation-progress-complete",
    failed: "reinterpretation-progress-failed",
  } as const;
  return translate(keys[progress.phase]);
}

export function reinterpretationProgressView(
  progress: ReinterpretationProgress,
  translate: Translator,
): TaskProgressView {
  const stages: TaskProgressStage[] = stageKeys.map((key, index) => {
    const state = stageState(progress, index);
    return {
      id: String(index),
      label: translate(key),
      state,
      stateLabel: stateLabel(state, translate),
    };
  });
  return {
    taskId: "compatibility.reinterpret",
    runId: `compatibility.reinterpret:${progress.started_at_ms ?? "idle"}`,
    state:
      progress.phase === "failed"
        ? "failed"
        : progress.phase === "complete"
          ? "complete"
          : "running",
    eyebrow: translate("compatibility-eyebrow"),
    heading: heading(progress, translate),
    progressPercent: progress.progress_percent,
    stages,
    metrics: [],
    meters: [],
    currentItemLabel: progress.current_file
      ? translate("reinterpretation-current-file")
      : null,
    currentItem: progress.current_file,
    currentItemContext: progress.interpretation_id?.slice(0, 12) ?? null,
    notice:
      progress.phase === "failed"
        ? {
            tone: "error",
            text: translate("reinterpretation-failed-note"),
          }
        : null,
  };
}
