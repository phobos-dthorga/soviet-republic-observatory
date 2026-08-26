import { describe, expect, it } from "vitest";
import {
  expandedCategories,
  expandedValues,
  optionForChart,
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
});
