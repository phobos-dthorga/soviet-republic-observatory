import { get } from "svelte/store";
import { afterEach, describe, expect, it, vi } from "vitest";
import { sourceLanguagePack } from "./catalog";
import {
  activeDirection,
  activeLocale,
  applyLanguage,
  applyWordingMode,
  translate,
} from "./runtime";
import type { LanguagePackManifest } from "./types";

const partial: LanguagePackManifest = {
  schema_version: 1,
  id: "community-fr-runtime",
  locale: "fr",
  name: "Français",
  source_locale: "en-AU",
  source_catalog_version: 1,
  source_catalog_revision: 1,
  direction: "right_to_left",
  messages: {
    "nav-briefing": "Rapport",
    "chart-accessible-label": "{ $title }. { $description }",
  },
};

afterEach(() => {
  vi.unstubAllGlobals();
  applyWordingMode("player_friendly");
  applyLanguage(sourceLanguagePack);
});

describe("translation runtime", () => {
  it("layers the selected pack over canonical English", () => {
    applyLanguage(partial);
    expect(translate("nav-briefing")).toBe("Rapport");
    expect(translate("nav-broadcast")).toBe("Broadcast");
    expect(
      translate("chart-accessible-label", {
        title: "Titre",
        description: "Description",
      }),
    ).toContain("Titre");
    expect(get(activeLocale)).toBe("fr");
    expect(get(activeDirection)).toBe("right_to_left");
  });

  it("uses technical wording only with built-in English", () => {
    applyLanguage(sourceLanguagePack);
    applyWordingMode("technical");
    expect(translate("settings-rebuild-action")).toBe(
      "Rebuild analytical warehouse",
    );

    applyLanguage(partial);
    expect(translate("settings-rebuild-action")).toBe(
      sourceLanguagePack.messages["settings-rebuild-action"],
    );
    expect(translate("nav-briefing")).toBe("Rapport");
  });

  it("formats canonical plural branches", () => {
    applyLanguage(sourceLanguagePack);
    expect(translate("saves-observed", { count: 1 })).toContain(
      "1 save observed",
    );
    expect(
      translate("saves-observed", { count: 2 }).replace(/[\u2068\u2069]/g, ""),
    ).toContain("2 saves observed");
  });

  it("applies locale and direction to the document boundary", () => {
    const documentElement = {
      lang: "",
      dir: "",
      dataset: {} as Record<string, string>,
    };
    vi.stubGlobal("document", { documentElement });
    applyLanguage(partial);
    applyWordingMode("technical");
    expect(documentElement).toMatchObject({
      lang: "fr",
      dir: "rtl",
      dataset: {
        languagePack: "community-fr-runtime",
        wordingMode: "technical",
      },
    });
  });
});
