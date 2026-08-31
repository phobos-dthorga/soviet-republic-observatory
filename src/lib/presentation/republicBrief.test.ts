import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import type { RepublicBrief } from "../observations/types";
import { reviewRepublicBrief } from "../ui-review/fixtures";
import {
  createBriefChangeChart,
  createBriefEducationChart,
  createBriefReceiverChart,
} from "./republicBrief";

const translate = ((key: string, arguments_?: Record<string, unknown>) =>
  arguments_ ? `${key}:${JSON.stringify(arguments_)}` : key) as Translator;

describe("Republic Brief presentation", () => {
  it("uses only host-provided exact-head metrics and deltas", () => {
    const brief = reviewRepublicBrief();
    const chart = createBriefChangeChart(brief, translate);
    expect(chart.provenance.kind).toBe("calculation");
    expect(chart.provenance.source).toContain("UI-REVIEW-PG7.zip");
    expect(chart.series[0].points.map((point) => point.value)).toEqual([
      233, 32, 223, 255,
    ]);
    expect(chart.reference_lines?.[0].value).toBe(0);
  });

  it("does not invent categories when source metrics are unavailable", () => {
    const brief: RepublicBrief = {
      ...reviewRepublicBrief(),
      metrics: [],
    };
    expect(createBriefChangeChart(brief, translate).series).toEqual([]);
    expect(createBriefEducationChart(brief, translate).series).toEqual([]);
    expect(createBriefReceiverChart(brief, translate).series).toEqual([]);
  });

  it("converts only host-calculated receiver basis points into percentages", () => {
    const chart = createBriefReceiverChart(reviewRepublicBrief(), translate);
    expect(chart.value_domain).toEqual({ min: 0, max: 100 });
    expect(chart.series[0].points.map((point) => point.value)).toEqual([
      46.65, 36.06, 13.8, 3.49,
    ]);
    expect(chart.provenance.kind).toBe("calculation");
  });
});
