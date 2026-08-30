import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import type { AnySchema } from "ajv";
import { describe, expect, it } from "vitest";
import { sourceLanguagePack } from "./catalog";
import { createPseudoLanguagePack } from "./pseudo";
import {
  eligibleMessageCount,
  validateBuiltInLanguagePack,
  validateCommunityLanguagePackJson,
} from "./validation";
import type { LanguagePackManifest } from "./types";

function readJson<T>(relativePath: string): T {
  return JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as T;
}

const schema = readJson<AnySchema>(
  "../../../schemas/language-pack-v1.schema.json",
);
const french = readJson<LanguagePackManifest>(
  "../../../examples/language-packs/community-fr-example.rolanguage.json",
);
const validateSchema = new Ajv2020({ allErrors: true, strict: true }).compile(
  schema,
);
const cloneFrench = () =>
  structuredClone(french) as LanguagePackManifest & Record<string, unknown>;

describe("language pack v1", () => {
  it("accepts the canonical catalogue and the partial community example", () => {
    expect(validateBuiltInLanguagePack()).toEqual({
      ok: true,
      manifest: sourceLanguagePack,
    });
    expect(validateSchema(french), JSON.stringify(validateSchema.errors)).toBe(
      true,
    );
    expect(validateCommunityLanguagePackJson(JSON.stringify(french)).ok).toBe(
      true,
    );
    expect(eligibleMessageCount()).toBeLessThan(
      Object.keys(sourceLanguagePack.messages).length,
    );
  });

  it.each([
    [
      "unknown field",
      (pack: Record<string, unknown>) => (pack.extra = true),
      "invalid_manifest",
    ],
    [
      "reserved ID",
      (pack: Record<string, unknown>) => (pack.id = "observatory-impostor"),
      "invalid_identifier",
    ],
    [
      "bad schema",
      (pack: Record<string, unknown>) => (pack.schema_version = 2),
      "unsupported_version",
    ],
    [
      "future revision",
      (pack: Record<string, unknown>) => (pack.source_catalog_revision = 31),
      "unsupported_version",
    ],
    [
      "bad locale",
      (pack: Record<string, unknown>) => (pack.locale = "fr<script>"),
      "invalid_metadata",
    ],
    [
      "unknown message",
      (pack: LanguagePackManifest) =>
        (pack.messages["unknown-message"] = "Non"),
      "invalid_message",
    ],
    [
      "variable mismatch",
      (pack: LanguagePackManifest) =>
        (pack.messages["chart-accessible-label"] = "{ $title }"),
      "invalid_message",
    ],
    [
      "markup",
      (pack: LanguagePackManifest) =>
        (pack.messages["nav-primary"] = "<strong>Menu</strong>"),
      "invalid_message",
    ],
    [
      "bidi control",
      (pack: LanguagePackManifest) =>
        (pack.messages["nav-primary"] = "Menu\u202E"),
      "invalid_message",
    ],
    [
      "message reference",
      (pack: LanguagePackManifest) =>
        (pack.messages["nav-primary"] = "{ nav-broadcast }"),
      "invalid_message",
    ],
    [
      "protected message",
      (pack: LanguagePackManifest) =>
        (pack.messages["evidence-save-fact"] = "Fait"),
      "protected_message",
    ],
  ])("rejects %s", (_name, mutate, expectedCode) => {
    const pack = cloneFrench();
    mutate(pack);
    const result = validateCommunityLanguagePackJson(JSON.stringify(pack));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.code).toBe(expectedCode);
  });

  it("rejects invalid JSON and files over the byte limit", () => {
    expect(validateCommunityLanguagePackJson("{")).toMatchObject({
      ok: false,
      code: "invalid_json",
    });
    expect(
      validateCommunityLanguagePackJson(" ".repeat(256 * 1024 + 1)),
    ).toMatchObject({ ok: false, code: "manifest_too_large" });
  });

  it("keeps variables intact in expanded and RTL pseudo packs", () => {
    for (const direction of ["left_to_right", "right_to_left"] as const) {
      const pack = createPseudoLanguagePack(direction);
      const result = validateCommunityLanguagePackJson(JSON.stringify(pack));
      expect(
        result,
        result.ok ? undefined : `${result.code}: ${result.detail}`,
      ).toMatchObject({ ok: true });
      expect(pack.messages["chart-accessible-label"]).toContain("{ $title }");
      expect(pack.messages["chart-accessible-label"]).toContain(
        "{ $description }",
      );
      expect(pack.messages["language-introduction"].length).toBeGreaterThan(
        sourceLanguagePack.messages["language-introduction"].length,
      );
    }
  });
});
