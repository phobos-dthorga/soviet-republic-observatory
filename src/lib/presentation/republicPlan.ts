import type { ChartSpec } from "../charts/types";
import type { Translator } from "../i18n/runtime";
import type { PlanTargetEvaluation } from "../observations/types";
import { briefMetricLabel } from "./republicBrief";

export function createPlanTargetChart(
  target: PlanTargetEvaluation,
  planName: string,
  translate: Translator,
): ChartSpec {
  const points = target.points;
  const observedAt = points.at(-1);
  return {
    schema_version: 1,
    id: `core.plan.target.${target.target.metric_id}`,
    title: translate("plan-chart-title", {
      metric: briefMetricLabel(target.target.metric_id, translate),
    }),
    description: translate("plan-chart-description"),
    kind: "line",
    category_axis_scale: "game_day",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("plan-axis-recorded-count"),
    unit: translate("unit-citizens"),
    series: [
      {
        id: "observed",
        label: translate("plan-series-observed"),
        points: points.map((point) => ({
          category: translate("observation-game-date-compact", {
            year: point.year,
            day: String(point.day).padStart(3, "0"),
          }),
          category_value: point.game_day,
          value: point.observed_value,
        })),
        provenance: {
          kind: "save_fact",
          source: translate("plan-source-observations"),
          observed_at: observedAt
            ? translate("observation-game-date-compact", {
                year: observedAt.year,
                day: String(observedAt.day).padStart(3, "0"),
              })
            : translate("chart-unavailable"),
          coverage: "complete",
        },
      },
      {
        id: "scheduled",
        label: translate("plan-series-scheduled"),
        style: "dashed",
        points: points.map((point) => ({
          category: translate("observation-game-date-compact", {
            year: point.year,
            day: String(point.day).padStart(3, "0"),
          }),
          category_value: point.game_day,
          value: point.scheduled_value,
        })),
        provenance: {
          kind: "player_definition",
          source: translate("plan-source-player-plan", { plan: planName }),
          observed_at: translate("plan-source-not-observation"),
          coverage: "complete",
        },
      },
    ],
    provenance: {
      kind: "calculation",
      source: translate("plan-source-comparison"),
      observed_at: observedAt
        ? translate("observation-game-date-compact", {
            year: observedAt.year,
            day: String(observedAt.day).padStart(3, "0"),
          })
        : translate("chart-unavailable"),
      coverage: points.length ? "complete" : "partial",
    },
  };
}
