import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  LanguageSettingsService,
  MemoryLanguagePackRepository,
} from "./service";

const frenchJson = readFileSync(
  new URL(
    "../../../examples/language-packs/community-fr-example.rolanguage.json",
    import.meta.url,
  ),
  "utf8",
);

describe("language settings lifecycle", () => {
  it("keeps installation, selection, and removal distinct", () => {
    const service = new LanguageSettingsService(
      new MemoryLanguagePackRepository(),
    );
    expect(service.status().selected_language_pack_id).toBe(
      "observatory-en-au",
    );
    const installed = service.install(frenchJson);
    expect(installed.packs).toHaveLength(2);
    expect(installed.selected_language_pack_id).toBe("observatory-en-au");
    expect(
      service.select("community-fr-example").selected_language_pack_id,
    ).toBe("community-fr-example");
    const removed = service.remove("community-fr-example");
    expect(removed.selected_language_pack_id).toBe("observatory-en-au");
    expect(removed.packs).toHaveLength(1);
  });

  it("does not revive a stale selected ID during installation", () => {
    const repository = new MemoryLanguagePackRepository();
    repository.saveSelectedId("community-fr-example");
    const service = new LanguageSettingsService(repository);
    expect(service.status().selected_language_pack_id).toBe(
      "observatory-en-au",
    );
    expect(service.install(frenchJson).selected_language_pack_id).toBe(
      "observatory-en-au",
    );
  });
});
