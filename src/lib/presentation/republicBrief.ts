import type { ChartSpec, Provenance } from "../charts/types";
import type { TranslationKey } from "../i18n/catalog";
import type { Translator } from "../i18n/runtime";
import type { BriefMetric, RepublicBrief } from "../observations/types";
import { populationFactLabel } from "./population";

const CLASSIFIED_TOTAL = "core.citizens.electronics.classified_total";

const RECEIVER_METRICS = [
  "core.citizens.electronics.none",
  "core.citizens.electronics.radio",
  "core.citizens.electronics.television",
  "core.citizens.electronics.computer",
] as const;

const EDUCATION_METRICS = [
  "source.stats.citizens.no_education",
  "source.stats.citizens.basic_education",
  "source.stats.citizens.higher_education",
] as const;

const BRIEF_LABELS = {
  [CLASSIFIED_TOTAL]: "briefing-metric-classified-receivers",
} as const satisfies Record<string, TranslationKey>;

export function briefMetric(
  brief: RepublicBrief,
  metricId: string,
): BriefMetric | null {
  return brief.metrics.find((metric) => metric.metric_id === metricId) ?? null;
}

export function briefMetricLabel(
  metricId: string,
  translate: Translator,
): string {
  const briefKey = (BRIEF_LABELS as Partial<Record<string, TranslationKey>>)[
    metricId
  ];
  return briefKey
    ? translate(briefKey)
    : populationFactLabel(metricId, translate);
}

function briefProvenance(
  brief: RepublicBrief,
  translate: Translator,
  kind: "save_fact" | "calculation",
  includeComparison = false,
): Provenance {
  const observation = brief.observation;
  const comparison = brief.comparison;
  return {
    kind,
    source:
      observation && includeComparison && comparison
        ? translate("briefing-source-comparison", {
            from: comparison.source_file_name,
            to: observation.source_file_name,
            profile: `${observation.profile_id}@${observation.profile_version}`,
          })
        : observation
          ? translate("briefing-source-snapshot", {
              file: observation.source_file_name,
              profile: `${observation.profile_id}@${observation.profile_version}`,
            })
          : translate("population-source-no-snapshot"),
    observed_at: observation
      ? translate("observation-game-date-compact", {
          year: observation.year,
          day: String(observation.day).padStart(3, "0"),
        })
      : translate("chart-unavailable"),
    coverage: observation?.coverage_status ?? "partial",
  };
}

export function createBriefChangeChart(
  brief: RepublicBrief,
  translate: Translator,
): ChartSpec {
  const provenance = briefProvenance(brief, translate, "calculation", true);
  const points = brief.metrics
    .filter((metric) => metric.role === "headline" && metric.delta !== null)
    .map((metric) => ({
      category: briefMetricLabel(metric.metric_id, translate),
      value: metric.delta ?? 0,
    }));
  return {
    schema_version: 1,
    id: "core.briefing.headline_change",
    title: translate("briefing-chart-change-title"),
    description: translate("briefing-chart-change-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("population-axis-source-category"),
    value_axis_label: translate("briefing-axis-recorded-change"),
    unit: translate("unit-citizens"),
    reference_lines: [
      {
        id: "no-change",
        label: translate("briefing-no-change"),
        axis: "value",
        value: 0,
      },
    ],
    series: points.length
      ? [
          {
            id: "headline-change",
            label: translate("briefing-series-change"),
            points,
          },
        ]
      : [],
    provenance,
  };
}

export function createBriefEducationChart(
  brief: RepublicBrief,
  translate: Translator,
): ChartSpec {
  const points = EDUCATION_METRICS.flatMap((metricId) => {
    const metric = briefMetric(brief, metricId);
    return metric
      ? [
          {
            category: briefMetricLabel(metricId, translate),
            value: metric.value,
          },
        ]
      : [];
  });
  return {
    schema_version: 1,
    id: "core.briefing.education_profile",
    title: translate("population-chart-education-title"),
    description: translate("briefing-chart-education-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("population-axis-source-category"),
    value_axis_label: translate("population-axis-recorded-citizens"),
    unit: translate("unit-citizens"),
    series: points.length
      ? [
          {
            id: "education-counts",
            label: translate("population-series-recorded-count"),
            points,
          },
        ]
      : [],
    provenance: briefProvenance(brief, translate, "save_fact"),
  };
}

export function createBriefReceiverChart(
  brief: RepublicBrief,
  translate: Translator,
): ChartSpec {
  const points = RECEIVER_METRICS.flatMap((metricId) => {
    const metric = briefMetric(brief, metricId);
    return metric?.share_basis_points == null
      ? []
      : [
          {
            category: briefMetricLabel(metricId, translate),
            value: metric.share_basis_points / 100,
          },
        ];
  });
  return {
    schema_version: 1,
    id: "core.briefing.receiver_composition",
    title: translate("briefing-chart-receiver-title"),
    description: translate("briefing-chart-receiver-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("receiver-class"),
    value_axis_label: translate("chart-axis-classified-share"),
    unit: "%",
    value_domain: { min: 0, max: 100 },
    series: points.length
      ? [
          {
            id: "receiver-share",
            label: translate("briefing-series-classified-share"),
            points,
          },
        ]
      : [],
    provenance: briefProvenance(brief, translate, "calculation"),
  };
}
