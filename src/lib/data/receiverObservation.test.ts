import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import type { ReceiverDataset } from "../observations/types";
import { createObservedReceiverChart } from "./receiverObservation";

const t: Translator = (key, arguments_) =>
  `${key}${arguments_ ? `:${JSON.stringify(arguments_)}` : ""}`;

const dataset: ReceiverDataset = {
  payload_hash: "a".repeat(64),
  source_file_name: "synthetic.zip",
  source_file_size: 100,
  source_modified_ms: 1,
  imported_at_ms: 2,
  parser_version: "test-parser",
  format_profile: "test-profile",
  branch_id: "unassigned",
  geographic_scope: "republic",
  coverage: {
    status: "complete",
    history_records: 3,
    chartable_records: 3,
    dropped_records: 0,
    warnings: [],
  },
  source_fields: [],
  points: [
    {
      record_id: 0,
      year: 1980,
      day: 1,
      game_day: 1980 * 365 + 1,
      none: 70,
      radio: 20,
      television: 8,
      computer: 2,
      classified_total: 100,
    },
    {
      record_id: 1,
      year: 1980,
      day: 6,
      game_day: 1980 * 365 + 6,
      none: 60,
      radio: 25,
      television: 10,
      computer: 5,
      classified_total: 100,
    },
    {
      record_id: 2,
      year: 1980,
      day: 30,
      game_day: 1980 * 365 + 30,
      none: 50,
      radio: 30,
      television: 12,
      computer: 8,
      classified_total: 100,
    },
  ],
};

describe("observed receiver chart", () => {
  it("uses the four stable classes, actual game-day positions, and save provenance", () => {
    const chart = createObservedReceiverChart(dataset, t);

    expect(chart.category_axis_scale).toBe("game_day");
    expect(chart.provenance).toMatchObject({
      kind: "save_fact",
      coverage: "complete",
    });
    expect(chart.series.map((series) => series.id)).toEqual([
      "none",
      "radio",
      "television",
      "computer",
    ]);
    expect(chart.series[0].points.map((point) => point.category_value)).toEqual(
      dataset.points.map((point) => point.game_day),
    );
    expect(chart.series[0].points[2].gap_before).toBe(true);
    expect(
      chart.series.reduce((total, series) => total + series.points[0].value, 0),
    ).toBeCloseTo(100);
  });
});
