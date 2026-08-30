import { get, writable } from "svelte/store";
import { sourceLanguagePack, type TranslationKey } from "./catalog";
import { applyLanguage } from "./runtime";
import {
  DEFAULT_LANGUAGE_PACK_ID,
  eligibleMessageCount,
  validateCommunityLanguagePackJson,
} from "./validation";
import type {
  AvailableLanguagePack,
  LanguagePackManifest,
  LanguageStatus,
  LanguagePackInspection,
  LanguageServiceErrorCode,
} from "./types";
import {
  exportNativeLanguagePack,
  getNativeLanguageStatus,
  handoverLegacyLanguagePacks,
  inspectNativeLanguagePack,
  installNativeLanguagePack,
  nativeLanguageHostAvailable,
  removeNativeLanguagePack,
  selectNativeLanguagePack,
} from "./desktopClient";

export type { LanguageServiceErrorCode } from "./types";

const LEGACY_PACKS_KEY = "republic-observatory.language-packs.v1";
const LEGACY_SELECTED_KEY = "republic-observatory.selected-language.v1";

export class LanguageServiceError extends Error {
  constructor(
    public readonly code: LanguageServiceErrorCode,
    public readonly detail?: string,
  ) {
    super(code);
  }
}

export interface LanguagePackRepository {
  loadSelectedId(): string | null;
  saveSelectedId(packId: string): void;
  listManifests(): string[];
  saveManifest(packId: string, manifestJson: string): void;
  removeManifest(packId: string): void;
}

export class MemoryLanguagePackRepository implements LanguagePackRepository {
  private selectedId: string | null = null;
  private readonly manifests = new Map<string, string>();

  loadSelectedId(): string | null {
    return this.selectedId;
  }

  saveSelectedId(packId: string): void {
    this.selectedId = packId;
  }

  listManifests(): string[] {
    return [...this.manifests.values()];
  }

  saveManifest(packId: string, manifestJson: string): void {
    this.manifests.set(packId, manifestJson);
  }

  removeManifest(packId: string): void {
    this.manifests.delete(packId);
  }
}

class BrowserLanguagePackRepository implements LanguagePackRepository {
  loadSelectedId(): string | null {
    return localStorage.getItem(LEGACY_SELECTED_KEY);
  }

  saveSelectedId(packId: string): void {
    localStorage.setItem(LEGACY_SELECTED_KEY, packId);
  }

  listManifests(): string[] {
    let parsed: unknown;
    try {
      parsed = JSON.parse(localStorage.getItem(LEGACY_PACKS_KEY) ?? "{}");
    } catch {
      return [];
    }
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed))
      return [];
    return Object.values(parsed).filter(
      (value): value is string => typeof value === "string",
    );
  }

  saveManifest(packId: string, manifestJson: string): void {
    const manifests = Object.fromEntries(
      this.listManifests().flatMap((stored) => {
        const result = validateCommunityLanguagePackJson(stored);
        return result.ok ? [[result.manifest.id, stored]] : [];
      }),
    );
    manifests[packId] = manifestJson;
    localStorage.setItem(LEGACY_PACKS_KEY, JSON.stringify(manifests));
  }

  removeManifest(packId: string): void {
    const manifests = Object.fromEntries(
      this.listManifests().flatMap((stored) => {
        const result = validateCommunityLanguagePackJson(stored);
        return result.ok && result.manifest.id !== packId
          ? [[result.manifest.id, stored]]
          : [];
      }),
    );
    localStorage.setItem(LEGACY_PACKS_KEY, JSON.stringify(manifests));
  }
}

export class LanguageSettingsService {
  constructor(private readonly repository: LanguagePackRepository) {}

  status(): LanguageStatus {
    const eligible = eligibleMessageCount();
    const packs: AvailableLanguagePack[] = [
      {
        manifest: sourceLanguagePack,
        trust: "built_in",
        translated_messages: eligible,
        eligible_messages: eligible,
      },
    ];

    for (const stored of this.repository.listManifests()) {
      const result = validateCommunityLanguagePackJson(stored);
      if (
        !result.ok ||
        packs.some((pack) => pack.manifest.id === result.manifest.id)
      ) {
        continue;
      }
      packs.push({
        manifest: result.manifest,
        trust: "community",
        translated_messages: Object.keys(result.manifest.messages).length,
        eligible_messages: eligible,
      });
    }

    const requested =
      this.repository.loadSelectedId() ?? DEFAULT_LANGUAGE_PACK_ID;
    const active =
      packs.find((pack) => pack.manifest.id === requested)?.manifest ??
      sourceLanguagePack;
    return {
      selected_language_pack_id: active.id,
      active_pack: active,
      packs,
      storage_authority: "browser_preview",
    };
  }

  install(manifestJson: string): LanguageStatus {
    const previouslySelected = this.status().selected_language_pack_id;
    const result = validateCommunityLanguagePackJson(manifestJson);
    if (!result.ok) throw new LanguageServiceError(result.code, result.detail);
    this.repository.saveManifest(
      result.manifest.id,
      JSON.stringify(result.manifest),
    );
    this.repository.saveSelectedId(previouslySelected);
    return this.status();
  }

  select(packId: string): LanguageStatus {
    const status = this.status();
    if (!status.packs.some((pack) => pack.manifest.id === packId)) {
      throw new LanguageServiceError("unknown_pack");
    }
    this.repository.saveSelectedId(packId);
    return this.status();
  }

  remove(packId: string): LanguageStatus {
    if (packId === DEFAULT_LANGUAGE_PACK_ID) {
      throw new LanguageServiceError("built_in_remove");
    }
    const status = this.status();
    if (!status.packs.some((pack) => pack.manifest.id === packId)) {
      throw new LanguageServiceError("unknown_pack");
    }
    this.repository.removeManifest(packId);
    if (status.selected_language_pack_id === packId) {
      this.repository.saveSelectedId(DEFAULT_LANGUAGE_PACK_ID);
    }
    return this.status();
  }
}

export const languageStatus = writable<LanguageStatus>({
  selected_language_pack_id: DEFAULT_LANGUAGE_PACK_ID,
  active_pack: sourceLanguagePack,
  packs: [],
  storage_authority: nativeLanguageHostAvailable()
    ? "native_sqlite"
    : "browser_preview",
});

let browserService: LanguageSettingsService | undefined;

function service(): LanguageSettingsService {
  if (!browserService) {
    browserService = new LanguageSettingsService(
      typeof localStorage === "undefined"
        ? new MemoryLanguagePackRepository()
        : new BrowserLanguagePackRepository(),
    );
  }
  return browserService;
}

function activate(status: LanguageStatus): LanguageStatus {
  languageStatus.set(status);
  applyLanguage(status.active_pack);
  return status;
}

export function initializeLanguage(): LanguageStatus {
  if (nativeLanguageHostAvailable()) {
    const initial = activate(fallbackStatus("native_sqlite"));
    void initializeNativeLanguage();
    return initial;
  }
  try {
    return activate(service().status());
  } catch {
    return activate(fallbackStatus("browser_preview"));
  }
}

export async function installLanguagePack(
  manifestJson: string,
): Promise<LanguageStatus> {
  try {
    const status = nativeLanguageHostAvailable()
      ? await installNativeLanguagePack(manifestJson)
      : service().install(manifestJson);
    return activate(status);
  } catch (error) {
    throw normalizedLanguageError(error);
  }
}

export async function selectLanguagePack(
  packId: string,
): Promise<LanguageStatus> {
  try {
    const status = nativeLanguageHostAvailable()
      ? await selectNativeLanguagePack(packId)
      : service().select(packId);
    return activate(status);
  } catch (error) {
    throw normalizedLanguageError(error);
  }
}

export async function removeLanguagePack(
  packId: string,
): Promise<LanguageStatus> {
  try {
    const status = nativeLanguageHostAvailable()
      ? await removeNativeLanguagePack(packId)
      : service().remove(packId);
    return activate(status);
  } catch (error) {
    throw normalizedLanguageError(error);
  }
}

export async function inspectLanguagePack(
  manifestJson: string,
): Promise<LanguagePackInspection> {
  if (nativeLanguageHostAvailable()) {
    try {
      return await inspectNativeLanguagePack(manifestJson);
    } catch (error) {
      throw normalizedLanguageError(error);
    }
  }
  const result = validateCommunityLanguagePackJson(manifestJson);
  return result.ok
    ? { valid: true, manifest: result.manifest }
    : {
        valid: false,
        code: result.code,
        detail: result.detail,
      };
}

export async function exportLanguagePack(packId: string): Promise<string> {
  try {
    if (nativeLanguageHostAvailable()) {
      return await exportNativeLanguagePack(packId);
    }
    const pack = service()
      .status()
      .packs.find((candidate) => candidate.manifest.id === packId);
    if (!pack) throw new LanguageServiceError("unknown_pack");
    return JSON.stringify(pack.manifest, null, 2);
  } catch (error) {
    throw normalizedLanguageError(error);
  }
}

export const languageErrorMessageKeys: Record<
  LanguageServiceErrorCode,
  TranslationKey
> = {
  manifest_too_large: "error-language-manifest-too-large",
  invalid_json: "error-language-invalid-json",
  invalid_manifest: "error-language-invalid-manifest",
  unsupported_version: "error-language-unsupported-version",
  invalid_identifier: "error-language-invalid-identifier",
  invalid_metadata: "error-language-invalid-metadata",
  invalid_message: "error-language-invalid-message",
  protected_message: "error-language-protected-message",
  storage_unavailable: "error-language-storage-unavailable",
  unknown_pack: "error-language-unknown-pack",
  built_in_remove: "error-language-built-in-remove",
};

export function currentLanguageStatus(): LanguageStatus {
  return get(languageStatus);
}

function fallbackStatus(
  storageAuthority: LanguageStatus["storage_authority"],
): LanguageStatus {
  const eligible = eligibleMessageCount();
  return {
    selected_language_pack_id: DEFAULT_LANGUAGE_PACK_ID,
    active_pack: sourceLanguagePack,
    packs: [
      {
        manifest: sourceLanguagePack,
        trust: "built_in",
        translated_messages: eligible,
        eligible_messages: eligible,
      },
    ],
    storage_authority: storageAuthority,
  };
}

async function initializeNativeLanguage(): Promise<void> {
  try {
    const legacy = readLegacyLanguageState();
    const status = await handoverLegacyLanguagePacks(
      legacy.manifests,
      legacy.selectedId,
    );
    clearLegacyLanguageState();
    activate(status);
  } catch {
    try {
      activate(await getNativeLanguageStatus());
    } catch {
      activate(fallbackStatus("native_unavailable"));
    }
  }
}

function readLegacyLanguageState(): {
  manifests: string[];
  selectedId: string | null;
} {
  if (typeof localStorage === "undefined") {
    return { manifests: [], selectedId: null };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(localStorage.getItem(LEGACY_PACKS_KEY) ?? "{}");
  } catch {
    parsed = {};
  }
  const manifests =
    typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
      ? Object.values(parsed).filter(
          (value): value is string =>
            typeof value === "string" &&
            validateCommunityLanguagePackJson(value).ok,
        )
      : [];
  return {
    manifests,
    selectedId: localStorage.getItem(LEGACY_SELECTED_KEY),
  };
}

function clearLegacyLanguageState(): void {
  if (typeof localStorage === "undefined") return;
  localStorage.removeItem(LEGACY_PACKS_KEY);
  localStorage.removeItem(LEGACY_SELECTED_KEY);
}

function normalizedLanguageError(error: unknown): LanguageServiceError {
  if (error instanceof LanguageServiceError) return error;
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    error.code in languageErrorMessageKeys
  ) {
    return new LanguageServiceError(
      error.code as LanguageServiceErrorCode,
      "diagnostic" in error && typeof error.diagnostic === "string"
        ? error.diagnostic
        : undefined,
    );
  }
  return new LanguageServiceError("storage_unavailable");
}
