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
  LanguageValidationCode,
} from "./types";

export type LanguageServiceErrorCode =
  | LanguageValidationCode
  | "storage_unavailable"
  | "unknown_pack"
  | "built_in_remove";

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
  private readonly packsKey = "republic-observatory.language-packs.v1";
  private readonly selectedKey = "republic-observatory.selected-language.v1";

  loadSelectedId(): string | null {
    return localStorage.getItem(this.selectedKey);
  }

  saveSelectedId(packId: string): void {
    localStorage.setItem(this.selectedKey, packId);
  }

  listManifests(): string[] {
    let parsed: unknown;
    try {
      parsed = JSON.parse(localStorage.getItem(this.packsKey) ?? "{}");
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
    localStorage.setItem(this.packsKey, JSON.stringify(manifests));
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
    localStorage.setItem(this.packsKey, JSON.stringify(manifests));
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
  try {
    return activate(service().status());
  } catch {
    return activate({
      selected_language_pack_id: DEFAULT_LANGUAGE_PACK_ID,
      active_pack: sourceLanguagePack,
      packs: [
        {
          manifest: sourceLanguagePack,
          trust: "built_in",
          translated_messages: eligibleMessageCount(),
          eligible_messages: eligibleMessageCount(),
        },
      ],
    });
  }
}

export function installLanguagePack(manifestJson: string): LanguageStatus {
  try {
    const status = service().install(manifestJson);
    languageStatus.set(status);
    return status;
  } catch (error) {
    if (error instanceof LanguageServiceError) throw error;
    throw new LanguageServiceError("storage_unavailable");
  }
}

export function selectLanguagePack(packId: string): LanguageStatus {
  try {
    return activate(service().select(packId));
  } catch (error) {
    if (error instanceof LanguageServiceError) throw error;
    throw new LanguageServiceError("storage_unavailable");
  }
}

export function removeLanguagePack(packId: string): LanguageStatus {
  try {
    return activate(service().remove(packId));
  } catch (error) {
    if (error instanceof LanguageServiceError) throw error;
    throw new LanguageServiceError("storage_unavailable");
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
