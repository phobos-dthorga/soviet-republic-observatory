import { FluentBundle, FluentResource } from "@fluent/bundle";
import { writable } from "svelte/store";
import { sourceLanguagePack } from "./catalog";
import type { TranslationKey } from "./catalog";
import { messageResource, validateBuiltInLanguagePack } from "./validation";
import type { LanguagePackManifest, TextDirection } from "./types";

export type TranslationArguments = Record<string, string | number | Date>;
export type Translator = (
  messageId: TranslationKey,
  arguments_?: TranslationArguments,
  fallback?: string,
) => string;

const builtInValidation = validateBuiltInLanguagePack();
if (!builtInValidation.ok) {
  throw new Error(
    `Invalid built-in language catalogue: ${builtInValidation.code}`,
  );
}

const sourceBundle = buildBundle(sourceLanguagePack);
let activeBundle = sourceBundle;

export const translation = writable<Translator>(translate);
export const activeLocale = writable(sourceLanguagePack.locale);
export const activeDirection = writable<TextDirection>(
  sourceLanguagePack.direction,
);
export const activeLanguagePackId = writable(sourceLanguagePack.id);

export function applyLanguage(manifest: LanguagePackManifest): void {
  activeBundle =
    manifest.id === sourceLanguagePack.id
      ? sourceBundle
      : buildBundle(manifest);
  activeLocale.set(manifest.locale);
  activeDirection.set(manifest.direction);
  activeLanguagePackId.set(manifest.id);
  translation.set(translate);

  if (typeof document !== "undefined") {
    document.documentElement.lang = manifest.locale;
    document.documentElement.dir =
      manifest.direction === "right_to_left" ? "rtl" : "ltr";
    document.documentElement.dataset.languagePack = manifest.id;
  }
}

export function translate(
  messageId: TranslationKey,
  arguments_: TranslationArguments = {},
  fallback: string = messageId,
): string {
  return (
    formatMessage(activeBundle, messageId, arguments_) ??
    formatMessage(sourceBundle, messageId, arguments_) ??
    fallback
  );
}

function buildBundle(manifest: LanguagePackManifest): FluentBundle {
  const bundle = new FluentBundle(manifest.locale, { useIsolating: true });
  for (const [messageId, pattern] of Object.entries(manifest.messages)) {
    bundle.addResource(new FluentResource(messageResource(messageId, pattern)));
  }
  return bundle;
}

function formatMessage(
  bundle: FluentBundle,
  messageId: string,
  arguments_: TranslationArguments,
): string | null {
  const message = bundle.getMessage(messageId);
  if (!message?.value) return null;
  const errors: Error[] = [];
  const formatted = bundle.formatPattern(message.value, arguments_, errors);
  return errors.length === 0 ? formatted : null;
}
