import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import type { EnvironmentWorkspaceModel } from "../observations/types";
import {
  carbonContributorsChart,
  environmentActivityChart,
  environmentChannelLabel,
  formatCo2e,
} from "./environment";

const translate = ((key: string, arguments_?: Record<string, unknown>) =>
  arguments_ ? `${key}:${JSON.stringify(arguments_)}` : key) as Translator;

const workspace = {
  activity: [
    {
      record_id: 1,
      year: 2010,
      day: 20,
      game_day: 733_670,
      resource_token: "chemicals",
      activity_channel: "production",
      primary_value: 12,
      secondary_value: 0,
      source_field: "$Resources_Produced",
      source_line: 8,
      row_ordinal: 0,
      quantity_is_publishable: true,
      exact_observation: null,
    },
    {
      record_id: 2,
      year: 2012,
      day: 30,
      game_day: 734_410,
      resource_token: "chemicals",
      activity_channel: "production",
      primary_value: 18,
      secondary_value: 0,
      source_field: "$Resources_Produced",
      source_line: 9,
      row_ordinal: 0,
      quantity_is_publishable: true,
      exact_observation: null,
    },
  ],
} as EnvironmentWorkspaceModel;

describe("environment presentation", () => {
  it("keeps real game dates on the shared date-axis contract", () => {
    const chart = environmentActivityChart(
      workspace,
      "production",
      "chemicals",
      translate,
    );
    expect(chart.category_axis_scale).toBe("game_day");
    expect(chart.series[0]?.points).toEqual([
      { category: "2010 · 020", category_value: 733_670, value: 12 },
      { category: "2012 · 030", category_value: 734_410, value: 18 },
    ]);
  });

  it("keeps waste channels named as waste rather than carbon", () => {
    expect(environmentChannelLabel("factory_waste", translate)).toBe(
      "environment-channel-factory-waste",
    );
  });

  it("uses grams, kilograms, and tonnes according to magnitude", () => {
    expect(formatCo2e(25, "en-AU")).toBe("25 g CO₂e");
    expect(formatCo2e(2_500, "en-AU")).toBe("2.5 kg CO₂e");
    expect(formatCo2e(2_500_000, "en-AU")).toBe("2.5 t CO₂e");
  });

  it("keeps carbon contributors separated by exact resource and activity", () => {
    const chart = carbonContributorsChart(
      {
        available: true,
        factor_set_id: "carbon-test",
        factor_set_revision: 1,
        estimated_grams_co2e: 250,
        covered_rows: 1,
        eligible_rows: 2,
        coverage_percent: 50,
        missing_factors: ["factory_use:chemicals"],
        contributions: [
          {
            resource_token: "chemicals",
            activity_channel: "production",
            recorded_quantity: 5,
            grams_co2e_per_unit: 50,
            estimated_grams_co2e: 250,
          },
        ],
        limitation: null,
      },
      translate,
    );
    expect(chart.kind).toBe("bar");
    expect(chart.orientation).toBe("horizontal");
    expect(chart.series[0]?.points).toEqual([
      {
        category: "chemicals · environment-channel-production",
        value: 250,
      },
    ]);
  });
});
