import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  DirectoryKind,
  ObservationImportResult,
  ObserverErrorCode,
  ReceiverDataset,
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

export function observeLatestSave(): Promise<ObservationImportResult> {
  return invoke<ObservationImportResult>("observe_latest_save");
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
