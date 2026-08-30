import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";

const schema = JSON.parse(
  readFileSync(resolve("schemas/theme-v1.schema.json"), "utf8"),
);
const fixture = JSON.parse(
  readFileSync(resolve("schemas/fixtures/theme-v1.rotheme.json"), "utf8"),
);
const validate = new Ajv2020({ strict: true, allErrors: true }).compile(schema);

describe("theme schema v1", () => {
  it("accepts the inert example", () => {
    expect(
      validate(structuredClone(fixture)),
      JSON.stringify(validate.errors),
    ).toBe(true);
  });

  it.each([
    "css",
    "html",
    "script",
    "url",
    "path",
    "echarts",
    "opacity",
    "font",
  ])("rejects the forbidden %s capability", (field) => {
    const malicious = structuredClone(fixture) as Record<string, unknown>;
    malicious[field] = "arbitrary capability";
    expect(validate(malicious)).toBe(false);
  });

  it("rejects non-hex colours and excessive palettes", () => {
    const unsafe = structuredClone(fixture);
    unsafe.colours.canvas = "rgba(0, 0, 0, .5)";
    unsafe.chart_palette = Array.from({ length: 9 }, () => "#FFFFFF");
    expect(validate(unsafe)).toBe(false);
  });
});
