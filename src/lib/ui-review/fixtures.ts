import type {
  ArchiveOverview,
  CatalogueRefreshProgress,
  CatalogueStatus,
  PopulationDataset,
} from "../observations/types";

const reviewContext = {
  context_id: "review-context-main-2015-077",
  selected_branch_id: "main",
  head_interpretation_id: "review-interpretation-2015-077",
  original_branch_id: "main",
  mode: "latest" as const,
  origin: "automatic" as const,
  is_tip: true,
  membership_revision: 38,
  compatibility_profile_id: "org.republic-observatory.wrsr-1.1.1.9",
  compatibility_profile_hash: "reviewed-profile-fixture",
  observation_watermark: "review-watermark-077",
  catalogue_generation_id: "review-catalogue-generation",
  overlay_revision: null,
};

export function reviewPopulationDataset(): PopulationDataset {
  return {
    analysis_context: { ...reviewContext },
    observations: [
      {
        interpretation_id: "review-interpretation-2015-077",
        source_file_name: "UI-REVIEW-PG7.zip",
        membership_revision: 38,
        sampled_year: 2015,
        sampled_day: 77,
        sampled_game_day: 4_093,
        coverage_status: "complete",
        mapping_classification: "reviewed_mapping",
        profile_id: "org.republic-observatory.wrsr-1.1.1.9",
        profile_version: "1.0.0",
        resolved_profile_hash: "reviewed-profile-fixture",
        facts: [
          {
            fact_id: "source.stats.citizens.adults",
            value: 58_137,
            source_field: "$Citizens_Adults",
            source_line: 1_719_858,
          },
          {
            fact_id: "source.stats.citizens.small_children",
            value: 5_974,
            source_field: "$Citizens_ChildrenSmall",
            source_line: 1_719_857,
          },
          {
            fact_id: "source.stats.citizens.unemployed",
            value: 24_034,
            source_field: "$Citizens_Unemployed",
            source_line: 1_719_860,
          },
        ],
      },
    ],
    cities: [],
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
}

export function reviewArchiveOverview(historical = false): ArchiveOverview {
  const head = historical
    ? "review-interpretation-2015-067"
    : "review-interpretation-2015-077";
  return {
    selected_branch_id: "main",
    file_observation_count: 2,
    distinct_state_count: 2,
    unresolved_state_count: 0,
    branches: [
      {
        branch_id: "main",
        branch_kind: "main",
        parent_branch_id: null,
        fork_record_id: null,
        observation_count: 2,
        latest_year: 2015,
        latest_day: 77,
        selected: true,
        origin: "automatic",
        short_identity: "main",
        player_label: null,
        anchor_interpretation_id: null,
        membership_revision: 38,
      },
    ],
    observations: [67, 77].map((day, index) => {
      const interpretationId = `review-interpretation-2015-0${day}`;
      return {
        payload_hash: `review-payload-${day}`,
        interpretation_id: interpretationId,
        mapping_classification: "reviewed_mapping",
        profile_id: "org.republic-observatory.wrsr-1.1.1.9",
        profile_version: "1.0.0",
        resolved_profile_hash: "reviewed-profile-fixture",
        source_file_name: `UI-REVIEW-PG${index + 6}.zip`,
        imported_at_ms: 1_788_000_000_000 + day,
        branch_id: "main",
        relationship: index === 0 ? ("root" as const) : ("successor" as const),
        parent_payload_hash: index === 0 ? null : "review-payload-67",
        shared_record_count: index === 0 ? 0 : 37,
        latest_year: 2015,
        latest_day: day,
        history_records: index + 37,
        coverage_status: "complete" as const,
        file_observation_count: 1,
        republic_snapshot_fields: 24,
        city_snapshot_count: 139,
        city_snapshot_fields: 2_224,
        included_in_context: day <= (historical ? 67 : 77),
        active_head: interpretationId === head,
        context_sequence: day <= (historical ? 67 : 77) ? index + 1 : null,
      };
    }),
    analysis_context: {
      ...reviewContext,
      context_id: historical
        ? "review-context-main-2015-067"
        : reviewContext.context_id,
      head_interpretation_id: head,
      mode: historical ? "historical_preview" : "latest",
      is_tip: !historical,
    },
  };
}

export function reviewCatalogueProgress(
  failed = false,
): CatalogueRefreshProgress {
  return {
    phase: failed ? "failed" : "scanning",
    trigger: "manual",
    progress_percent: failed ? 61 : 42,
    started_at_ms: 1_788_000_000_000,
    updated_at_ms: 1_788_000_005_000,
    current_source: "Base-game definitions",
    current_file: failed ? "definition review interrupted" : "building.ini",
    current_file_index: 2_311,
    sources_discovered: 925,
    sources_total: 1_287,
    files_discovered: 5_505,
    files_processed: 2_311,
    files_reused: 2_000,
    files_parsed: 311,
    entities_prepared: 2_614,
    rows_written: 0,
    rows_total: 42_350,
    error_code: failed ? "ui_review_fixture_failure" : null,
  };
}

export function reviewWarehouseAttention(): CatalogueStatus {
  const refresh = reviewCatalogueProgress(false);
  refresh.phase = "complete";
  refresh.progress_percent = 100;
  return {
    warehouse: {
      phase: "attention",
      schema_version: 5,
      pending_jobs: 2,
      failed_jobs: 0,
      lag_ms: 34_560_000,
      last_projected_at_ms: 1_788_000_000_000,
      observation_watermark: "review-watermark-077",
      database_size_bytes: 80_216_064,
      active_write: null,
      consecutive_write_failures: 0,
      retry_after_ms: null,
    },
    generation: null,
    last_checked_at_ms: 1_788_000_005_000,
    last_refreshed_at_ms: 1_788_000_000_000,
    last_filesystem_event_ms: null,
    error_code: null,
    active_overlay: null,
    refresh,
  };
}
