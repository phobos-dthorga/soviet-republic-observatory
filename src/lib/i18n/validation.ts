import { FluentBundle, FluentResource } from "@fluent/bundle";
import { sourceLanguagePack } from "./catalog";
import type {
  LanguagePackManifest,
  LanguageValidationCode,
  LanguageValidationResult,
} from "./types";

export const LANGUAGE_PACK_SCHEMA_VERSION = 1 as const;
export const SOURCE_CATALOG_VERSION = 1 as const;
export const SOURCE_CATALOG_REVISION = 53 as const;
export const SOURCE_LOCALE = "en-AU" as const;
export const DEFAULT_LANGUAGE_PACK_ID = "observatory-en-au" as const;
export const MAX_LANGUAGE_PACK_BYTES = 256 * 1024;
export const MAX_LANGUAGE_MESSAGES = 4_096;
export const MAX_MESSAGE_PATTERN_BYTES = 2_048;

export const PROTECTED_MESSAGE_PREFIXES = [
  "legal-",
  "privacy-",
  "credential-",
  "save-safety-",
  "extension-permission-",
  "security-",
  "data-protection-",
  "destructive-",
  "error-",
  "recovery-",
  "evidence-",
  "coverage-",
  "causality-",
  "synthetic-",
  "research-setup-",
  "attention-",
] as const;

const manifestFields = new Set([
  "schema_version",
  "id",
  "locale",
  "name",
  "author",
  "source_locale",
  "source_catalog_version",
  "source_catalog_revision",
  "direction",
  "messages",
]);

function failure(
  code: LanguageValidationCode,
  detail?: string,
): LanguageValidationResult {
  return { ok: false, code, detail };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validLabel(value: unknown, maximum: number): value is string {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    value.length <= maximum &&
    ![...value].some(
      (character) =>
        /[\p{Cc}<>]/u.test(character) ||
        /[\u202A-\u202E\u2066-\u2069]/u.test(character),
    )
  );
}

function validPackId(value: unknown, community: boolean): value is string {
  return (
    typeof value === "string" &&
    /^(?=.{3,64}$)[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(value) &&
    (!community || !value.startsWith("observatory-"))
  );
}

function validLocale(value: unknown): value is string {
  if (!(
    typeof value === "string" &&
    value.length <= 64 &&
    /^(?:[A-Za-z]{2,3}|[A-Za-z]{5,8})(?:-[A-Za-z0-9]{1,8})*$/.test(value)
  )) {
    return false;
  }
  try {
    return Intl.getCanonicalLocales(value).length === 1;
  } catch {
    return false;
  }
}

export function protectedMessage(key: string): boolean {
  return PROTECTED_MESSAGE_PREFIXES.some((prefix) => key.startsWith(prefix));
}

export function messageVariables(pattern: string): Set<string> {
  return new Set(
    [...pattern.matchAll(/\$([A-Za-z][A-Za-z0-9_-]*)/g)].map(
      (match) => match[1],
    ),
  );
}

function equalSets(left: Set<string>, right: Set<string>): boolean {
  return (
    left.size === right.size && [...left].every((value) => right.has(value))
  );
}

export function messageResource(messageId: string, pattern: string): string {
  const [first = "", ...remaining] = pattern.split("\n");
  return `${messageId} = ${first}${remaining.map((line) => `\n    ${line}`).join("")}`;
}

function validMessagePattern(
  key: string,
  pattern: string,
  locale: string,
): boolean {
  if (
    pattern.trim().length === 0 ||
    new TextEncoder().encode(pattern).length > MAX_MESSAGE_PATTERN_BYTES ||
    pattern.includes("<") ||
    pattern.replaceAll("->", "").includes(">") ||
    [...pattern].some(
      (character) =>
        (/\p{Cc}/u.test(character) &&
          character !== "\n" &&
          character !== "\t") ||
        /[\u202A-\u202E\u2066-\u2069]/u.test(character),
    )
  ) {
    return false;
  }

  const bundle = new FluentBundle(locale, { useIsolating: true });
  const errors = bundle.addResource(
    new FluentResource(messageResource(key, pattern)),
  );
  if (errors.length > 0) return false;
  const message = bundle.getMessage(key);
  if (!message?.value) return false;
  const formatErrors: Error[] = [];
  bundle.formatPattern(
    message.value,
    Object.fromEntries([...messageVariables(pattern)].map((name) => [name, 1])),
    formatErrors,
  );
  return formatErrors.length === 0;
}

function validateManifest(
  value: unknown,
  community: boolean,
): LanguageValidationResult {
  if (!isRecord(value)) return failure("invalid_manifest");
  if ([...Object.keys(value)].some((key) => !manifestFields.has(key))) {
    return failure("invalid_manifest", "unknown manifest field");
  }
  if (
    value.schema_version !== LANGUAGE_PACK_SCHEMA_VERSION ||
    value.source_locale !== SOURCE_LOCALE ||
    value.source_catalog_version !== SOURCE_CATALOG_VERSION ||
    typeof value.source_catalog_revision !== "number" ||
    !Number.isInteger(value.source_catalog_revision) ||
    value.source_catalog_revision < 1 ||
    value.source_catalog_revision > SOURCE_CATALOG_REVISION
  ) {
    return failure("unsupported_version");
  }
  if (!validPackId(value.id, community)) return failure("invalid_identifier");
  if (
    !validLocale(value.locale) ||
    !validLabel(value.name, 80) ||
    (value.author !== undefined && !validLabel(value.author, 80)) ||
    (value.direction !== "left_to_right" && value.direction !== "right_to_left")
  ) {
    return failure("invalid_metadata");
  }
  if (!isRecord(value.messages)) return failure("invalid_message");

  const entries = Object.entries(value.messages);
  if (entries.length === 0 || entries.length > MAX_LANGUAGE_MESSAGES) {
    return failure("invalid_message");
  }
  for (const [key, pattern] of entries) {
    const sourcePattern = sourceLanguagePack.messages[key];
    if (
      !/^[a-z][a-z0-9-]{2,95}$/.test(key) ||
      typeof pattern !== "string" ||
      sourcePattern === undefined
    ) {
      return failure("invalid_message", key);
    }
    if (community && protectedMessage(key)) {
      return failure("protected_message", key);
    }
    if (
      !validMessagePattern(key, pattern, value.locale) ||
      !equalSets(messageVariables(pattern), messageVariables(sourcePattern))
    ) {
      return failure("invalid_message", key);
    }
  }

  return { ok: true, manifest: value as LanguagePackManifest };
}

export function validateCommunityLanguagePackJson(
  manifestJson: string,
): LanguageValidationResult {
  if (new TextEncoder().encode(manifestJson).length > MAX_LANGUAGE_PACK_BYTES) {
    return failure("manifest_too_large");
  }
  try {
    return validateManifest(JSON.parse(manifestJson) as unknown, true);
  } catch {
    return failure("invalid_json");
  }
}

export function validateBuiltInLanguagePack(): LanguageValidationResult {
  return validateManifest(sourceLanguagePack, false);
}

export function eligibleMessageCount(): number {
  return Object.keys(sourceLanguagePack.messages).filter(
    (key) => !protectedMessage(key),
  ).length;
}
