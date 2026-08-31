import type { ChartSpec } from "../charts/types";
import type { TranslationKey } from "../i18n/catalog";
import type { Translator } from "../i18n/runtime";
import type {
  PlanDirection,
  PlanTargetEvaluation,
} from "../observations/types";
import { briefMetricLabel } from "./republicBrief";

const planErrorKeys = {
  invalid_republic_plan_name: "plan-error-name",
  invalid_republic_plan_end_date: "plan-error-end-date",
  invalid_republic_plan_target_count: "plan-error-target-count",
  invalid_republic_plan_unknown_metric: "plan-error-unknown-metric",
  invalid_republic_plan_duplicate_metric: "plan-error-duplicate-metric",
  invalid_republic_plan_guardrail: "plan-error-guardrail",
  invalid_republic_plan_direction_mismatch: "plan-error-direction-mismatch",
  invalid_republic_plan_window: "plan-error-window",
  invalid_republic_plan_metric_unavailable: "plan-error-metric-unavailable",
  unknown_republic_plan: "plan-error-unknown-plan",
  republic_plan_branch_mismatch: "plan-error-branch-mismatch",
} as const satisfies Record<string, TranslationKey>;

export function planDirectionForValues(
  baseline: number | null,
  target: number,
): PlanDirection | null {
  if (baseline === null || !Number.isFinite(target)) return null;
  if (target === baseline) return "maintain";
  return target > baseline ? "increase" : "decrease";
}

export function planErrorTranslationKey(error: unknown): TranslationKey {
  if (!error || typeof error !== "object" || !("code" in error)) {
    return "plan-error-save";
  }
  const code = String(error.code);
  return planErrorKeys[code as keyof typeof planErrorKeys] ?? "plan-error-save";
}

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
