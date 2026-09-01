import type { Translator } from "../i18n/runtime";
import type { CatalogueRefreshProgress } from "../observations/types";
import type {
  TaskProgressStage,
  TaskProgressView,
  TaskStageState,
} from "./progress";

const phaseOrder: Record<CatalogueRefreshProgress["phase"], number> = {
  idle: -1,
  discovering: 0,
  scanning: 1,
  publishing: 2,
  finalising: 3,
  complete: 4,
  failed: -1,
};

export function catalogueProgressHeading(
  progress: CatalogueRefreshProgress,
  translate: Translator,
): string {
  switch (progress.phase) {
    case "discovering":
      return translate("catalogue-progress-discovering");
    case "scanning":
      return translate("catalogue-progress-scanning");
    case "publishing":
      return translate("catalogue-progress-publishing");
    case "finalising":
      return translate("catalogue-progress-finalising");
    case "complete":
      return translate("catalogue-progress-complete");
    case "failed":
      return translate("catalogue-progress-failed");
    default:
      return translate("catalogue-progress-idle");
  }
}

export function catalogueProgressView(
  progress: CatalogueRefreshProgress,
  translate: Translator,
  clockMs: number,
): TaskProgressView {
  const active = [
    "discovering",
    "scanning",
    "publishing",
    "finalising",
  ].includes(progress.phase);
  const stalled =
    active &&
    progress.updated_at_ms != null &&
    clockMs - progress.updated_at_ms > 15_000;
  const currentFileContext =
    progress.current_file_index != null && progress.files_discovered > 0
      ? translate("catalogue-progress-file-position", {
          current: progress.current_file_index,
          total: progress.files_discovered,
        })
      : null;

  return {
    taskId: "catalogue.refresh",
    runId: `catalogue.refresh:${progress.started_at_ms ?? "idle"}`,
    state:
      progress.phase === "failed"
        ? "failed"
        : progress.phase === "complete"
          ? "complete"
          : "running",
    eyebrow: triggerLabel(progress, translate),
    heading: catalogueProgressHeading(progress, translate),
    progressPercent: progress.progress_percent,
    stages: catalogueStages(progress, translate),
    meters: [
      ...(progress.files_discovered > 0 && progress.phase !== "discovering"
        ? [
            {
              id: "files",
              label: translate("catalogue-progress-file-meter"),
              completed: progress.files_processed,
              total: progress.files_discovered,
            },
          ]
        : []),
      ...(progress.rows_total > 0
        ? [
            {
              id: "warehouse-rows",
              label: translate("catalogue-progress-row-meter"),
              completed: progress.rows_written,
              total: progress.rows_total,
            },
          ]
        : []),
    ],
    metrics: [
      {
        id: "elapsed",
        label: translate("catalogue-progress-elapsed"),
        value: elapsedLabel(progress, translate, clockMs),
      },
      {
        id: "sources",
        label: translate("catalogue-progress-sources"),
        value: `${progress.sources_discovered} / ${progress.sources_total}`,
      },
      {
        id: "files",
        label: translate("catalogue-progress-files"),
        value: `${progress.files_processed} / ${progress.files_discovered}`,
      },
      {
        id: "reused",
        label: translate("catalogue-progress-reused"),
        value: String(progress.files_reused),
      },
      {
        id: "parsed",
        label: translate("catalogue-progress-parsed"),
        value: String(progress.files_parsed),
      },
      {
        id: "entities",
        label: translate("catalogue-progress-entities"),
        value: String(progress.entities_prepared),
      },
      {
        id: "rows",
        label: translate("catalogue-progress-warehouse-rows"),
        value: `${progress.rows_written} / ${progress.rows_total}`,
      },
    ],
    currentItemLabel: progress.current_file
      ? translate("catalogue-progress-current-file")
      : progress.current_source
        ? translate("catalogue-progress-current-source")
        : null,
    currentItem: progress.current_file ?? progress.current_source,
    currentItemContext: progress.current_file
      ? (currentFileContext ?? progress.current_source)
      : null,
    notice: stalled
      ? {
          tone: "warning",
          text: translate("catalogue-progress-stalled"),
        }
      : progress.phase === "failed"
        ? {
            tone: "error",
            text: translate("catalogue-progress-error-summary"),
            technicalDetails: {
              code: progress.error_code ?? "unknown",
              operation: "catalogue_refresh",
            },
          }
        : null,
  };
}

function triggerLabel(
  progress: CatalogueRefreshProgress,
  translate: Translator,
): string {
  if (progress.trigger === "manual")
    return translate("catalogue-trigger-manual");
  if (progress.trigger === "filesystem")
    return translate("catalogue-trigger-filesystem");
  return translate("catalogue-trigger-startup");
}

function elapsedLabel(
  progress: CatalogueRefreshProgress,
  translate: Translator,
  clockMs: number,
): string {
  if (progress.started_at_ms == null) return "—";
  const endedAt =
    progress.phase === "complete" || progress.phase === "failed"
      ? (progress.updated_at_ms ?? clockMs)
      : clockMs;
  const elapsedSeconds = Math.max(
    0,
    Math.round((endedAt - progress.started_at_ms) / 1_000),
  );
  if (elapsedSeconds < 60)
    return translate("catalogue-duration-seconds", { count: elapsedSeconds });
  return translate("catalogue-duration-minutes", {
    count: Math.floor(elapsedSeconds / 60),
    seconds: elapsedSeconds % 60,
  });
}

function catalogueStages(
  progress: CatalogueRefreshProgress,
  translate: Translator,
): TaskProgressStage[] {
  const activeIndex = failedStageIndex(progress);
  const stages = [
    ["discovery", translate("catalogue-progress-stage-discovery")],
    ["scan", translate("catalogue-progress-stage-scan")],
    ["publish", translate("catalogue-progress-stage-publish")],
    ["finalise", translate("catalogue-progress-stage-finalise")],
  ] as const;
  return stages.map(([id, label], index) => {
    const state = stageState(progress, index, activeIndex);
    return {
      id,
      label,
      state,
      stateLabel: stageStateLabel(state, translate),
    };
  });
}

function failedStageIndex(progress: CatalogueRefreshProgress): number {
  if (progress.phase !== "failed") return phaseOrder[progress.phase];
  if (progress.rows_total > 0 || progress.rows_written > 0) return 2;
  if (progress.files_discovered > 0 || progress.files_processed > 0) return 1;
  return 0;
}

function stageState(
  progress: CatalogueRefreshProgress,
  index: number,
  activeIndex: number,
): TaskStageState {
  if (progress.phase === "complete") return "complete";
  if (progress.phase === "failed" && index === activeIndex) return "failed";
  if (index < activeIndex) return "complete";
  if (index === activeIndex) return "active";
  return "pending";
}

function stageStateLabel(state: TaskStageState, translate: Translator): string {
  if (state === "active") return translate("task-progress-stage-active");
  if (state === "complete") return translate("task-progress-stage-complete");
  if (state === "failed") return translate("task-progress-stage-failed");
  return translate("task-progress-stage-pending");
}
