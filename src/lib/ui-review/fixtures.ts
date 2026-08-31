import type {
  ArchiveOverview,
  CatalogueRefreshProgress,
  CatalogueStatus,
  PopulationDataset,
  ProductionPathwayModel,
  ProductionRouteModel,
  RepublicBrief,
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

export function reviewRepublicBrief(): RepublicBrief {
  const source = (source_field: string, source_line: number) => [
    { source_field, source_line },
  ];
  const context = (
    population_basis:
      | "all_recorded_citizens"
      | "source_defined_adults"
      | "source_defined_small_children"
      | "source_defined_unemployed"
      | "classified_receiver_population",
    limitations: Array<
      | "not_employment_count"
      | "not_workers_only"
      | "source_age_boundary_unverified"
      | "source_window_unverified"
      | "excludes_unclassified_citizens"
    >,
    denominator_metric_id: string | null = null,
  ) => ({
    population_basis,
    time_basis: "exact_selected_observation" as const,
    geographic_scope: "whole_republic" as const,
    denominator_metric_id,
    comparison_basis: "proven_preceding_same_branch_and_profile" as const,
    limitations,
  });
  return {
    schema_version: 1,
    analysis_context: { ...reviewContext },
    observation: {
      interpretation_id: "review-interpretation-2015-077",
      source_file_name: "UI-REVIEW-PG7.zip",
      year: 2015,
      day: 77,
      game_day: 4_093,
      coverage_status: "complete",
      mapping_classification: "reviewed_mapping",
      profile_id: "org.republic-observatory.wrsr-1.1.1.9",
      profile_version: "1.0.0",
      resolved_profile_hash: "reviewed-profile-fixture",
    },
    comparison: {
      interpretation_id: "review-interpretation-2015-067",
      source_file_name: "UI-REVIEW-PG6.zip",
      year: 2015,
      day: 67,
      game_day: 4_083,
    },
    metrics: [
      {
        metric_id: "source.stats.citizens.adults",
        role: "headline",
        value: 58_137,
        previous_value: 57_904,
        delta: 233,
        share_basis_points: null,
        evidence_kind: "save_fact",
        sources: source("$Citizens_Adults", 1_719_858),
        context: context("source_defined_adults", ["not_employment_count"]),
      },
      {
        metric_id: "source.stats.citizens.small_children",
        role: "headline",
        value: 5_974,
        previous_value: 5_942,
        delta: 32,
        share_basis_points: null,
        evidence_kind: "save_fact",
        sources: source("$Citizens_ChildrenSmall", 1_719_857),
        context: context("source_defined_small_children", [
          "source_age_boundary_unverified",
        ]),
      },
      {
        metric_id: "source.stats.citizens.unemployed",
        role: "headline",
        value: 24_034,
        previous_value: 23_811,
        delta: 223,
        share_basis_points: null,
        evidence_kind: "save_fact",
        sources: source("$Citizens_Unemployed", 1_719_860),
        context: context("source_defined_unemployed", [
          "source_window_unverified",
        ]),
      },
      {
        metric_id: "core.citizens.electronics.classified_total",
        role: "headline",
        value: 70_296,
        previous_value: 70_041,
        delta: 255,
        share_basis_points: null,
        evidence_kind: "calculation",
        sources: [
          { source_field: "$Citizens_ElectronicNone", source_line: 1_719_856 },
          { source_field: "$Citizens_ElectronicRadio", source_line: 1_719_858 },
          { source_field: "$Citizens_ElectronicTV", source_line: 1_719_859 },
          {
            source_field: "$Citizens_ElectronicComputer",
            source_line: 1_719_860,
          },
        ],
        context: context("classified_receiver_population", [
          "excludes_unclassified_citizens",
        ]),
      },
      ...[
        [
          "source.stats.citizens.no_education",
          12_902,
          "$Citizens_NoEducation",
          1_719_861,
        ],
        [
          "source.stats.citizens.basic_education",
          42_345,
          "$Citizens_BasicEducationNum",
          1_719_862,
        ],
        [
          "source.stats.citizens.higher_education",
          22_018,
          "$Citizens_HighEducationNum",
          1_719_863,
        ],
      ].map(([metric_id, value, source_field, source_line]) => ({
        metric_id: metric_id as string,
        role: "education" as const,
        value: value as number,
        previous_value: null,
        delta: null,
        share_basis_points: null,
        evidence_kind: "save_fact" as const,
        sources: source(source_field as string, source_line as number),
        context: context("all_recorded_citizens", ["not_workers_only"]),
      })),
      ...[
        [
          "core.citizens.electronics.none",
          32_790,
          4_665,
          "$Citizens_ElectronicNone",
          1_719_856,
        ],
        [
          "core.citizens.electronics.radio",
          25_347,
          3_606,
          "$Citizens_ElectronicRadio",
          1_719_858,
        ],
        [
          "core.citizens.electronics.television",
          9_702,
          1_380,
          "$Citizens_ElectronicTV",
          1_719_859,
        ],
        [
          "core.citizens.electronics.computer",
          2_457,
          349,
          "$Citizens_ElectronicComputer",
          1_719_860,
        ],
      ].map(
        ([
          metric_id,
          value,
          share_basis_points,
          source_field,
          source_line,
        ]) => ({
          metric_id: metric_id as string,
          role: "receiver_class" as const,
          value: value as number,
          previous_value: null,
          delta: null,
          share_basis_points: share_basis_points as number,
          evidence_kind: "save_fact" as const,
          sources: source(source_field as string, source_line as number),
          context: context(
            "classified_receiver_population",
            ["excludes_unclassified_citizens"],
            "core.citizens.electronics.classified_total",
          ),
        }),
      ),
    ],
    findings: [],
    dispatch_code: "observation_ready",
    operations: {
      recorder_phase: "watching",
      recorder_queue_depth: 0,
      recorder_attention_count: 0,
      warehouse_phase: "ready",
      warehouse_pending_jobs: 0,
      warehouse_failed_jobs: 0,
      warehouse_lag_ms: 0,
      catalogue_generation_id: "review-catalogue-generation",
      catalogue_entity_count: 6_031,
      city_scope_count: 139,
    },
    unavailable_capabilities: [
      "plan_attainment",
      "import_exposure",
      "observed_material_reliance",
    ],
  };
}

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

const reviewDefinitionMapping = {
  mapping_id: "core.definition.production_input",
  catalogue_scope_id: null,
  mapping_classification: "reviewed_mapping",
  scope_state: null,
  update_policy: null,
  acknowledged_content_hash: null,
  current_content_hash: null,
};

const reviewWarehouseSnapshot = {
  catalogue_generation_id: "review-production-generation",
  compatibility_profile_id: "org.republic-observatory.wrsr-1.1.1.9",
  compatibility_profile_version: "1.0.0",
  compatibility_profile_hash: "reviewed-profile-fixture",
  mapping_classification: "reviewed_mapping",
  overlay_profile_id: null,
  overlay_revision: null,
  observation_watermark: "review-watermark-077",
  warehouse_schema_version: 5,
  projector_version: "republic-observatory-projector.v1",
};

export function reviewProductionRoute(): ProductionRouteModel {
  return {
    schema_version: 2,
    route_id: "review::recipe::fuel",
    revision_hash: "review-fuel-revision",
    building_entity_id: "review::building::fuel-refinery",
    display_name: "Fuel refinery production route",
    package_name: "Deterministic UI review catalogue",
    coverage: "complete",
    status: "ready_with_auxiliary",
    relation_count: 3,
    primary_flow_count: 2,
    auxiliary_flow_count: 1,
    unit: "source_rate",
    selected_output_resource_id: "resource::fuel",
    target_quantity: 10,
    scale_factor: 10,
    mapping_classification: "reviewed_mapping",
    flows: [
      {
        id: "production_input-0",
        direction: "production_input",
        resource_id: "resource::oil",
        display_name: "oil",
        source_quantity: 2,
        scaled_quantity: 20,
        unit: "source_rate",
        basis_role: "primary",
        basis_exclusion: null,
        resolution: "source_coefficient",
        source_directive: "$CONSUMPTION",
        source_line: 10,
        mapping: reviewDefinitionMapping,
      },
      {
        id: "production_input-1",
        direction: "production_input",
        resource_id: "resource::eletric",
        display_name: "eletric",
        source_quantity: 0.01,
        scaled_quantity: 0.1,
        unit: "per_second",
        basis_role: "auxiliary",
        basis_exclusion: "different_unit",
        resolution: "source_coefficient",
        source_directive: "$CONSUMPTION_PER_SECOND",
        source_line: 11,
        mapping: reviewDefinitionMapping,
      },
      {
        id: "production_output-0",
        direction: "production_output",
        resource_id: "resource::fuel",
        display_name: "fuel",
        source_quantity: 1,
        scaled_quantity: 10,
        unit: "source_rate",
        basis_role: "primary",
        basis_exclusion: null,
        resolution: "source_coefficient",
        source_directive: "$PRODUCTION",
        source_line: 12,
        mapping: reviewDefinitionMapping,
      },
    ],
    snapshot: reviewWarehouseSnapshot,
  };
}

export function reviewProductionPathway(): ProductionPathwayModel {
  return {
    schema_version: 1,
    status: "needs_selection",
    root_recipe_entity_id: "review::recipe::fuel",
    output_resource_id: "resource::fuel",
    target_quantity: 10,
    unit: "source_rate",
    max_depth: 4,
    mapping_classification: "reviewed_mapping",
    nodes: [
      {
        id: "oil",
        kind: "resource",
        display_name: "oil",
        resource_id: "resource::oil",
        recipe_entity_id: null,
        package_name: null,
        depth: 1,
      },
      {
        id: "fuel-stage",
        kind: "process",
        display_name: "Fuel refinery production route",
        resource_id: null,
        recipe_entity_id: "review::recipe::fuel",
        package_name: "UI review",
        depth: 0,
      },
      {
        id: "fuel",
        kind: "resource",
        display_name: "fuel",
        resource_id: "resource::fuel",
        recipe_entity_id: null,
        package_name: null,
        depth: 0,
      },
    ],
    links: [
      {
        id: "oil-input",
        source: "oil",
        target: "fuel-stage",
        resource_id: "resource::oil",
        quantity: 20,
        unit: "source_rate",
        source_directive: "$CONSUMPTION",
        source_line: 10,
        mapping: reviewDefinitionMapping,
      },
      {
        id: "fuel-output",
        source: "fuel-stage",
        target: "fuel",
        resource_id: "resource::fuel",
        quantity: 10,
        unit: "source_rate",
        source_directive: "$PRODUCTION",
        source_line: 12,
        mapping: reviewDefinitionMapping,
      },
    ],
    choices: [
      {
        resource_node_id: "oil",
        resource_id: "resource::oil",
        display_name: "oil",
        required_quantity: 20,
        unit: "source_rate",
        selected_recipe_entity_id: null,
        candidates: [
          {
            recipe_entity_id: "review::recipe::crude-oil",
            display_name: "Crude-oil refinery route",
            package_name: "Base-game definitions",
            output_quantity: 1,
            unit: "source_rate",
          },
          {
            recipe_entity_id: "review::recipe::bio-oil",
            display_name: "Bio-oil refinery route",
            package_name: "Community industry pack",
            output_quantity: 1,
            unit: "source_rate",
          },
        ],
      },
    ],
    terminal_requirements: [
      {
        resource_id: "resource::oil",
        display_name: "oil",
        quantity: 20,
        unit: "source_rate",
        reason: "route_selection_required",
      },
    ],
    auxiliary_requirements: [
      {
        stage_id: "fuel-stage",
        recipe_entity_id: "review::recipe::fuel",
        resource_id: "resource::eletric",
        display_name: "eletric",
        quantity: 0.1,
        unit: "per_second",
        reason: "different_unit",
        source_directive: "$CONSUMPTION_PER_SECOND",
        source_line: 11,
        mapping: reviewDefinitionMapping,
      },
    ],
    diagnostics: [],
    snapshot: reviewWarehouseSnapshot,
  };
}
