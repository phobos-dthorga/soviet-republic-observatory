import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import { reviewRepublicPlanWorkspace } from "../ui-review/fixtures";
import { createPlanTargetChart } from "./republicPlan";

const translate = ((key: string, arguments_?: Record<string, unknown>) =>
  arguments_ ? `${key}:${JSON.stringify(arguments_)}` : key) as Translator;

describe("Republic Plan presentation", () => {
  it("keeps observed facts distinct from player-defined schedules", () => {
    const plan = reviewRepublicPlanWorkspace().active_plan!;
    const chart = createPlanTargetChart(
      plan.targets[0],
      plan.revision.name,
      translate,
    );

    expect(chart.series[0].provenance?.kind).toBe("save_fact");
    expect(chart.series[1].provenance?.kind).toBe("player_definition");
    expect(chart.provenance.kind).toBe("calculation");
    expect(chart.series[0].points.map((point) => point.value)).toEqual([
      55_000, 58_137,
    ]);
    expect(chart.series[1].points.map((point) => point.value)).toEqual([
      55_000, 58_400,
    ]);
  });

  it("does not synthesize observations for an empty target series", () => {
    const plan = reviewRepublicPlanWorkspace().active_plan!;
    const chart = createPlanTargetChart(
      { ...plan.targets[0], points: [] },
      plan.revision.name,
      translate,
    );

    expect(chart.series.every((series) => series.points.length === 0)).toBe(
      true,
    );
    expect(chart.provenance.coverage).toBe("partial");
  });
});
