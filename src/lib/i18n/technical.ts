import technicalManifest from "../../../locales/en-AU-technical.json";
import { sourceLanguagePack } from "./catalog";
import { messageVariables } from "./validation";

export type TechnicalWordingOverlay = {
  schema_version: number;
  source_locale: string;
  source_catalog_version: number;
  source_catalog_revision: number;
  messages: Record<string, string>;
};

export type TechnicalOverlayValidation =
  { ok: true } | { ok: false; detail: string };

export const technicalWordingOverlay =
  technicalManifest as TechnicalWordingOverlay;

export function validateTechnicalWordingOverlay(): TechnicalOverlayValidation {
  if (
    technicalWordingOverlay.schema_version !== 1 ||
    technicalWordingOverlay.source_locale !== sourceLanguagePack.locale ||
    technicalWordingOverlay.source_catalog_version !==
      sourceLanguagePack.source_catalog_version ||
    technicalWordingOverlay.source_catalog_revision !==
      sourceLanguagePack.source_catalog_revision
  ) {
    return { ok: false, detail: "technical overlay identity is out of date" };
  }

  for (const [key, pattern] of Object.entries(
    technicalWordingOverlay.messages,
  )) {
    const sourcePattern = sourceLanguagePack.messages[key];
    if (!sourcePattern) {
      return { ok: false, detail: `unknown technical message ${key}` };
    }
    if (!pattern?.trim()) {
      return { ok: false, detail: `empty technical message ${key}` };
    }
    const sourceVariables = messageVariables(sourcePattern);
    const technicalVariables = messageVariables(pattern);
    if (
      sourceVariables.size !== technicalVariables.size ||
      [...sourceVariables].some((variable) => !technicalVariables.has(variable))
    ) {
      return { ok: false, detail: `variable mismatch in ${key}` };
    }
  }
  return { ok: true };
}

const validation = validateTechnicalWordingOverlay();
if (!validation.ok) {
  throw new Error(`Invalid technical wording overlay: ${validation.detail}`);
}
