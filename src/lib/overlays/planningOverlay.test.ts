import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import type { AnySchema } from "ajv";
import { describe, expect, it } from "vitest";

function readJson(relativePath: string): Record<string, unknown> {
  return JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as Record<string, unknown>;
}

const schema = readJson(
  "../../../schemas/planning-overlay-v1.schema.json",
) as AnySchema;
const example = readJson(
  "../../../examples/planning-overlays/supplemental-planning-material.rooverlay.json",
);
const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile(schema);

describe("Planning Overlay v1", () => {
  it("accepts the inert supplemental-definition example", () => {
    expect(validate(example), ajv.errorsText(validate.errors)).toBe(true);
  });

  it.each([
    [
      "unknown fields",
      (value: Record<string, unknown>) => (value.sql = "DROP TABLE facts"),
    ],
    [
      "unsafe IDs",
      (value: Record<string, unknown>) => (value.id = "Unsafe/Profile"),
    ],
    [
      "incomplete reverse-domain IDs",
      (value: Record<string, unknown>) => (value.id = "org.example"),
    ],
    [
      "bad versions",
      (value: Record<string, unknown>) => (value.version = "latest"),
    ],
    [
      "HTML",
      (value: Record<string, unknown>) =>
        (value.description = "<script>run()</script>"),
    ],
    [
      "URLs",
      (value: Record<string, unknown>) =>
        (value.author = "https://example.invalid"),
    ],
    [
      "paths",
      (value: Record<string, unknown>) =>
        (value.description = "Read C:\\private\\file.ini"),
    ],
    [
      "renderer configuration",
      (value: Record<string, unknown>) => (value.echarts = { series: [] }),
    ],
    [
      "expressions",
      (value: Record<string, unknown>) =>
        (value.expression = "entity.cost * 2"),
    ],
  ])("rejects %s injection", (_name, mutate) => {
    const invalid = structuredClone(example);
    mutate(invalid);
    expect(validate(invalid)).toBe(false);
  });

  it("rejects excessive supplements and operation values without a precondition", () => {
    const excessive = structuredClone(example) as Record<string, unknown> & {
      supplements: unknown[];
    };
    excessive.supplements = Array.from({ length: 513 }, () =>
      structuredClone(excessive.supplements[0]),
    );
    expect(validate(excessive)).toBe(false);

    const missingPrecondition = structuredClone(example) as Record<
      string,
      unknown
    > & {
      operations: Array<Record<string, unknown>>;
    };
    missingPrecondition.operations = [
      {
        operation: "set",
        entity_id: "base.buildings::building::factory",
        field_id: "building.workers.required",
        value: { kind: "number", number: 50 },
        reason: "Local planning assumption",
      },
    ];
    expect(validate(missingPrecondition)).toBe(false);
  });
});
