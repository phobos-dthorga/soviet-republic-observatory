import { describe, expect, it } from "vitest";
import { translate } from "../i18n/runtime";
import type { BroadcastOutcomeModel } from "../observations/types";
import {
  broadcastMetricLabel,
  broadcastOutcomeAvailabilityLabel,
  createBroadcastOutcomeChart,
} from "./broadcast";

describe("Broadcast presentation", () => {
  it("uses player-facing labels for stable metrics", () => {
    expect(broadcastMetricLabel("core.citizens.status.health", translate)).toBe(
      "Health",
    );
    expect(
      broadcastOutcomeAvailabilityLabel("insufficient_pairs", translate),
    ).toContain("12");
  });

  it("keeps missing confirmed records as chart gaps", () => {
    const outcome: BroadcastOutcomeModel = {
      availability: "available",
      receiver_metric_id: "core.citizens.electronics.radio",
      status_metric_id: "core.citizens.status.happiness",
      lag_confirmed_records: 0,
      coefficient: 0.5,
      pair_count: 2,
      start_year: 2015,
      start_day: 1,
      end_year: 2015,
      end_day: 30,
      elapsed_days_median: 0,
      elapsed_days_min: 0,
      elapsed_days_max: 0,
      pairs: [pair(1, 1), pair(3, 30)],
    };

    const chart = createBroadcastOutcomeChart(outcome, translate);

    expect(chart.series[0].points[1].gap_before).toBe(true);
    expect(chart.series[1].points[0].value).toBe(2);
    expect(chart.unit).toBe("pp");
  });
});

function pair(recordId: number, day: number) {
  return {
    receiver_record_id: recordId,
    receiver_year: 2015,
    receiver_day: day,
    receiver_game_day: 735_000 + day,
    status_record_id: recordId,
    status_year: 2015,
    status_day: day,
    status_game_day: 735_000 + day,
    elapsed_game_days: 0,
    receiver_share_change: recordId,
    status_change: 0.02,
    exact_observation: null,
  };
}
