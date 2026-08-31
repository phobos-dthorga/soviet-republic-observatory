import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import { reviewRepublicPlanWorkspace } from "../ui-review/fixtures";
import {
  createPlanTargetChart,
  planDirectionForValues,
  planErrorTranslationKey,
} from "./republicPlan";

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

  it("derives one unambiguous direction from baseline and target", () => {
    expect(planDirectionForValues(59_592, 60_000)).toBe("increase");
    expect(planDirectionForValues(59_592, 59_592)).toBe("maintain");
    expect(planDirectionForValues(59_592, 50_000)).toBe("decrease");
    expect(planDirectionForValues(null, 60_000)).toBeNull();
  });

  it("turns native plan error codes into safe localized messages", () => {
    expect(
      planErrorTranslationKey({
        code: "invalid_republic_plan_direction_mismatch",
        diagnostic: "The republic plan is invalid: direction_mismatch",
      }),
    ).toBe("plan-error-direction-mismatch");
    expect(planErrorTranslationKey({ code: "future_error" })).toBe(
      "plan-error-save",
    );
  });
});
