import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { ResearchBuildProgress, ResearchSetupStatus } from "./types";

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
  notice_revision: 1,
  notice_accepted: false,
  source_available: false,
  compiler_available: false,
  checkout_state: "not_selected",
  checkout_name: null,
  reviewed_tesmio_revision: "3baa141f9f08921aea9c95f0a400289cabd9960a",
  probe_built: false,
  artifact_state: "absent",
  probe_content_hash: null,
  probe_size_bytes: null,
  output_display_path: null,
  last_built_at_ms: null,
  can_build: false,
  blockers: ["desktop_required"],
  warnings: [],
  progress: browserProgress,
};

export function researchDesktopAvailable(): boolean {
  return isTauri();
}

export function getResearchSetup(): Promise<ResearchSetupStatus> {
  return isTauri()
    ? invoke<ResearchSetupStatus>("get_research_setup")
    : Promise.resolve(structuredClone(browserStatus));
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
