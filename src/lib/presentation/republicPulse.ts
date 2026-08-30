import type { ChartSpec } from "../charts/types";
import { formatDate } from "../i18n/format";
import type { Translator } from "../i18n/runtime";
import type {
  ArchiveComparison,
  ArchiveObservation,
  ArchiveOverview,
} from "../observations/types";

const RECEIVER_LABELS = {
  "core.citizens.electronics.none": "receiver-none",
  "core.citizens.electronics.radio": "receiver-radio",
  "core.citizens.electronics.television": "receiver-television",
  "core.citizens.electronics.computer": "receiver-computer",
} as const;

function gameDay(observation: ArchiveObservation): number | null {
  if (observation.latest_year === null || observation.latest_day === null)
    return null;
  return observation.latest_year * 365 + observation.latest_day;
}

function gameDate(
  observation: ArchiveObservation,
  translate: Translator,
): string {
  return translate("observation-game-date-compact", {
    year: observation.latest_year ?? "—",
    day:
      observation.latest_day === null
        ? "—"
        : String(observation.latest_day).padStart(3, "0"),
  });
}

export function selectedBranchObservations(
  archive: ArchiveOverview | null,
): ArchiveObservation[] {
  if (!archive || archive.selected_branch_id === "unassigned") return [];
  return archive.observations
    .filter((observation) => observation.included_in_context)
    .sort(
      (left, right) =>
        (left.context_sequence ?? Number.MAX_SAFE_INTEGER) -
        (right.context_sequence ?? Number.MAX_SAFE_INTEGER),
    );
}

export function createCadenceChart(
  archive: ArchiveOverview | null,
  translate: Translator,
  locale: string,
): ChartSpec {
  const observations = selectedBranchObservations(archive);
  const points = observations.slice(1).flatMap((observation, index) => {
    const currentDay = gameDay(observation);
    const previousDay = gameDay(observations[index]);
    if (currentDay === null || previousDay === null) return [];
    return [
      {
        category: gameDate(observation, translate),
        value: Math.max(0, currentDay - previousDay),
      },
    ];
  });
  const latest = observations.at(-1);
  return {
    schema_version: 1,
    id: "core.monitoring.observation_cadence",
    title: translate("monitor-chart-cadence-title"),
    description: translate("monitor-chart-cadence-description"),
    kind: "bar",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("monitor-axis-elapsed-game-days"),
    unit: translate("unit-game-days"),
    series: points.length
      ? [
          {
            id: "elapsed-game-days",
            label: translate("monitor-series-elapsed-game-days"),
            points,
          },
        ]
      : [],
    provenance: {
      kind: "calculation",
      source: latest?.source_file_name ?? translate("monitor-source-no-save"),
      observed_at: latest
        ? formatDate(latest.imported_at_ms, locale, {
            dateStyle: "medium",
            timeStyle: "short",
          })
        : translate("chart-unavailable"),
      coverage: latest?.coverage_status ?? "partial",
    },
  };
}

export function createReceiverChangeChart(
  comparison: ArchiveComparison | null,
  translate: Translator,
): ChartSpec {
  return {
    schema_version: 1,
    id: "core.monitoring.receiver_change",
    title: translate("monitor-chart-receiver-change-title"),
    description: translate("monitor-chart-receiver-change-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("receiver-class"),
    value_axis_label: translate("monitor-axis-citizen-change"),
    unit: translate("unit-citizens"),
    reference_lines: comparison
      ? [
          {
            id: "no-change",
            label: translate("monitor-reference-no-change"),
            axis: "value",
            value: 0,
          },
        ]
      : undefined,
    series: comparison
      ? [
          {
            id: "receiver-change",
            label: translate("monitor-series-citizen-change"),
            points: comparison.receiver_changes.map((change) => ({
              category: translate(
                RECEIVER_LABELS[
                  change.metric_id as keyof typeof RECEIVER_LABELS
                ] ?? "receiver-class",
              ),
              value: change.delta,
            })),
          },
        ]
      : [],
    provenance: {
      kind: "calculation",
      source: comparison
        ? translate("monitor-source-comparison", {
            from: comparison.from.source_file_name,
            to: comparison.to.source_file_name,
          })
        : translate("monitor-source-no-comparison"),
      observed_at: comparison
        ? translate("observation-game-date-compact", {
            year: comparison.to.year,
            day: String(comparison.to.day).padStart(3, "0"),
          })
        : translate("chart-unavailable"),
      coverage: comparison?.to.coverage_status ?? "partial",
    },
  };
}

export function largestObservationInterval(
  archive: ArchiveOverview | null,
): number | null {
  const observations = selectedBranchObservations(archive);
  let largest: number | null = null;
  for (let index = 1; index < observations.length; index += 1) {
    const currentDay = gameDay(observations[index]);
    const previousDay = gameDay(observations[index - 1]);
    if (currentDay === null || previousDay === null) continue;
    const interval = Math.max(0, currentDay - previousDay);
    largest = largest === null ? interval : Math.max(largest, interval);
  }
  return largest;
}
