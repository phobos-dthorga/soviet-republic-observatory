import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import type { AnySchema } from "ajv";
import { describe, expect, it } from "vitest";
import {
  evaluateAnalysisOperation,
  RECEIVER_CORE_METRICS,
  validateAnalysisPackSemantics,
  type AnalysisPack,
} from "./analysisPack";

function readJson<T>(relativePath: string): T {
  return JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as T;
}

const analysisSchema = readJson<AnySchema>(
  "../../../schemas/analysis-pack-v1.schema.json",
);
const chartTemplateSchema = readJson<AnySchema>(
  "../../../schemas/chart-template-v1.schema.json",
);
const chartSpecSchema = readJson<AnySchema>(
  "../../../schemas/chart-spec-v1.schema.json",
);
const examplePack = readJson<AnalysisPack>(
  "../../../examples/analysis-packs/receiver-adoption-laboratory.roanalysis.json",
);

const ajv = new Ajv2020({ allErrors: true, strict: true });
ajv.addSchema(chartTemplateSchema);
const validatePackSchema = ajv.compile(analysisSchema);
const validateChartSpecSchema = ajv.compile(chartSpecSchema);
const availableCoreMetrics = new Set<string>(RECEIVER_CORE_METRICS);

function cloneExample(): AnalysisPack {
  return structuredClone(examplePack);
}

describe("Analysis Pack v1", () => {
  it("accepts the Receiver Adoption Laboratory proof", () => {
    expect(
      validatePackSchema(examplePack),
      ajv.errorsText(validatePackSchema.errors),
    ).toBe(true);
    expect(
      validateAnalysisPackSemantics(examplePack, availableCoreMetrics),
    ).toEqual([]);
  });

  it.each([
    [
      "unknown fields",
      (pack: Record<string, unknown>) => (pack.unknown = true),
    ],
    [
      "unsafe IDs",
      (pack: Record<string, unknown>) => (pack.id = "Unsafe/Pack"),
    ],
    ["bad versions", (pack: Record<string, unknown>) => (pack.version = "one")],
    [
      "unsupported operations",
      (pack: AnalysisPack) =>
        ((pack.derived_metrics[0].operation as { kind: string }).kind = "eval"),
    ],
    [
      "excessive metric limits",
      (pack: AnalysisPack) =>
        (pack.derived_metrics = Array.from({ length: 65 }, (_, index) => ({
          ...pack.derived_metrics[0],
          id: `metric_${index}`,
        }))),
    ],
    [
      "script injection",
      (pack: AnalysisPack) =>
        Object.assign(pack.derived_metrics[0].operation, {
          script: "return window.fetch('/secrets')",
        }),
    ],
    [
      "HTML injection",
      (pack: AnalysisPack) =>
        (pack.description = "<button onclick='run()'>Run</button>"),
    ],
    [
      "ECharts injection",
      (pack: AnalysisPack) =>
        Object.assign(pack.charts[0], {
          echarts: { tooltip: { formatter: "javascript:run()" } },
        }),
    ],
    [
      "URL references",
      (pack: AnalysisPack) => (pack.author = "https://example.invalid/author"),
    ],
    [
      "filesystem references",
      (pack: AnalysisPack) =>
        (pack.description = "Load C:\\analysis\\private-model.json"),
    ],
  ])("rejects %s", (_name, mutate) => {
    const invalid = cloneExample() as AnalysisPack & Record<string, unknown>;
    mutate(invalid);
    expect(validatePackSchema(invalid)).toBe(false);
  });

  it("rejects duplicate IDs semantically", () => {
    const invalid = cloneExample();
    invalid.derived_metrics[1].id = invalid.derived_metrics[0].id;
    invalid.charts.push(structuredClone(invalid.charts[0]));
    invalid.charts[0].series[1].id = invalid.charts[0].series[0].id;

    expect(validatePackSchema(invalid)).toBe(true);
    expect(
      validateAnalysisPackSemantics(invalid, availableCoreMetrics).map(
        (issue) => issue.code,
      ),
    ).toEqual(
      expect.arrayContaining([
        "duplicate_derived_metric",
        "duplicate_chart",
        "duplicate_series",
      ]),
    );
  });

  it("rejects forward references, missing core metrics, and invalid domains", () => {
    const invalid = cloneExample();
    invalid.derived_metrics[0].operation = {
      kind: "sum",
      operands: [
        { derived_metric: "radio_share" },
        { core_metric: "core.citizens.electronics.pager" },
      ],
    };
    invalid.charts[0].value_domain = { min: 100, max: 0 };

    expect(validatePackSchema(invalid)).toBe(true);
    expect(
      validateAnalysisPackSemantics(invalid, availableCoreMetrics).map(
        (issue) => issue.code,
      ),
    ).toEqual(
      expect.arrayContaining([
        "forward_or_unknown_derived_metric",
        "unknown_core_metric",
        "invalid_chart_domain",
      ]),
    );
  });

  it("makes safe ratios unavailable for zero, missing, or non-finite denominators", () => {
    const operation = {
      kind: "safe_ratio" as const,
      numerator: { core_metric: "core.numerator" },
      denominator: { core_metric: "core.denominator" },
      scale: 100,
    };
    const numerator = 3;

    for (const denominator of [
      0,
      undefined,
      Number.NaN,
      Number.POSITIVE_INFINITY,
    ]) {
      expect(
        evaluateAnalysisOperation(operation, (reference) =>
          "core_metric" in reference &&
          reference.core_metric === "core.numerator"
            ? numerator
            : denominator,
        ),
      ).toBeNull();
    }
    expect(
      evaluateAnalysisOperation(operation, (reference) =>
        "core_metric" in reference && reference.core_metric === "core.numerator"
          ? numerator
          : 12,
      ),
    ).toBe(25);
  });

  it("accepts a concrete stacked chart with negative values and series provenance", () => {
    const chart = {
      schema_version: 1,
      id: "signed-preview",
      title: "Signed preview",
      description: "Exercises the concrete application-owned chart contract.",
      kind: "bar",
      orientation: "horizontal",
      value_domain: { min: -5, max: 5 },
      series: [
        {
          id: "effects",
          label: "Expected effect",
          stack_id: "effects",
          points: [{ category: "Religion sympathy", value: -1.8 }],
          provenance: {
            kind: "extension_calculation",
            source: "Synthetic test",
            observed_at: "2004-08-17",
            coverage: "experimental",
          },
        },
      ],
      provenance: {
        kind: "calculation",
        source: "Synthetic test",
        observed_at: "2004-08-17",
        coverage: "complete",
      },
    };

    expect(
      validateChartSpecSchema(chart),
      ajv.errorsText(validateChartSpecSchema.errors),
    ).toBe(true);
  });
});
