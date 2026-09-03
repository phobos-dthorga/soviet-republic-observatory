import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { TesmioProbeStatus } from "../observations/types";
import type {
  ResearchBuildProgress,
  ResearchSessionProgress,
  ResearchSetupStatus,
  ResearchSourceDownloadProgress,
} from "./types";

const browserProgress: ResearchBuildProgress = {
  task_id: "research_probe_build",
  run_id: "not_started",
  state: "idle",
  phase: "idle",
  progress_percent: null,
  started_at_ms: null,
  updated_at_ms: null,
  current_item: null,
  log_lines: [],
  error_code: null,
  failed_stage: null,
  compiler_exit_code: null,
  remediation_code: null,
};

const browserStatus: ResearchSetupStatus = {
  notice_revision: 5,
  notice_accepted: false,
  source_available: false,
  compiler_available: false,
  checkout_state: "not_selected",
  source_origin: null,
  checkout_name: null,
  reviewed_tesmio_revision: "3baa141f9f08921aea9c95f0a400289cabd9960a",
  probe_built: false,
  artifact_state: "absent",
  probe_content_hash: null,
  probe_size_bytes: null,
  output_display_path: null,
  last_built_at_ms: null,
  can_build: false,
  can_download: false,
  blockers: ["desktop_required"],
  warnings: [],
  progress: browserProgress,
  download_progress: {
    task_id: "research_source_download",
    run_id: "not_started",
    state: "idle",
    phase: "idle",
    progress_percent: null,
    transferred_bytes: 0,
    expected_bytes: null,
    started_at_ms: null,
    updated_at_ms: null,
    current_item: null,
    error_code: null,
  },
  session: {
    state: "game_not_configured",
    launch_state: "idle",
    game_configured: false,
    reviewed_loader_source_available: false,
    probe_ready: false,
    report_snapshot_count: 0,
    report_collection_stage: null,
    people_readings_ready: false,
    resource_readings_ready: false,
    environment_readings_ready: false,
    facility_contract_version: null,
    last_report_at_ms: null,
    managed_folder: "W&R/tesmioloader/observatory",
    can_prepare: false,
    can_launch: false,
    writes_game_directory: true,
    writes_save_data: false,
    changes_running_game_memory: true,
    progress: {
      task_id: "research_session_preparation",
      run_id: "not_started",
      state: "idle",
      phase: "idle",
      progress_percent: null,
      started_at_ms: null,
      updated_at_ms: null,
      current_item: null,
      log_lines: [],
      error_code: null,
    },
  },
};

export function researchDesktopAvailable(): boolean {
  return isTauri();
}

export function getResearchSetup(): Promise<ResearchSetupStatus> {
  return isTauri()
    ? invoke<ResearchSetupStatus>("get_research_setup")
    : Promise.resolve(structuredClone(browserStatus));
}

export function getResearchReportStatus(): Promise<TesmioProbeStatus> {
  return isTauri()
    ? invoke<TesmioProbeStatus>("get_research_report_status")
    : Promise.resolve({
        state: "missing",
        read_only: true,
        optional: true,
        persisted: false,
        probe_id: null,
        probe_version: null,
        loader_api_version: null,
        target_game_version: null,
        executable_timestamp: null,
        content_hash: null,
        snapshot_count: 0,
        sample_count: 0,
        latest_year: null,
        latest_day: null,
        latest_population_count: null,
        collection_stage: null,
        people_readings_ready: false,
        resource_readings_ready: false,
        environment_readings_ready: false,
        facility_contract_version: null,
        last_report_at_ms: null,
        warnings: [],
      });
}

export function setResearchNoticeAccepted(
  accepted: boolean,
): Promise<ResearchSetupStatus> {
  return invoke<ResearchSetupStatus>("set_research_notice_accepted", {
    accepted,
  });
}

export async function chooseResearchCheckout(
  title: string,
): Promise<string | null> {
  if (!isTauri()) return null;
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}

export function configureResearchCheckout(
  path: string,
): Promise<ResearchSetupStatus> {
  return invoke<ResearchSetupStatus>("configure_research_tesmio_checkout", {
    path,
  });
}

export function getResearchBuildProgress(): Promise<ResearchBuildProgress> {
  return isTauri()
    ? invoke<ResearchBuildProgress>("get_research_build_progress")
    : Promise.resolve(structuredClone(browserProgress));
}

export function listenForResearchBuildProgress(
  accept: (progress: ResearchBuildProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen<ResearchBuildProgress>("research-setup-progress", (event) =>
    accept(event.payload),
  );
}

export function buildResearchProbe(): Promise<ResearchSetupStatus> {
  return invoke<ResearchSetupStatus>("build_research_probe");
}

export function downloadReviewedTesmioSource(): Promise<ResearchSetupStatus> {
  return invoke<ResearchSetupStatus>("download_reviewed_tesmio_source");
}

export function getResearchSourceDownloadProgress(): Promise<ResearchSourceDownloadProgress> {
  return isTauri()
    ? invoke<ResearchSourceDownloadProgress>(
        "get_research_source_download_progress",
      )
    : Promise.resolve(structuredClone(browserStatus.download_progress));
}

export function listenForResearchSourceDownloadProgress(
  accept: (progress: ResearchSourceDownloadProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen<ResearchSourceDownloadProgress>(
    "research-source-download-progress",
    (event) => accept(event.payload),
  );
}

export function getResearchSessionProgress(): Promise<ResearchSessionProgress> {
  return isTauri()
    ? invoke<ResearchSessionProgress>("get_research_session_progress")
    : Promise.resolve(structuredClone(browserStatus.session.progress));
}

export function listenForResearchSessionProgress(
  accept: (progress: ResearchSessionProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen<ResearchSessionProgress>("research-session-progress", (event) =>
    accept(event.payload),
  );
}

export function prepareObservationOnlySession(): Promise<ResearchSetupStatus> {
  return invoke<ResearchSetupStatus>("prepare_observation_only_session", {
    gameDirectoryWriteConfirmed: true,
  });
}

export function launchObservationOnlySession(): Promise<ResearchSetupStatus> {
  return invoke<ResearchSetupStatus>("launch_observation_only_session", {
    runningGameMemoryConfirmed: true,
  });
}
