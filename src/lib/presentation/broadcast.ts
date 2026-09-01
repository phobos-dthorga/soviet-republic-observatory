import type { ChartPoint, ChartSpec } from "../charts/types";
import type { TranslationKey } from "../i18n/catalog";
import type { Translator } from "../i18n/runtime";
import type {
  BroadcastOutcomeAvailability,
  BroadcastOutcomeModel,
} from "../observations/types";

const receiverLabelKeys: Record<string, TranslationKey> = {
  "core.citizens.electronics.none": "receiver-none",
  "core.citizens.electronics.radio": "receiver-radio",
  "core.citizens.electronics.television": "receiver-television",
  "core.citizens.electronics.computer": "receiver-computer",
};

const statusLabelKeys: Record<string, TranslationKey> = {
  "core.citizens.status.happiness": "broadcast-status-happiness",
  "core.citizens.status.food_satisfaction":
    "broadcast-status-food-satisfaction",
  "core.citizens.status.health": "broadcast-status-health",
  "core.citizens.status.government_loyalty":
    "broadcast-status-government-loyalty",
  "core.citizens.status.alcohol_addiction":
    "broadcast-status-alcohol-addiction",
  "core.citizens.status.culture_enjoyment":
    "broadcast-status-culture-enjoyment",
  "core.citizens.status.sports_enjoyment": "broadcast-status-sports-enjoyment",
  "core.citizens.status.religion_sympathy":
    "broadcast-status-religion-sympathy",
  "core.citizens.status.clothing_quality": "broadcast-status-clothing-quality",
};

const availabilityKeys: Record<BroadcastOutcomeAvailability, TranslationKey> = {
  available: "broadcast-outcome-available",
  receiver_unavailable: "broadcast-outcome-receiver-unavailable",
  status_unavailable: "broadcast-outcome-status-unavailable",
  insufficient_pairs: "broadcast-outcome-insufficient",
  constant_receiver_changes: "broadcast-outcome-constant-receiver",
  constant_status_changes: "broadcast-outcome-constant-status",
};

export function broadcastMetricLabel(
  metricId: string,
  translate: Translator,
): string {
  const key = receiverLabelKeys[metricId] ?? statusLabelKeys[metricId];
  return key ? translate(key) : metricId;
}

export function broadcastOutcomeAvailabilityLabel(
  availability: BroadcastOutcomeAvailability,
  translate: Translator,
): string {
  return translate(availabilityKeys[availability]);
}

export function createBroadcastOutcomeChart(
  outcome: BroadcastOutcomeModel,
  translate: Translator,
): ChartSpec {
  const receiver = broadcastMetricLabel(outcome.receiver_metric_id, translate);
  const status = broadcastMetricLabel(outcome.status_metric_id, translate);
  const point = (
    index: number,
    value: (pair: BroadcastOutcomeModel["pairs"][number]) => number,
  ): ChartPoint => {
    const pair = outcome.pairs[index];
    const previous = outcome.pairs[index - 1];
    return {
      category: translate("observation-game-date-compact", {
        year: pair.status_year,
        day: String(pair.status_day).padStart(3, "0"),
      }),
      category_value: pair.status_game_day,
      value: value(pair),
      gap_before:
        previous !== undefined &&
        pair.status_record_id !== previous.status_record_id + 1,
    };
  };
  return {
    schema_version: 1,
    id: "broadcast-outcome-comparison",
    title: translate("broadcast-outcome-chart-title"),
    description: translate("broadcast-outcome-chart-description"),
    kind: "line",
    category_axis_scale: "game_day",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("broadcast-outcome-chart-axis"),
    unit: "pp",
    series: [
      {
        id: "receiver-change",
        label: translate("broadcast-outcome-chart-receiver-series", {
          receiver,
        }),
        points: outcome.pairs.map((_pair, index) =>
          point(index, (pair) => pair.receiver_share_change),
        ),
      },
      {
        id: "status-change",
        label: translate("broadcast-outcome-chart-status-series", { status }),
        points: outcome.pairs.map((_pair, index) =>
          point(index, (pair) => pair.status_change * 100),
        ),
      },
    ],
    provenance: {
      kind: "calculation",
      source: translate("broadcast-outcome-chart-source"),
      observed_at:
        outcome.end_year === null || outcome.end_day === null
          ? translate("chart-unavailable")
          : translate("observation-game-date-compact", {
              year: outcome.end_year,
              day: String(outcome.end_day).padStart(3, "0"),
            }),
      coverage: "complete",
    },
  };
}
