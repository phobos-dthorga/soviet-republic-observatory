import { describe, expect, it } from "vitest";
import {
  condensedSeriesSummary,
  expandedCategories,
  expandedGameDayValues,
  expandedValues,
  formatGameDayValue,
  gameDayDomain,
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
    expect(
      expandedCategories(baseSpec.series[0].points, "aucune observation")[1],
    ).toBe("1979 Q2 · aucune observation");
  });

  it("uses actual game-day positions instead of equally spaced categories", () => {
    const gameDaySpec: ChartSpec = {
      ...baseSpec,
      category_axis_scale: "game_day",
      series: [
        {
          ...baseSpec.series[0],
          points: [
            { category: "1980 · 363", category_value: 723063, value: 80 },
            {
              category: "1981 · 005",
              category_value: 723070,
              value: 84,
              gap_before: true,
            },
          ],
        },
      ],
    };
    const option = optionForChart(gameDaySpec) as {
      xAxis: {
        type: string;
        scale: boolean;
        min: number;
        max: number;
        axisLabel: { formatter: (value: number) => string };
      };
      series: Array<{ data: ReturnType<typeof expandedGameDayValues> }>;
    };

    expect(option.xAxis.type).toBe("value");
    expect(option.xAxis.scale).toBe(true);
    expect(option.xAxis.min).toBe(723056);
    expect(option.xAxis.max).toBe(723077);
    expect(option.xAxis.axisLabel.formatter(1981 * 365 + 5)).toBe("1981 · 005");
    expect(option.series[0].data).toEqual(
      expandedGameDayValues(gameDaySpec.series[0].points),
    );
    expect(formatGameDayValue(1980 * 365 + 364)).toBe("1980 · 364");
  });

  it("focuses a long market history on its recorded years instead of year zero", () => {
    const start = 1960 * 365;
    const end = 2018 * 365 + 256;
    const points = Array.from({ length: 2968 }, (_, index) => ({
      category: `Recorded save ${index + 1}`,
      category_value: start + ((end - start) * index) / 2967,
      value: 250 + index / 2,
    }));
    const spec: ChartSpec = {
      ...baseSpec,
      category_axis_scale: "game_day",
      series: [{ ...baseSpec.series[0], points }],
    };
    const option = optionForChart(spec) as {
      xAxis: { min: number; max: number; scale: boolean };
    };

    expect(option.xAxis.scale).toBe(true);
    expect(option.xAxis.min).toBe(start - 90);
    expect(option.xAxis.max).toBe(end + 90);
    expect(start / (option.xAxis.max - option.xAxis.min)).toBeGreaterThan(30);
  });

  it("gives a single dated save a readable local window", () => {
    const day = 2018 * 365 + 256;
    expect(
      gameDayDomain([
        {
          id: "single",
          label: "Single save",
          points: [{ category: "2018 · 256", category_value: day, value: 1 }],
        },
      ]),
    ).toEqual({ min: day - 30, max: day + 30 });
  });

  it("derives one shared date window across every series", () => {
    expect(
      gameDayDomain([
        {
          id: "early",
          label: "Early",
          points: [
            { category: "1960 · 000", category_value: 715400, value: 1 },
          ],
        },
        {
          id: "late",
          label: "Late",
          points: [
            { category: "2018 · 256", category_value: 736826, value: 2 },
          ],
        },
      ]),
    ).toEqual({ min: 715310, max: 736916 });
  });

  it("condenses long textual series without losing extrema or gaps", () => {
    const points = Array.from({ length: 30 }, (_, index) => ({
      category: `Day ${index}`,
      value: index === 12 ? -5 : index,
      gap_before: index === 18,
    }));

    expect(condensedSeriesSummary(points)).toMatchObject({
      count: 30,
      first: points[0],
      minimum: points[12],
      maximum: points[29],
      latest: points[29],
      gapCount: 1,
    });
    expect(condensedSeriesSummary(points.slice(0, 24))).toBeNull();
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

  it("formats tooltip values with the active locale", () => {
    const option = optionForChart(baseSpec, undefined, false, "de-DE") as {
      tooltip: { valueFormatter: (value: unknown) => string };
    };
    expect(option.tooltip.valueFormatter(1234.5)).toBe("1.234,5 %");
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
