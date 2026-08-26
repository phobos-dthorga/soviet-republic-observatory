import { describe, expect, it } from "vitest";
import {
  expandedCategories,
  expandedValues,
  optionForChart,
  provenanceForSeries,
} from "./chartOptions";
import type { ChartSpec } from "./types";

const baseSpec: ChartSpec = {
  schema_version: 1,
  id: "test",
  title: "Test chart",
  description: "Synthetic test",
  kind: "line",
  unit: "%",
  series: [
    {
      id: "actual",
      label: "Actual",
      points: [
        { category: "1979 Q1", value: 80 },
        { category: "1979 Q2", value: 84, gap_before: true },
      ],
    },
  ],
  provenance: {
    kind: "calculation",
    source: "Synthetic fixture",
    observed_at: "2004-08-17",
    coverage: "complete",
  },
};

describe("chart options", () => {
  it("preserves an explicit observation gap", () => {
    expect(expandedCategories(baseSpec.series[0].points)).toEqual([
      "1979 Q1",
      "1979 Q2 · no observation",
      "1979 Q2",
    ]);
    expect(expandedValues(baseSpec.series[0].points)).toEqual([80, null, 84]);
  });

  it("uses horizontal value and category axes for a horizontal bar", () => {
    const option = optionForChart({
      ...baseSpec,
      kind: "bar",
      orientation: "horizontal",
    }) as { xAxis: { type: string }; yAxis: { type: string } };

    expect(option.xAxis.type).toBe("value");
    expect(option.yAxis.type).toBe("category");
  });

  it("disables animation for reduced motion", () => {
    const option = optionForChart(baseSpec, undefined, true) as {
      animationDuration: number;
    };

    expect(option.animationDuration).toBe(0);
  });

  it("applies a fixed value domain and stack identifiers", () => {
    const option = optionForChart({
      ...baseSpec,
      kind: "area",
      value_domain: { min: 0, max: 100 },
      series: [
        { ...baseSpec.series[0], stack_id: "receiver-classes" },
        {
          id: "comparison",
          label: "Comparison",
          stack_id: "receiver-classes",
          points: [{ category: "1979 Q1", value: 20 }],
        },
      ],
    }) as {
      yAxis: { min: number; max: number };
      series: Array<{ stack?: string }>;
    };

    expect(option.yAxis).toMatchObject({ min: 0, max: 100 });
    expect(option.series.map((series) => series.stack)).toEqual([
      "receiver-classes",
      "receiver-classes",
    ]);
  });

  it("keeps negative values and a visible zero reference", () => {
    const option = optionForChart({
      ...baseSpec,
      kind: "bar",
      orientation: "horizontal",
      value_domain: { min: -5, max: 5 },
      reference_lines: [
        { id: "zero", label: "No expected effect", axis: "value", value: 0 },
      ],
      series: [
        {
          id: "effects",
          label: "Expected effect",
          points: [
            { category: "Loyalty", value: 2.4 },
            { category: "Religion sympathy", value: -1.8 },
          ],
        },
      ],
    }) as {
      xAxis: { min: number; max: number };
      series: Array<{
        data: Array<number | null>;
        markLine?: { data: Array<{ xAxis?: number }> };
      }>;
    };

    expect(option.xAxis).toMatchObject({ min: -5, max: 5 });
    expect(option.series[0].data).toEqual([2.4, -1.8]);
    expect(option.series[0].markLine?.data[0]).toMatchObject({ xAxis: 0 });
  });

  it("inherits chart provenance unless a series supplies its own", () => {
    const inherited = provenanceForSeries(baseSpec, baseSpec.series[0]);
    const overridden = provenanceForSeries(baseSpec, {
      ...baseSpec.series[0],
      provenance: {
        kind: "extension_calculation",
        source: "Synthetic Analysis Pack mirror",
        observed_at: "2004-08-17",
        coverage: "experimental",
      },
    });

    expect(inherited).toBe(baseSpec.provenance);
    expect(overridden.kind).toBe("extension_calculation");
  });
});
