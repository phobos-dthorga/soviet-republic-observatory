import { sourceLanguagePack } from "./catalog";
import { protectedMessage } from "./validation";
import type { LanguagePackManifest, TextDirection } from "./types";

const substitutions: Record<string, string> = {
  a: "áá",
  A: "ÁÁ",
  e: "ëë",
  E: "ËË",
  i: "ïï",
  I: "ÏÏ",
  o: "ôô",
  O: "ÔÔ",
  u: "üü",
  U: "ÜÜ",
};

export function pseudoLocalisePattern(pattern: string): string {
  return pattern
    .split(/(\{[^}]+\})/g)
    .map((part) =>
      part.startsWith("{")
        ? part
        : [...part]
            .map((character) => substitutions[character] ?? character)
            .join("")
            .replace(/([.!?])(?=\s|$)/g, "$1~"),
    )
    .join("");
}

export function createPseudoLanguagePack(
  direction: TextDirection = "left_to_right",
): LanguagePackManifest {
  const rtl = direction === "right_to_left";
  return {
    schema_version: 1,
    id: rtl ? "pseudo-ar-xb-test" : "pseudo-en-xa-test",
    locale: rtl ? "ar-XB" : "en-XA",
    name: rtl ? "Pseudo RTL test" : "Pseudo expanded test",
    author: "Republic Observatory test fixture",
    source_locale: sourceLanguagePack.source_locale,
    source_catalog_version: sourceLanguagePack.source_catalog_version,
    source_catalog_revision: sourceLanguagePack.source_catalog_revision,
    direction,
    messages: Object.fromEntries(
      Object.entries(sourceLanguagePack.messages)
        .filter(([key]) => !protectedMessage(key))
        .map(([key, pattern]) => [key, `⟦${pseudoLocalisePattern(pattern)}⟧`]),
    ),
  };
}
