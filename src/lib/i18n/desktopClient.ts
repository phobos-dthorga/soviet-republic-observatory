import { invoke, isTauri } from "@tauri-apps/api/core";
import type { LanguagePackInspection, LanguageStatus } from "./types";

export function nativeLanguageHostAvailable(): boolean {
  return isTauri();
}

export function getNativeLanguageStatus(): Promise<LanguageStatus> {
  return invoke<LanguageStatus>("language_status");
}

export function inspectNativeLanguagePack(
  json: string,
): Promise<LanguagePackInspection> {
  return invoke<LanguagePackInspection>("inspect_language_pack", { json });
}

export function installNativeLanguagePack(
  json: string,
): Promise<LanguageStatus> {
  return invoke<LanguageStatus>("install_language_pack", { json });
}

export function selectNativeLanguagePack(
  packId: string,
): Promise<LanguageStatus> {
  return invoke<LanguageStatus>("select_language_pack", { packId });
}

export function removeNativeLanguagePack(
  packId: string,
): Promise<LanguageStatus> {
  return invoke<LanguageStatus>("remove_language_pack", { packId });
}

export function exportNativeLanguagePack(packId: string): Promise<string> {
  return invoke<string>("export_language_pack", { packId });
}

export function handoverLegacyLanguagePacks(
  manifests: string[],
  selectedLanguagePackId: string | null,
): Promise<LanguageStatus> {
  return invoke<LanguageStatus>("handover_legacy_language_packs", {
    handover: {
      manifests,
      selected_language_pack_id: selectedLanguagePackId,
    },
  });
}
