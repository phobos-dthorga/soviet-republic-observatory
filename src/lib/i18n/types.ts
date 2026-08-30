export type TextDirection = "left_to_right" | "right_to_left";

export type LanguagePackManifest = {
  schema_version: number;
  id: string;
  locale: string;
  name: string;
  author?: string;
  source_locale: string;
  source_catalog_version: number;
  source_catalog_revision: number;
  direction: TextDirection;
  messages: Record<string, string>;
};

export type LanguagePackTrust = "built_in" | "reviewed" | "community";

export type AvailableLanguagePack = {
  manifest: LanguagePackManifest;
  trust: LanguagePackTrust;
  translated_messages: number;
  eligible_messages: number;
};

export type LanguageStatus = {
  selected_language_pack_id: string;
  active_pack: LanguagePackManifest;
  packs: AvailableLanguagePack[];
  storage_authority: "native_sqlite" | "native_unavailable" | "browser_preview";
};

export type LanguagePackInspection = {
  valid: boolean;
  code?: LanguageServiceErrorCode;
  detail?: string;
  manifest?: LanguagePackManifest;
};

export type LanguageValidationCode =
  | "manifest_too_large"
  | "invalid_json"
  | "invalid_manifest"
  | "unsupported_version"
  | "invalid_identifier"
  | "invalid_metadata"
  | "invalid_message"
  | "protected_message";

export type LanguageServiceErrorCode =
  | LanguageValidationCode
  | "storage_unavailable"
  | "unknown_pack"
  | "built_in_remove";

export type LanguageValidationResult =
  | { ok: true; manifest: LanguagePackManifest }
  | { ok: false; code: LanguageValidationCode; detail?: string };
