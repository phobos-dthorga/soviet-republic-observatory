import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ThemeInspection, ThemeStatus } from "./types";

export function nativeThemeHostAvailable(): boolean {
  return isTauri();
}

export function getNativeThemeStatus(): Promise<ThemeStatus> {
  return invoke<ThemeStatus>("theme_status");
}

export function inspectNativeTheme(document: string): Promise<ThemeInspection> {
  return invoke<ThemeInspection>("inspect_theme", { document });
}

export function importNativeTheme(document: string): Promise<ThemeStatus> {
  return invoke<ThemeStatus>("import_theme", { document });
}

export function selectNativeTheme(
  themeId: string,
  version: string,
  contentHash: string,
): Promise<ThemeStatus> {
  return invoke<ThemeStatus>("select_theme", {
    themeId,
    version,
    contentHash,
  });
}

export function exportNativeTheme(
  themeId: string,
  version: string,
  contentHash: string,
): Promise<string> {
  return invoke<string>("export_theme", { themeId, version, contentHash });
}

export function removeNativeTheme(
  themeId: string,
  version: string,
): Promise<ThemeStatus> {
  return invoke<ThemeStatus>("remove_theme", { themeId, version });
}
