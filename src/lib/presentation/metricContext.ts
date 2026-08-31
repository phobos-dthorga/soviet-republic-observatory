import type { TranslationKey } from "../i18n/catalog";
import type { Translator } from "../i18n/runtime";
import type {
  BriefMetric,
  MetricContext,
  MetricContextLimitation,
  MetricPopulationBasis,
} from "../observations/types";
import type { ContextHelpContent, ContextHelpDetail } from "../ui/types";

const populationKeys: Record<MetricPopulationBasis, TranslationKey> = {
  all_recorded_citizens: "metric-context-population-all-recorded",
  source_defined_adults: "metric-context-population-adults",
  source_defined_small_children: "metric-context-population-small-children",
  source_defined_unemployed: "metric-context-population-unemployed",
  classified_receiver_population: "metric-context-population-receivers",
};

const limitationKeys: Record<MetricContextLimitation, TranslationKey> = {
  not_employment_count: "metric-context-limitation-not-employment",
  not_workers_only: "metric-context-limitation-not-workers",
  source_age_boundary_unverified: "metric-context-limitation-age",
  source_window_unverified: "metric-context-limitation-window",
  excludes_unclassified_citizens: "metric-context-limitation-unclassified",
};

export type MetricLabelResolver = (metricId: string) => string;

export function metricPopulationLabel(
  context: MetricContext,
  translate: Translator,
): string {
  return translate(populationKeys[context.population_basis]);
}

export function metricContextSummary(
  context: MetricContext,
  translate: Translator,
): string {
  return translate("metric-context-visible-summary", {
    population: metricPopulationLabel(context, translate),
    geography: translate("metric-context-geography-republic"),
  });
}

export function metricContextDetails(
  context: MetricContext,
  translate: Translator,
  metricLabel: MetricLabelResolver,
): ContextHelpDetail[] {
  const denominator = context.denominator_metric_id
    ? metricLabel(context.denominator_metric_id)
    : translate("metric-context-denominator-none");
  return [
    {
      label: translate("metric-context-population-label"),
      value: metricPopulationLabel(context, translate),
    },
    {
      label: translate("metric-context-time-label"),
      value: translate("metric-context-time-exact"),
    },
    {
      label: translate("metric-context-geography-label"),
      value: translate("metric-context-geography-republic"),
    },
    {
      label: translate("metric-context-denominator-label"),
      value: denominator,
    },
    {
      label: translate("metric-context-comparison-label"),
      value: translate("metric-context-comparison-preceding"),
    },
    {
      label: translate("metric-context-limitations-label"),
      value: context.limitations
        .map((limitation) => translate(limitationKeys[limitation]))
        .join("; "),
    },
  ];
}

export function metricContextHelp(
  metric: BriefMetric,
  translate: Translator,
  metricLabel: MetricLabelResolver,
): ContextHelpContent {
  const label = metricLabel(metric.metric_id);
  return {
    topic: `metric-context-${metric.metric_id.replaceAll(".", "-")}`,
    title: translate("metric-context-help-title", { metric: label }),
    text: translate("metric-context-help-text"),
    details: metricContextDetails(metric.context, translate, metricLabel),
  };
}
