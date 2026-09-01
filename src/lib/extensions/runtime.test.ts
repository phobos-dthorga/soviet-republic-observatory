import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import type { AnySchema } from "ajv";
import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import {
  chartSpecForAnalysisContribution,
  type AnalysisPackContribution,
} from "./runtime";

const chartSchema = JSON.parse(
  readFileSync(
    new URL("../../../schemas/chart-spec-v1.schema.json", import.meta.url),
    "utf8",
  ),
) as AnySchema;
const validateChart = new Ajv2020({ strict: true }).compile(chartSchema);
const t = ((key: string, variables?: Record<string, unknown>) =>
  key === "observation-game-date-compact"
    ? `Y${variables?.year} D${variables?.day}`
    : key) as Translator;

const contribution: AnalysisPackContribution = {
  pack_id: "org.example.receiver-laboratory",
  version: "1.0.0",
  content_hash: "a".repeat(64),
  default_locale: "en-AU",
  charts: [
    {
      schema_version: 1,
      id: "receiver-class-shares",
      title: "Receiver class shares",
      description: "One selected branch and scope.",
      kind: "area",
      category_axis_label: "Observation date",
      value_axis_label: "Share",
      unit: "%",
      value_domain: { min: 0, max: 100 },
      provenance: {
        kind: "extension_calculation",
        source: "Host-evaluated pack",
        observed_at: "Y2 D010",
        coverage: "complete",
      },
      series: [
        {
          id: "radio",
          label: "Radio",
          published_metric_id: "core.citizens.electronics.radio",
          stack_id: "receiver_classes",
          provenance: {
            kind: "extension_calculation",
            source: "Host-evaluated rule",
            observed_at: "Y2 D010",
            coverage: "partial",
          },
          points: [
            {
              year: 1,
              day: 365,
              game_day: 730,
              value: 42,
              gap_before: false,
            },
            {
              year: 2,
              day: 10,
              game_day: 740,
              value: 45,
              gap_before: true,
            },
          ],
        },
      ],
    },
  ],
};

describe("Analysis Pack chart resolution", () => {
  it("produces the strict application-owned chart contract", () => {
    const chart = chartSpecForAnalysisContribution(
      contribution,
      contribution.charts[0],
      t,
    );
    expect(validateChart(chart), JSON.stringify(validateChart.errors)).toBe(
      true,
    );
    expect(chart.id).toHaveLength(34);
    expect(chart.series[0].points[1]).toMatchObject({
      category: "Y2 D010",
      category_value: 740,
      gap_before: true,
    });
    expect(chart.series[0].provenance?.kind).toBe("extension_calculation");
  });
});
