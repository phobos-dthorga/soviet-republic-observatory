import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ArchiveOverview,
  ArchiveComparison,
  BranchSelectionResult,
  DirectoryKind,
  ObservationImportResult,
  ObserverErrorCode,
  ReceiverDataset,
  RecorderHealth,
  RecorderUpdate,
  SetupState,
} from "./types";

export function desktopHostAvailable(): boolean {
  return isTauri();
}

export async function chooseDirectory(title: string): Promise<string | null> {
  if (!desktopHostAvailable()) return null;
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}

export function configureDirectory(
  kind: DirectoryKind,
  path: string,
): Promise<SetupState> {
  return invoke<SetupState>("configure_directory", { kind, path });
}

export function getSetupState(): Promise<SetupState> {
  return invoke<SetupState>("get_setup_state");
}

export function getLatestReceiverDataset(): Promise<ReceiverDataset | null> {
  return invoke<ReceiverDataset | null>("get_latest_receiver_dataset");
}

export function getArchiveOverview(): Promise<ArchiveOverview> {
  return invoke<ArchiveOverview>("get_archive_overview");
}

export function getRecorderHealth(): Promise<RecorderHealth> {
  return invoke<RecorderHealth>("get_recorder_health");
}

export function listenForRecorderUpdates(
  accept: (update: RecorderUpdate) => void,
): Promise<UnlistenFn> {
  return listen<RecorderUpdate>("recorder-update", (event) =>
    accept(event.payload),
  );
}

export function selectTimelineBranch(
  branchId: string,
): Promise<BranchSelectionResult> {
  return invoke<BranchSelectionResult>("select_timeline_branch", {
    branchId,
  });
}

export function observeLatestSave(): Promise<ObservationImportResult> {
  return invoke<ObservationImportResult>("observe_latest_save");
}

export function setAutomaticObservation(enabled: boolean): Promise<SetupState> {
  return invoke<SetupState>("set_automatic_observation", { enabled });
}

export function compareArchiveObservations(
  fromPayloadHash: string,
  toPayloadHash: string,
): Promise<ArchiveComparison> {
  return invoke<ArchiveComparison>("compare_archive_observations", {
    fromPayloadHash,
    toPayloadHash,
  });
}

export function observerErrorCode(error: unknown): ObserverErrorCode {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string"
  ) {
    return error.code as ObserverErrorCode;
  }
  return "unknown";
}
