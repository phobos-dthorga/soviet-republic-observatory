import type { ChartSpec, EvidenceCoverage, Provenance } from "../charts/types";
import type { Translator } from "../i18n/runtime";

export type AnalysisPackInspection = {
  valid: boolean;
  code: string | null;
  pack_id: string | null;
  name: string | null;
  author: string | null;
  version: string | null;
  host_api_version: number | null;
  default_locale: string | null;
  description: string | null;
  content_hash: string | null;
  consumed_metrics: string[];
  derived_metrics: string[];
  charts: string[];
};

export type AnalysisPackSummary = {
  pack_id: string;
  display_name: string;
  author: string;
  default_locale: string;
  description: string;
  active_revision: number | null;
  latest_revision: number;
  revision_count: number;
  semantic_version: string;
  host_api_version: number;
  content_hash: string;
  derived_metric_count: number;
  chart_count: number;
  enabled: boolean;
  validation_state: string;
};

export type ResolvedAnalysisPoint = {
  year: number;
  day: number;
  game_day: number;
  value: number;
  gap_before: boolean;
};

export type ResolvedAnalysisChart = {
  schema_version: 1;
  id: string;
  title: string;
  description: string;
  kind: "line" | "area" | "bar";
  orientation?: "vertical" | "horizontal";
  category_axis_label?: string;
  value_axis_label?: string;
  unit?: string;
  value_domain?: { min: number; max: number };
  series: Array<{
    id: string;
    label: string;
    published_metric_id: string | null;
    style?: "solid" | "dashed";
    stack_id?: string;
    points: ResolvedAnalysisPoint[];
    provenance: Provenance;
  }>;
  provenance: Provenance;
};

export type AnalysisPackContribution = {
  pack_id: string;
  version: string;
  content_hash: string;
  default_locale: string;
  charts: ResolvedAnalysisChart[];
};

function safeCoverage(value: string): EvidenceCoverage {
  return value === "complete" || value === "partial" ? value : "experimental";
}

function safeProvenance(provenance: Provenance): Provenance {
  return {
    ...provenance,
    kind: "extension_calculation",
    coverage: safeCoverage(provenance.coverage),
  };
}

export function chartSpecForAnalysisContribution(
  contribution: AnalysisPackContribution,
  chart: ResolvedAnalysisChart,
  t: Translator,
): ChartSpec {
  return {
    schema_version: 1,
    id: `${chart.id.slice(0, 48)}-${contribution.content_hash.slice(0, 12)}`,
    title: chart.title,
    description: chart.description,
    kind: chart.kind,
    orientation: chart.orientation,
    category_axis_scale: "game_day",
    category_axis_label: chart.category_axis_label,
    value_axis_label: chart.value_axis_label,
    unit: chart.unit,
    value_domain: chart.value_domain,
    series: chart.series.map((series) => ({
      id: series.id,
      label: series.label,
      style: series.style,
      stack_id: series.stack_id,
      provenance: safeProvenance(series.provenance),
      points: series.points.map((point) => ({
        category: t("observation-game-date-compact", {
          year: point.year,
          day: String(point.day).padStart(3, "0"),
        }),
        category_value: point.game_day,
        value: point.value,
        gap_before: point.gap_before,
      })),
    })),
    provenance: safeProvenance(chart.provenance),
  };
}
