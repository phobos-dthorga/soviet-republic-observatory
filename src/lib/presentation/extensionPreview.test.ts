import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { receiverPackPreview } from "./extensionPreview";

const example = JSON.parse(
  readFileSync(
    new URL(
      "../../../examples/analysis-packs/receiver-adoption-laboratory.roanalysis.json",
      import.meta.url,
    ),
    "utf8",
  ),
) as {
  id: string;
  name: string;
  author: string;
  version: string;
  host_api_version: number;
  default_locale?: string;
  derived_metrics: Array<{ id: string }>;
};

describe("synthetic extension preview", () => {
  it("mirrors the checked-in proof without loading it at runtime", () => {
    expect(receiverPackPreview).toMatchObject({
      id: example.id,
      name: example.name,
      author: example.author,
      version: example.version,
      hostApi: String(example.host_api_version),
      defaultLocale: example.default_locale ?? "en-AU",
      derivedMetrics: example.derived_metrics.map((metric) => metric.id),
    });
  });
});
