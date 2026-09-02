import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import type { ArchiveComparison, ArchiveOverview } from "../observations/types";
import {
  createCadenceChart,
  createReceiverChangeChart,
  largestObservationInterval,
  selectedBranchObservations,
} from "./republicPulse";

const translate: Translator = (key, arguments_) =>
  `${key}${arguments_ ? JSON.stringify(arguments_) : ""}`;

const archive: ArchiveOverview = {
  selected_branch_id: "main",
  file_observation_count: 3,
  distinct_state_count: 3,
  unresolved_state_count: 0,
  branches: [],
  observations: [
    observation("latest", "main", 2, 80, 3),
    observation("fork", "fork-1", 8, 1, 2),
    observation("middle", "main", 2, 20, 2),
    observation("first", "main", 1, 330, 1),
  ],
  analysis_context: {
    context_id: "ctx-test",
    selected_branch_id: "main",
    head_interpretation_id: "latest-interpretation",
    original_branch_id: "main",
    mode: "latest",
    origin: "automatic",
    is_tip: true,
    membership_revision: 3,
    compatibility_profile_id: null,
    compatibility_profile_hash: null,
    observation_watermark: "latest-interpretation",
    catalogue_generation_id: null,
    resource_catalogue_revision_id: null,
    overlay_revision: null,
  },
};

describe("Republic Pulse", () => {
  it("orders only the selected resolved branch by actual game date", () => {
    expect(
      selectedBranchObservations(archive).map(
        (observation) => observation.payload_hash,
      ),
    ).toEqual(["first", "middle", "latest"]);
    expect(largestObservationInterval(archive)).toBe(60);
    expect(
      createCadenceChart(archive, translate, "en-AU").series[0].points,
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ value: 60 }),
        expect.objectContaining({ value: 55 }),
      ]),
    );
  });

  it("keeps signed receiver movements around an explicit zero reference", () => {
    const comparison = {
      from: { source_file_name: "first.zip" },
      to: { source_file_name: "latest.zip", coverage_status: "complete" },
      receiver_changes: [
        { metric_id: "core.citizens.electronics.radio", delta: -12 },
        { metric_id: "core.citizens.electronics.television", delta: 40 },
      ],
    } as ArchiveComparison;
    const chart = createReceiverChangeChart(comparison, translate);
    expect(chart.series[0].points.map((point) => point.value)).toEqual([
      -12, 40,
    ]);
    expect(chart.reference_lines?.[0].value).toBe(0);
  });
});

function observation(
  payload_hash: string,
  branch_id: string,
  latest_year: number,
  latest_day: number,
  imported_at_ms: number,
) {
  return {
    payload_hash,
    interpretation_id: `${payload_hash}-interpretation`,
    mapping_classification: "reviewed_mapping",
    profile_id: "org.example.test",
    profile_version: "1.0.0",
    resolved_profile_hash: "a".repeat(64),
    source_file_name: `${payload_hash}.zip`,
    imported_at_ms,
    branch_id,
    relationship: "successor" as const,
    parent_payload_hash: null,
    shared_record_count: 1,
    latest_year,
    latest_day,
    history_records: 1,
    coverage_status: "complete" as const,
    file_observation_count: 1,
    republic_snapshot_fields: 18,
    city_snapshot_count: 1,
    city_snapshot_fields: 5,
    included_in_context: branch_id === "main",
    active_head: payload_hash === "latest",
    context_sequence:
      branch_id === "main"
        ? payload_hash === "first"
          ? 1
          : payload_hash === "middle"
            ? 2
            : 3
        : null,
  };
}
