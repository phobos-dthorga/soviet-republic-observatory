import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import type { AnySchema } from "ajv";
import { describe, expect, it } from "vitest";

function readJson<T>(relativePath: string): T {
  return JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as T;
}

const schema = readJson<AnySchema>(
  "../../../schemas/compatibility-profile-v1.schema.json",
);
const reviewed = readJson<Record<string, unknown>>(
  "../../../compatibility/wrsr-1.1.1.9.rocompat.json",
);
const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile(schema);

function clone(): Record<string, any> {
  return structuredClone(reviewed);
}

describe("W&R compatibility profile v1 schema", () => {
  it("accepts the reviewed W&R 1.1.1.9 profile", () => {
    expect(validate(reviewed), ajv.errorsText(validate.errors)).toBe(true);
  });

  it("accepts a bounded Workshop definition scope with an explicit update policy", () => {
    const scoped = clone();
    scoped.mappings.catalogue_scopes = [catalogueScope()];
    scoped.mappings.definition_directives.push({
      id: "local.example.mod.workers",
      operation: "building.workers_required",
      matches: [{ kind: "exact", value: "$MOD_WORKERS" }],
      catalogue_scope: "local.example.mod",
    });
    expect(validate(scoped), ajv.errorsText(validate.errors)).toBe(true);
  });

  it.each([
    ["unknown fields", (value: any) => (value.script = "alert(1)")],
    ["unsafe IDs", (value: any) => (value.id = "Unsafe/Profile")],
    ["bad versions", (value: any) => (value.version = "latest")],
    [
      "unsafe catalogue source IDs",
      (value: any) =>
        (value.mappings.catalogue_scopes = [
          { ...catalogueScope(), source_id: "workshop.C:\\private" },
        ]),
    ],
    [
      "missing catalogue hashes",
      (value: any) => {
        const scope = catalogueScope();
        delete scope.acknowledged_content_hash;
        value.mappings.catalogue_scopes = [scope];
      },
    ],
    [
      "invalid catalogue update policies",
      (value: any) =>
        (value.mappings.catalogue_scopes = [
          { ...catalogueScope(), update_policy: "always_trust" },
        ]),
    ],
    [
      "attempted save-field package scoping",
      (value: any) =>
        (value.mappings.stats_fields[0].catalogue_scope = "local.example.mod"),
    ],
    [
      "unsupported operations",
      (value: any) =>
        (value.mappings.definition_directives[0].operation = "execute.script"),
    ],
    [
      "unknown host slots",
      (value: any) =>
        (value.mappings.stats_fields[0].host_slot = "private.parser.memory"),
    ],
    [
      "absolute binary paths",
      (value: any) => {
        value.mappings.binary_layouts = [binaryLayout()];
        value.mappings.binary_layouts[0].entry_name =
          "C:\\private\\citizens.bin";
      },
    ],
    [
      "HTML injection",
      (value: any) => (value.description = "<button>Execute</button>"),
    ],
    [
      "URL injection",
      (value: any) => (value.author = "https://example.invalid/profile"),
    ],
    [
      "excessive mappings",
      (value: any) =>
        (value.mappings.stats_fields = Array.from(
          { length: 129 },
          () => value.mappings.stats_fields[0],
        )),
    ],
  ])("rejects %s", (_name, mutate) => {
    const invalid = clone();
    mutate(invalid);
    expect(validate(invalid)).toBe(false);
  });
});

function binaryLayout(): Record<string, unknown> {
  return {
    id: "receiver_counts",
    entry_name: "receiver_counts.bin",
    byte_order: "little",
    base_offset: 0,
    record_count: { kind: "fixed", value: 1 },
    stride: 8,
    max_records: 1,
    magic_checks: [{ offset: 0, bytes_hex: "524F" }],
    fields: [
      {
        host_slot: "core.citizens.electronics.radio",
        offset: 4,
        primitive: "u32",
      },
    ],
  };
}

function catalogueScope(): Record<string, any> {
  return {
    id: "local.example.mod",
    source_id: "workshop.1234567890",
    acknowledged_content_hash: "a".repeat(64),
    update_policy: "exact",
  };
}
