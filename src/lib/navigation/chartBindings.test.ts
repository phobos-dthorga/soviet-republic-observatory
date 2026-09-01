import { describe, expect, it } from "vitest";
import type { ChartSpec } from "../charts/types";
import { exactObservationChartBindings } from "./chartBindings";
import { defaultWorkspaceLocation } from "./relatedData";

const chart: ChartSpec = {
  schema_version: 1,
  id: "history",
  title: "History",
  description: "History",
  kind: "line",
  category_axis_scale: "game_day",
  series: [
    {
      id: "observed",
      label: "Observed",
      points: [
        { category: "2018 · 300", category_value: 736_870, value: 10 },
        { category: "2018 · 333", category_value: 736_903, value: 12 },
      ],
    },
  ],
  provenance: {
    kind: "save_fact",
    source: "Fixture",
    observed_at: "2018 · 333",
    coverage: "complete",
  },
};

describe("exact observation chart bindings", () => {
  it("links only an exact unambiguous saved point", () => {
    const bindings = exactObservationChartBindings(
      chart,
      [
        { game_day: 736_870, exact_observation: null },
        {
          game_day: 736_903,
          exact_observation: {
            interpretation_id: "exact-save",
            branch_id: "main",
            year: 2018,
            day: 333,
          },
        },
      ],
      defaultWorkspaceLocation("broadcast"),
    );

    expect(bindings).toHaveLength(1);
    expect(bindings[0]).toMatchObject({
      seriesId: "observed",
      pointIndex: 1,
      destinations: [
        {
          exactObservation: { interpretation_id: "exact-save" },
          location: {
            workspace: "broadcast",
            filters: { interpretationId: "exact-save" },
          },
        },
      ],
    });
  });

  it("does not guess when two save variants claim one plotted day", () => {
    const exact = (interpretation_id: string) => ({
      interpretation_id,
      branch_id: "main",
      year: 2018,
      day: 333,
    });
    expect(
      exactObservationChartBindings(
        chart,
        [
          { game_day: 736_903, exact_observation: exact("variant-a") },
          { game_day: 736_903, exact_observation: exact("variant-b") },
        ],
        defaultWorkspaceLocation("broadcast"),
      ),
    ).toEqual([]);
  });
});
