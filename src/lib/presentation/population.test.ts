import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import type { PopulationDataset } from "../observations/types";
import {
  createCityMovementChart,
  createEducationProfileChart,
  createPopulationMovementChart,
  createPopulationStatusChart,
} from "./population";

const translate = ((key: string, arguments_?: Record<string, unknown>) =>
  arguments_ ? `${key}:${JSON.stringify(arguments_)}` : key) as Translator;

const dataset: PopulationDataset = {
  analysis_context: {
    context_id: "ctx-population",
    selected_branch_id: "main",
    head_interpretation_id: "observation-2",
    original_branch_id: "main",
    mode: "latest",
    origin: "automatic",
    is_tip: true,
    membership_revision: 2,
    compatibility_profile_id: "org.republic-observatory.wrsr-1.1.1.9",
    compatibility_profile_hash: "a".repeat(64),
    observation_watermark: "observation-2",
    catalogue_generation_id: null,
    overlay_revision: null,
  },
  observations: [
    {
      interpretation_id: "observation-1",
      source_file_name: "first.zip",
      membership_revision: 1,
      sampled_year: 2014,
      sampled_day: 8,
      sampled_game_day: 735_118,
      coverage_status: "complete",
      mapping_classification: "reviewed_mapping",
      profile_id: "org.republic-observatory.wrsr-1.1.1.9",
      profile_version: "1.0.0",
      resolved_profile_hash: "a".repeat(64),
      exact_observation: null,
      facts: [
        {
          fact_id: "source.stats.citizens.small_children",
          value: 100,
          source_field: "$Citizens_SmallChilds",
          source_line: 10,
        },
        {
          fact_id: "source.stats.citizens.born",
          value: 7,
          source_field: "$Citizens_Born",
          source_line: 11,
        },
      ],
    },
    {
      interpretation_id: "observation-2",
      source_file_name: "second.zip",
      membership_revision: 2,
      sampled_year: 2014,
      sampled_day: 18,
      sampled_game_day: 735_128,
      coverage_status: "complete",
      mapping_classification: "reviewed_mapping",
      profile_id: "org.republic-observatory.wrsr-1.1.1.9",
      profile_version: "1.0.0",
      resolved_profile_hash: "a".repeat(64),
      exact_observation: null,
      facts: [
        {
          fact_id: "source.stats.citizens.small_children",
          value: 110,
          source_field: "$Citizens_SmallChilds",
          source_line: 20,
        },
        {
          fact_id: "source.stats.citizens.born",
          value: 9,
          source_field: "$Citizens_Born",
          source_line: 21,
        },
        {
          fact_id: "source.stats.citizens.no_education",
          value: 30,
          source_field: "$Citizens_NoEducation",
          source_line: 22,
        },
        {
          fact_id: "source.stats.citizens.basic_education",
          value: 60,
          source_field: "$Citizens_BasicEducationNum",
          source_line: 23,
        },
        {
          fact_id: "source.stats.citizens.higher_education",
          value: 20,
          source_field: "$Citizens_HighEducationNum",
          source_line: 24,
        },
      ],
    },
  ],
  cities: [
    {
      scope_id: "17",
      sampled_year: 2014,
      sampled_day: 18,
      sampled_game_day: 735_128,
      coverage_status: "complete",
      facts: [
        {
          fact_id: "source.stats.citizens.born",
          value: 3,
          source_field: "$Citizens_Born",
          source_line: 100,
        },
      ],
    },
  ],
  observation_limit: 256,
  city_limit: 512,
  tesmio_probe: {
    state: "missing",
    read_only: true,
    optional: true,
    persisted: false,
    probe_id: null,
    probe_version: null,
    loader_api_version: null,
    target_game_version: null,
    executable_timestamp: null,
    content_hash: null,
    snapshot_count: 0,
    sample_count: 0,
    latest_year: null,
    latest_day: null,
    latest_population_count: null,
    warnings: [],
  },
};

describe("population presentation", () => {
  it("creates branch-aligned save-sampled trends without inventing rates", () => {
    const status = createPopulationStatusChart(dataset, translate);
    const movement = createPopulationMovementChart(dataset, translate);
    expect(status.category_axis_scale).toBe("game_day");
    expect(status.series[0].points.map((point) => point.value)).toEqual([
      100, 110,
    ]);
    expect(movement.series[0].points.map((point) => point.value)).toEqual([
      7, 9,
    ]);
    expect(movement.series.some((series) => series.id.includes("rate"))).toBe(
      false,
    );
  });

  it("uses only direct head and city facts for profile charts", () => {
    const education = createEducationProfileChart(dataset, translate);
    const city = createCityMovementChart(dataset.cities[0], dataset, translate);
    expect(education.series[0].points.map((point) => point.value)).toEqual([
      30, 60, 20,
    ]);
    expect(city.series[0].points).toHaveLength(1);
    expect(city.provenance.kind).toBe("save_fact");
  });

  it("marks an absent intermediate fact instead of joining across it", () => {
    const gapped: PopulationDataset = {
      ...dataset,
      observations: [
        dataset.observations[0],
        {
          ...dataset.observations[0],
          interpretation_id: "observation-missing",
          membership_revision: 2,
          sampled_day: 13,
          sampled_game_day: 735_123,
          coverage_status: "partial",
          facts: [],
        },
        {
          ...dataset.observations[1],
          membership_revision: 3,
        },
      ],
    };
    const chart = createPopulationStatusChart(gapped, translate);
    expect(chart.series[0].points[1].gap_before).toBe(true);
    expect(chart.provenance.coverage).toBe("partial");
  });
});
