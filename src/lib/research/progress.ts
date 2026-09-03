import type { Translator } from "../i18n/runtime";
import type { TranslationKey } from "../i18n/catalog";
import type {
  TaskProgressMetric,
  TaskProgressStage,
  TaskProgressView,
  TaskStageState,
} from "../tasks/progress";
import type {
  ResearchBuildPhase,
  ResearchBuildProgress,
  ResearchSessionPhase,
  ResearchSessionProgress,
  ResearchSourceDownloadPhase,
  ResearchSourceDownloadProgress,
} from "./types";

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

const downloadPhaseIndex: Record<ResearchSourceDownloadPhase, number> = {
  idle: -1,
  connecting: 0,
  downloading: 1,
  checking_archive: 2,
  installing: 3,
  verifying: 4,
  complete: 5,
  failed: -1,
};

const downloadPhaseMessage: Record<
  ResearchSourceDownloadPhase,
  TranslationKey
> = {
  idle: "research-download-progress-idle",
  connecting: "research-download-progress-connecting",
  downloading: "research-download-progress-downloading",
  checking_archive: "research-download-progress-checking",
  installing: "research-download-progress-installing",
  verifying: "research-download-progress-verifying",
  complete: "research-download-progress-complete",
  failed: "research-download-progress-failed",
};

const downloadItems: Record<string, TranslationKey> = {
  github_connection: "research-download-item-connection",
  reviewed_source_archive: "research-download-item-archive",
  archive_safety_checks: "research-download-item-safety",
  reviewed_source_files: "research-download-item-files",
  reviewed_header_identity: "research-download-item-headers",
  download_complete: "research-download-item-complete",
  download_stopped: "research-download-item-stopped",
};

const sessionPhaseIndex: Record<ResearchSessionPhase, number> = {
  idle: -1,
  preflight: 0,
  building_host: 1,
  installing: 2,
  verifying: 3,
  checking_setup: 0,
  starting_game: 1,
  loading_tesmio: 2,
  game_resumed: 3,
  waiting_for_report: 4,
  complete: 5,
  failed: -1,
};

const sessionPhaseMessage: Record<ResearchSessionPhase, TranslationKey> = {
  idle: "research-session-progress-idle",
  preflight: "research-session-progress-preflight",
  building_host: "research-session-progress-building",
  installing: "research-session-progress-installing",
  verifying: "research-session-progress-verifying",
  checking_setup: "research-session-progress-checking-setup",
  starting_game: "research-session-progress-starting-game",
  loading_tesmio: "research-session-progress-loading-tesmio",
  game_resumed: "research-session-progress-game-resumed",
  waiting_for_report: "research-session-progress-waiting-report",
  complete: "research-session-progress-complete",
  failed: "research-session-progress-failed",
};

const sessionItems: Record<string, TranslationKey> = {
  consent_and_paths: "research-session-item-consent",
  reviewed_tesmio_host: "research-session-item-host",
  isolated_game_folder: "research-session-item-folder",
  read_only_contract: "research-session-item-contract",
  existing_checked_setup: "research-session-item-existing",
  ready_for_confirmed_launch: "research-session-item-ready",
  checking_checked_setup: "research-session-item-checking-setup",
  starting_wr: "research-session-item-starting-game",
  loading_tesmio: "research-session-item-loading-tesmio",
  game_resumed: "research-session-item-game-resumed",
  waiting_for_checked_report: "research-session-item-waiting-report",
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

export function researchDownloadProgressView(
  progress: ResearchSourceDownloadProgress,
  translate: Translator,
  formatBytes: (value: number) => string,
): TaskProgressView {
  const metrics: TaskProgressMetric[] = [
    {
      id: "received",
      label: translate("research-download-metric-received"),
      value: formatBytes(progress.transferred_bytes),
    },
  ];
  if (progress.expected_bytes != null) {
    metrics.push({
      id: "expected",
      label: translate("research-download-metric-expected"),
      value: formatBytes(progress.expected_bytes),
    });
  }
  return {
    taskId: progress.task_id,
    runId: progress.run_id,
    state:
      progress.state === "idle"
        ? "complete"
        : progress.state === "failed"
          ? "failed"
          : progress.state,
    eyebrow: translate("research-download-progress-eyebrow"),
    heading: translate(downloadPhaseMessage[progress.phase]),
    progressPercent: progress.progress_percent,
    stages: downloadStages(progress, translate),
    metrics,
    meters: [],
    currentItemLabel: progress.current_item
      ? translate("research-download-current-item")
      : null,
    currentItem: progress.current_item
      ? translate(
          downloadItems[progress.current_item] ??
            "research-download-item-archive",
        )
      : null,
    currentItemContext: null,
    notice:
      progress.state === "failed"
        ? {
            tone: "error",
            text: translate("research-setup-download-failure"),
            technicalDetails: {
              code: progress.error_code ?? "unknown",
              operation: "research_source_download",
            },
          }
        : null,
  };
}

export function researchSessionProgressView(
  progress: ResearchSessionProgress,
  translate: Translator,
): TaskProgressView {
  return {
    taskId: progress.task_id,
    runId: progress.run_id,
    state:
      progress.state === "idle"
        ? "complete"
        : progress.state === "failed"
          ? "failed"
          : progress.state,
    eyebrow: translate("research-session-progress-eyebrow"),
    heading: translate(sessionPhaseMessage[progress.phase]),
    progressPercent: progress.progress_percent,
    stages: sessionStages(progress, translate),
    metrics: [],
    meters: [],
    currentItemLabel: progress.current_item
      ? translate("research-session-progress-current")
      : null,
    currentItem: progress.current_item
      ? translate(
          sessionItems[progress.current_item] ??
            "research-session-progress-current",
        )
      : null,
    currentItemContext: null,
    notice:
      progress.state === "failed"
        ? {
            tone: "error",
            text: translate("research-session-error-summary"),
            technicalDetails: {
              code: progress.error_code ?? "unknown",
              operation: "prepare_observation_only_session",
            },
          }
        : null,
  };
}

function sessionStages(
  progress: ResearchSessionProgress,
  translate: Translator,
): TaskProgressStage[] {
  const definitions: Array<[string, TranslationKey]> =
    progress.task_id === "research_session_launch"
      ? [
          ["check", "research-session-stage-check-setup"],
          ["start", "research-session-stage-start-game"],
          ["load", "research-session-stage-load-tesmio"],
          ["resume", "research-session-stage-game-resumed"],
          ["report", "research-session-stage-wait-report"],
        ]
      : [
          ["preflight", "research-session-stage-preflight"],
          ["build", "research-session-stage-build"],
          ["install", "research-session-stage-install"],
          ["verify", "research-session-stage-verify"],
        ];
  const active =
    progress.phase === "failed"
      ? failedSessionStage(progress.progress_percent, definitions.length)
      : sessionPhaseIndex[progress.phase];
  return definitions.map(([id, key], index) => {
    const state = sessionStageState(progress, index, active);
    return {
      id,
      label: translate(key),
      state,
      stateLabel: translate(stageStateMessage[state]),
    };
  });
}

function failedSessionStage(
  percent: number | null,
  stageCount: number,
): number {
  if (stageCount === 5) {
    if ((percent ?? 0) >= 80) return 3;
    if ((percent ?? 0) >= 50) return 2;
    if ((percent ?? 0) >= 25) return 1;
    return 0;
  }
  if ((percent ?? 0) >= 90) return 3;
  if ((percent ?? 0) >= 65) return 2;
  if ((percent ?? 0) >= 25) return 1;
  return 0;
}

function sessionStageState(
  progress: ResearchSessionProgress,
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

function downloadStages(
  progress: ResearchSourceDownloadProgress,
  translate: Translator,
): TaskProgressStage[] {
  const definitions: Array<[string, TranslationKey]> = [
    ["connect", "research-download-stage-connect"],
    ["receive", "research-download-stage-receive"],
    ["check", "research-download-stage-check"],
    ["store", "research-download-stage-store"],
    ["verify", "research-download-stage-verify"],
  ];
  const active =
    progress.phase === "failed"
      ? failedDownloadStage(progress.progress_percent)
      : downloadPhaseIndex[progress.phase];
  return definitions.map(([id, key], index) => {
    const state = downloadStageState(progress, index, active);
    return {
      id,
      label: translate(key),
      state,
      stateLabel: translate(stageStateMessage[state]),
    };
  });
}

function failedDownloadStage(percent: number | null): number {
  if ((percent ?? 0) >= 95) return 4;
  if ((percent ?? 0) >= 85) return 3;
  if ((percent ?? 0) >= 74) return 2;
  if ((percent ?? 0) >= 10) return 1;
  return 0;
}

function downloadStageState(
  progress: ResearchSourceDownloadProgress,
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
