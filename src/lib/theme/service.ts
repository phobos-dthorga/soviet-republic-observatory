import { writable } from "svelte/store";
import {
  exportNativeTheme,
  getNativeThemeStatus,
  importNativeTheme,
  inspectNativeTheme,
  nativeThemeHostAvailable,
  removeNativeTheme,
  selectNativeTheme,
} from "./desktopClient";
import { applyTheme } from "./runtime";
import type { ThemeInspection, ThemeStatus } from "./types";

export const themeStatus = writable<ThemeStatus | null>(null);

export async function initialiseThemes(): Promise<ThemeStatus | null> {
  if (!nativeThemeHostAvailable()) return null;
  const status = await getNativeThemeStatus();
  return accept(status);
}

export function inspectTheme(document: string): Promise<ThemeInspection> {
  return inspectNativeTheme(document);
}

export async function importTheme(document: string): Promise<ThemeStatus> {
  return accept(await importNativeTheme(document));
}

export async function selectTheme(
  themeId: string,
  version: string,
  contentHash: string,
): Promise<ThemeStatus> {
  return accept(await selectNativeTheme(themeId, version, contentHash));
}

export function exportTheme(
  themeId: string,
  version: string,
  contentHash: string,
): Promise<string> {
  return exportNativeTheme(themeId, version, contentHash);
}

export async function removeTheme(
  themeId: string,
  version: string,
): Promise<ThemeStatus> {
  return accept(await removeNativeTheme(themeId, version));
}

function accept(status: ThemeStatus): ThemeStatus {
  themeStatus.set(status);
  applyTheme(status.active_theme, status.active_report);
  return status;
}
