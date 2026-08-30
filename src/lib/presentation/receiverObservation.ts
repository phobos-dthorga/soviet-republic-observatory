import type { ChartPoint, ChartSpec } from "../charts/types";
import type { Translator } from "../i18n/runtime";
import type {
  ReceiverDataset,
  ReceiverHistoryPoint,
} from "../observations/types";

const GAP_THRESHOLD_GAME_DAYS = 14;

function receiverShare(point: ReceiverHistoryPoint, value: number): number {
  return (value / point.classified_total) * 100;
}

function chartPoints(
  dataset: ReceiverDataset,
  t: Translator,
  value: (point: ReceiverHistoryPoint) => number,
): ChartPoint[] {
  const usable = dataset.points.filter((point) => point.classified_total > 0);
  return usable.map((point, index) => {
    const previous = usable[index - 1];
    const elapsed = previous ? point.game_day - previous.game_day : 0;
    return {
      category: t("observation-game-date-compact", {
        year: point.year,
        day: String(point.day).padStart(3, "0"),
      }),
      category_value: point.game_day,
      value: receiverShare(point, value(point)),
      gap_before:
        previous !== undefined &&
        (point.record_id !== previous.record_id + 1 ||
          elapsed < 0 ||
          elapsed > GAP_THRESHOLD_GAME_DAYS),
    };
  });
}

export function createObservedReceiverChart(
  dataset: ReceiverDataset,
  t: Translator,
): ChartSpec {
  const shortHash = dataset.payload_hash.slice(0, 12);
  const latest = dataset.points.at(-1);
  const provenance = {
    kind: "save_fact" as const,
    source: t("evidence-source-receiver-save", {
      file: dataset.source_file_name,
      hash: shortHash,
    }),
    observed_at: latest
      ? t("observation-game-date-compact", {
          year: latest.year,
          day: String(latest.day).padStart(3, "0"),
        })
      : t("chart-unavailable"),
    coverage: dataset.coverage.status,
  };

  return {
    schema_version: 1,
    id: "receiver-ladder-observed",
    title: t("chart-receiver-title"),
    description: t("evidence-chart-receiver-description"),
    kind: "area",
    category_axis_scale: "game_day",
    category_axis_label: t("chart-axis-game-date"),
    value_axis_label: t("chart-axis-classified-share"),
    unit: "%",
    value_domain: { min: 0, max: 100 },
    series: [
      {
        id: "none",
        label: t("receiver-none"),
        stack_id: "receiver_classes",
        points: chartPoints(dataset, t, (point) => point.none),
      },
      {
        id: "radio",
        label: t("receiver-radio"),
        stack_id: "receiver_classes",
        points: chartPoints(dataset, t, (point) => point.radio),
      },
      {
        id: "television",
        label: t("receiver-television"),
        stack_id: "receiver_classes",
        points: chartPoints(dataset, t, (point) => point.television),
      },
      {
        id: "computer",
        label: t("receiver-computer"),
        stack_id: "receiver_classes",
        points: chartPoints(dataset, t, (point) => point.computer),
      },
    ],
    provenance,
  };
}
