import type { ChartSpec, Provenance } from "../charts/types";
import type { Translator } from "../i18n/runtime";
import { formatNumber } from "../i18n/format";
import type { ContextHelpContent } from "../ui/types";
import type {
  CarbonEstimateModel,
  EnvironmentActivityChannel,
  EnvironmentWorkspaceModel,
} from "../observations/types";

const saveEvidence: Provenance = {
  kind: "save_fact",
  source: "stats.ini environmental activity fields",
  observed_at: "selected recorded save",
  coverage: "partial",
};

export function environmentActivityChart(
  workspace: EnvironmentWorkspaceModel | null,
  channel: EnvironmentActivityChannel,
  resourceToken: string,
  translate: Translator,
): ChartSpec {
  const points = (workspace?.activity ?? [])
    .filter(
      (point) =>
        point.activity_channel === channel &&
        point.resource_token === resourceToken,
    )
    .map((point) => ({
      category: `${point.year} · ${String(point.day).padStart(3, "0")}`,
      category_value: point.game_day,
      value: point.primary_value,
    }));
  return {
    schema_version: 1,
    id: `environment-${channel}-${resourceToken}`,
    title: translate("environment-activity-chart-title", {
      resource: resourceToken || translate("environment-resource-none"),
    }),
    description: translate("environment-activity-chart-description"),
    kind: "line",
    category_axis_scale: "game_day",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("environment-axis-recorded-value"),
    series: points.length
      ? [
          {
            id: "primary",
            label: translate("environment-primary-source-value"),
            points,
            provenance: saveEvidence,
          },
        ]
      : [],
    provenance: saveEvidence,
  };
}

export function environmentChannelLabel(
  channel: EnvironmentActivityChannel,
  translate: Translator,
): string {
  const keys = {
    production: "environment-channel-production",
    construction_use: "environment-channel-construction",
    factory_use: "environment-channel-factory-use",
    shop_use: "environment-channel-shop-use",
    vehicle_use: "environment-channel-vehicle-use",
    factory_waste: "environment-channel-factory-waste",
    citizen_waste: "environment-channel-citizen-waste",
    demolition_waste: "environment-channel-demolition-waste",
  } as const;
  return translate(keys[channel]);
}

export function carbonContributorsChart(
  estimate: CarbonEstimateModel | null,
  translate: Translator,
): ChartSpec {
  const provenance: Provenance = {
    kind: "estimate",
    source: translate("environment-carbon-source"),
    observed_at: translate("environment-carbon-selected-save"),
    coverage: "partial",
  };
  return {
    schema_version: 1,
    id: "environment-carbon-contributors",
    title: translate("environment-carbon-contributors-title"),
    description: translate("environment-carbon-contributors-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_scale: "ordinal",
    category_axis_label: translate("environment-column-resource"),
    value_axis_label: translate("environment-carbon-axis"),
    unit: "g CO₂e",
    series: estimate?.available
      ? [
          {
            id: "estimated-co2e",
            label: translate("environment-carbon-result"),
            points: estimate.contributions.slice(0, 25).map((contribution) => ({
              category: `${contribution.resource_token} · ${environmentChannelLabel(
                contribution.activity_channel,
                translate,
              )}`,
              value: contribution.estimated_grams_co2e,
            })),
            provenance,
          },
        ]
      : [],
    provenance,
  };
}

export function environmentActivityHelp(
  workspace: EnvironmentWorkspaceModel | null,
  channel: EnvironmentActivityChannel,
  resourceToken: string | null,
  translate: Translator,
): ContextHelpContent {
  const rows = (workspace?.activity ?? []).filter(
    (point) =>
      point.activity_channel === channel &&
      (!resourceToken || point.resource_token === resourceToken),
  );
  const sourceFields = Array.from(
    new Set(rows.map((point) => point.source_field)),
  ).sort();
  const safeResourceTopic = (resourceToken ?? "summary")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .slice(0, 80);
  return {
    topic: `environment-${channel}-${safeResourceTopic}`,
    title: translate("environment-context-title"),
    text: translate("environment-context-description"),
    details: [
      {
        label: translate("metric-context-formula"),
        value: channel.endsWith("_waste")
          ? translate("environment-context-formula-waste")
          : resourceToken
            ? translate("environment-context-formula-resource")
            : translate("environment-context-formula-summary"),
      },
      {
        label: translate("metric-context-unit"),
        value: translate("environment-context-unit"),
      },
      {
        label: translate("metric-context-time-basis"),
        value: resourceToken
          ? translate("environment-context-time-history")
          : translate("environment-context-time-selected"),
      },
      {
        label: translate("markets-context-currency"),
        value: translate("environment-context-currency"),
      },
      {
        label: translate("metric-context-evidence"),
        value: translate("evidence-save-fact"),
      },
      {
        label: translate("metric-context-source-fields"),
        value:
          sourceFields.join(", ") || translate("environment-context-no-source"),
      },
      {
        label: translate("metric-context-profile"),
        value: `${workspace?.analysis_context.compatibility_profile_id ?? "—"} · ${workspace?.analysis_context.compatibility_profile_hash?.slice(0, 12) ?? "—"}`,
      },
      {
        label: translate("metric-context-analytical-head"),
        value:
          workspace?.analysis_context.head_interpretation_id?.slice(0, 12) ??
          "—",
      },
      {
        label: translate("metric-context-exclusions"),
        value: translate("environment-context-exclusions"),
      },
    ],
  };
}

export function carbonEstimateHelp(
  workspace: EnvironmentWorkspaceModel | null,
  translate: Translator,
): ContextHelpContent {
  return {
    topic: "environment-carbon-estimate",
    title: translate("environment-carbon-context-title"),
    text: translate("environment-carbon-context-description"),
    details: [
      {
        label: translate("metric-context-formula"),
        value: translate("environment-carbon-context-formula"),
      },
      {
        label: translate("metric-context-unit"),
        value: translate("environment-carbon-context-unit"),
      },
      {
        label: translate("metric-context-time-basis"),
        value: translate("environment-context-time-selected"),
      },
      {
        label: translate("markets-context-currency"),
        value: translate("environment-context-currency"),
      },
      {
        label: translate("metric-context-evidence"),
        value: translate("evidence-player-definition"),
      },
      {
        label: translate("metric-context-profile"),
        value: `${workspace?.analysis_context.compatibility_profile_id ?? "—"} · ${workspace?.analysis_context.compatibility_profile_hash?.slice(0, 12) ?? "—"}`,
      },
      {
        label: translate("metric-context-analytical-head"),
        value:
          workspace?.analysis_context.head_interpretation_id?.slice(0, 12) ??
          "—",
      },
      {
        label: translate("metric-context-exclusions"),
        value: translate("environment-carbon-context-exclusions"),
      },
    ],
  };
}

export function formatCo2e(grams: number, locale: string): string {
  if (Math.abs(grams) >= 1_000_000) {
    return `${formatNumber(grams / 1_000_000, locale, { maximumFractionDigits: 2 })} t CO₂e`;
  }
  if (Math.abs(grams) >= 1_000) {
    return `${formatNumber(grams / 1_000, locale, { maximumFractionDigits: 2 })} kg CO₂e`;
  }
  return `${formatNumber(grams, locale, { maximumFractionDigits: 2 })} g CO₂e`;
}
