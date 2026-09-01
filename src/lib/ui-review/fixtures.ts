import type {
  ArchiveOverview,
  CatalogueRefreshProgress,
  CatalogueStatus,
  MarketIndexingProgress,
  MarketWorkspace,
  PopulationDataset,
  ProductionPathwayModel,
  ProductionRouteModel,
  ReceiverDataset,
  RepublicBrief,
  RepublicPlanWorkspace,
} from "../observations/types";

export function reviewReceiverDataset(): ReceiverDataset {
  const point = (
    interpretation_id: string,
    year: number,
    day: number,
    radio: number,
  ) => ({
    record_id: year * 365 + day,
    year,
    day,
    game_day: year * 365 + day,
    none: 32_790,
    radio,
    television: 9_702,
    computer: 2_457,
    classified_total: 32_790 + radio + 9_702 + 2_457,
    exact_observation: {
      interpretation_id,
      branch_id: "main",
      year,
      day,
    },
  });
  return {
    payload_hash: "review-receiver-payload",
    interpretation_id: "review-reading-2015-077",
    source_file_name: "UI-REVIEW-PG7.zip",
    source_file_size: 1_234_567,
    source_modified_ms: 1_725_000_000_000,
    imported_at_ms: 1_725_000_001_000,
    parser_version: "wrsr-definition-directives.v3",
    format_profile: "reviewed-1.1.1.9",
    compatibility: {
      profile_id: "org.republic-observatory.wrsr-1.1.1.9",
      profile_version: "1.0.0",
      profile_content_hash: "review-profile-content",
      resolved_profile_hash: "review-profile-resolved",
      base_profile_hash: null,
      profile_source: "reviewed_builtin",
      mapping_classification: "reviewed_mapping",
      parser_engine_version: "wrsr-definition-directives.v3",
    },
    branch_id: "main",
    original_branch_id: "main",
    analysis_context_id: "review-context-main-tip",
    geographic_scope: "whole_republic",
    coverage: {
      status: "complete",
      history_records: 2,
      chartable_records: 2,
      dropped_records: 0,
      warnings: [],
    },
    source_fields: [
      {
        metric_id: "core.citizens.electronics.radio",
        source_field: "$Citizens_ElectronicRadio",
        latest_source_line: 1_719_858,
      },
    ],
    points: [
      point("review-reading-2015-067", 2015, 67, 25_102),
      point("review-reading-2015-077", 2015, 77, 25_347),
    ],
  };
}

export function reviewMarketWorkspace(
  state: "ready" | "partial" | "empty" | "lagging" = "ready",
): MarketWorkspace {
  const available = state !== "empty";
  const context = {
    metric_id: "market.trade_result.rub",
    formula: "export_account_value - import_account_value",
    currency: "rub",
    unit: "source_currency_account_value",
    time_basis: "selected_head_source_window",
    exclusions: [
      "channels_separate",
      "negative_exports_are_disposal",
      "no_annualisation_or_interpolation",
    ],
    evidence_class: "reviewed_mapping",
    profile_id: "org.republic-observatory.wrsr-1.1.1.9",
    profile_version: "1.1.0",
    source_fields: ["$Resources_ImportRUB", "$Resources_ExportRUB"],
    analytical_head: "review-reading-2015-077",
  };
  const trades = [
    {
      record_hash: "a".repeat(64),
      year: 2015,
      day: 60,
      game_day: 735360,
      import_value: 1200,
      export_value: 900,
      trade_result: -300,
      exact_observation: null,
    },
    {
      record_hash: "b".repeat(64),
      year: 2015,
      day: 77,
      game_day: 735377,
      import_value: 1100,
      export_value: 1450,
      trade_result: 350,
      exact_observation: {
        interpretation_id: "review-reading-2015-077",
        branch_id: "main",
        year: 2015,
        day: 77,
      },
    },
  ].map((row) => ({ ...row, currency: "rub", channel: "standard" }));
  return {
    analysis_context: reviewContext,
    available,
    partial: state === "partial",
    coverage_status: available
      ? state === "partial"
        ? "partial"
        : "complete"
      : null,
    history_records: available ? 2 : 0,
    row_count: available ? 28 : 0,
    city_scope_count: available ? 2 : 0,
    warehouse_history_available: state !== "lagging",
    warnings: [],
    currencies: available
      ? [
          {
            currency: "rub",
            standard_import_value: 1100,
            standard_export_value: 1450,
            standard_trade_result: 350,
            international_import_value: 200,
            international_export_value: 125,
            international_trade_result: -75,
            positive_export_hhi: 0.58,
            positive_export_resource_count: 2,
            context,
          },
          {
            currency: "usd",
            standard_import_value: 0,
            standard_export_value: 0,
            standard_trade_result: 0,
            international_import_value: 0,
            international_export_value: 0,
            international_trade_result: 0,
            positive_export_hhi: null,
            positive_export_resource_count: 0,
            context: {
              ...context,
              metric_id: "market.trade_result.usd",
              currency: "usd",
            },
          },
        ]
      : [],
    trade_history: available ? trades : [],
    resource_ledger: available
      ? [
          {
            currency: "rub",
            channel: "standard",
            resource_token: "oil",
            import_quantity: 10,
            export_quantity: 0,
            import_account_value: 1100,
            export_account_value: 0,
            trade_result: -1100,
            disposal_cost: null,
            source_fields: ["$Resources_ImportRUB"],
          },
          {
            currency: "rub",
            channel: "standard",
            resource_token: "steel",
            import_quantity: 0,
            export_quantity: 12,
            import_account_value: 0,
            export_account_value: 1450,
            trade_result: 1450,
            disposal_cost: null,
            source_fields: ["$Resources_ExportRUB"],
          },
        ]
      : [],
    price_ledger: available
      ? [
          {
            currency: "rub",
            resource_token: "oil",
            purchase_price: 110,
            sell_price: 95,
            base_price: 100,
            purchase_index: 110,
            sell_index: 105,
            robust_log_volatility: 0.031,
            volatility_observations: 5,
            source_fields: ["$Economy_PurchaseCostRUB"],
          },
        ]
      : [],
    scalar_ledger: available
      ? [
          {
            fact_id: "market.loan.balance",
            currency: "rub",
            category: null,
            value: 5000,
            source_field: "$Loan_BalanceRUB",
            source_line: 144,
          },
        ]
      : [],
    cities: available
      ? [
          {
            source_id: "17",
            currency: "rub",
            channel: "standard",
            import_value: 300,
            export_value: 450,
            trade_result: 150,
          },
        ]
      : [],
    baskets: available
      ? [
          {
            basket_id: "builtin.observed-imports.rub",
            revision: 1,
            name: "observed_imports",
            currency: "rub",
            price_side: "purchase",
            built_in: true,
            selected: false,
            base_record_hash: "a".repeat(64),
            resource_count: 1,
            coverage_resources: 1,
            index_value: 110,
            reason: "observed_positive_import_quantities",
            weights: [{ resource_token: "oil", weight: 10 }],
          },
          {
            basket_id: "builtin.observed-positive-exports.rub",
            revision: 1,
            name: "observed_positive_exports",
            currency: "rub",
            price_side: "sell",
            built_in: true,
            selected: false,
            base_record_hash: "a".repeat(64),
            resource_count: 1,
            coverage_resources: 1,
            index_value: 105,
            reason: "observed_positive_export_quantities",
            weights: [{ resource_token: "steel", weight: 12 }],
          },
        ]
      : [],
    scenarios: [],
    metric_contexts: available
      ? [
          {
            ...context,
            metric_id: "market.positive_export_hhi.rub.standard",
            formula: "positive_export_hhi",
            unit: "concentration_index",
          },
          {
            ...context,
            metric_id: "market.price.rub",
            formula: "recorded_price_and_relative_index",
            unit: "source_currency_per_resource_unit",
          },
          {
            ...context,
            metric_id: "market.city_trade_result.rub.standard",
            time_basis: "selected_head_city_snapshot",
          },
          {
            ...context,
            metric_id: "market.scalar_accounts",
            currency: null,
            formula: "recorded_source_value",
            unit: "source_native",
          },
        ]
      : [],
    terms_of_trade: available
      ? [
          {
            currency: "rub",
            base_record_hash: "a".repeat(64),
            import_basket_id: "builtin.observed-imports.rub",
            import_basket_revision: 1,
            export_basket_id: "builtin.observed-positive-exports.rub",
            export_basket_revision: 1,
            import_index: 110,
            export_index: 105,
            terms_of_trade_index: 95.45,
            context: {
              ...context,
              metric_id: "market.terms_of_trade.rub",
              formula:
                "export_fixed_basket_index / import_fixed_basket_index * 100",
              unit: "index_base_100",
            },
          },
        ]
      : [],
    reserves_available: false,
    terms_of_trade_available: available,
    limitations: [
      "reserves_unavailable",
      "city_republic_windows_separate",
      "currencies_require_explicit_exchange",
      "loan_tourism_denominator_required",
      "no_annualisation_or_interpolation",
    ],
    commissioning: {
      recorded_save_count: 25,
      indexed_save_count: available ? 23 : 0,
      current_engine_indexed_save_count: available ? 21 : 0,
      pending_current_engine_save_count: available ? 4 : 25,
      active_engine_current: available,
      active_parser_engine_version: available
        ? "compatibility-profile-engine.v2"
        : null,
      recommended_currency: available ? "rub" : null,
      recommended_channel: available ? "standard" : null,
      recommended_price_resource: available ? "oil" : null,
      facets: [
        {
          facet_id: "prices",
          status: available ? "observed" : "not_observed",
          observed_slots: available ? 6 : 0,
          expected_slots: 6,
          resource_count: available ? 56 : 0,
          currencies: available ? ["rub", "usd"] : [],
          channels: [],
          source_fields: available ? ["$Economy_PurchaseCostRUB"] : [],
        },
        {
          facet_id: "trade",
          status:
            state === "partial"
              ? "partial"
              : available
                ? "observed"
                : "not_observed",
          observed_slots: available ? (state === "partial" ? 4 : 8) : 0,
          expected_slots: 8,
          resource_count: available ? 41 : 0,
          currencies: available ? ["rub", "usd"] : [],
          channels: available ? ["standard", "international"] : [],
          source_fields: available ? ["$Resources_ImportRUB"] : [],
        },
        ...["costs", "tourism", "loans", "vehicles", "cities"].map(
          (facetId) => ({
            facet_id: facetId,
            status: available
              ? ("partial" as const)
              : ("not_observed" as const),
            observed_slots: available ? 2 : 0,
            expected_slots:
              facetId === "cities" ? 3 : facetId === "costs" ? 6 : 4,
            resource_count: 0,
            currencies: available ? ["rub", "usd"] : [],
            channels: [],
            source_fields: [],
          }),
        ),
      ],
    },
  };
}

export function reviewMarketIndexingProgress(
  failed = false,
): MarketIndexingProgress {
  return {
    job_id: "review-market-index",
    storage_contract_version: 2,
    phase: failed ? "failed" : "parsing_records",
    progress_percent: failed ? 46 : 63,
    started_at_ms: 1,
    updated_at_ms: 2,
    current_file: "Recorded-save-014.zip",
    current_archive: 14,
    total_archives: 25,
    records_processed: 1800,
    rows_processed: 425000,
    completed_archives: 13,
    missing_archives: 2,
    changed_archives: 1,
    failed_archives: failed ? 1 : 0,
    duplicate_archives: 4,
    cache_records_reused: 1195,
    cache_rows_avoided: 286000,
    contention_retries: 2,
    contention_wait_ms: 410,
    resume_count: 1,
    error_code: failed ? "invalid_archive" : null,
  };
}

const reviewContext = {
  context_id: "review-context-main-2015-077",
  selected_branch_id: "main",
  head_interpretation_id: "review-reading-2015-077",
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
      interpretation_id: "review-reading-2015-077",
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
      interpretation_id: "review-reading-2015-067",
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
    plan: {
      plan_id: "plan-review-five-year",
      name: "Fifth Five-Year Plan",
      revision: 2,
      target_count: 2,
      end_year: 2020,
      end_day: 77,
      state: "on_track",
      attainment_basis_points: 9_740,
      guardrail_breach_count: 0,
    },
    unavailable_capabilities: ["import_exposure", "observed_material_reliance"],
  };
}

export function reviewRepublicPlanWorkspace(): RepublicPlanWorkspace {
  const context = {
    population_basis: "source_defined_adults" as const,
    time_basis: "branch_observations_through_selected_head" as const,
    geographic_scope: "whole_republic" as const,
    denominator_metric_id: null,
    comparison_basis: "player_plan_schedule" as const,
    limitations: ["not_employment_count" as const],
  };
  const target = {
    metric_id: "source.stats.citizens.adults",
    baseline_value: 55_000,
    target_value: 65_000,
    direction: "increase" as const,
    guardrail_basis_points: 500,
  };
  return {
    analysis_context: { ...reviewContext },
    current_year: 2015,
    current_day: 77,
    available_metrics: [
      {
        metric_id: target.metric_id,
        current_value: 58_137,
        active_plan_baseline_value: 55_000,
        context,
      },
      {
        metric_id: "source.stats.citizens.small_children",
        current_value: 5_974,
        active_plan_baseline_value: 5_700,
        context: {
          ...context,
          population_basis: "source_defined_small_children",
          limitations: ["source_age_boundary_unverified"],
        },
      },
    ],
    plans: [
      {
        plan_id: "plan-review-five-year",
        name: "Fifth Five-Year Plan",
        branch_id: "main",
        active_revision: 2,
        latest_revision: 2,
        revision_count: 2,
        selected: true,
      },
    ],
    active_plan: {
      revision: {
        plan_id: "plan-review-five-year",
        name: "Fifth Five-Year Plan",
        revision: 2,
        branch_id: "main",
        start_interpretation_id: "review-reading-2015-067",
        start_profile_hash: "reviewed-profile-fixture",
        start_year: 2015,
        start_day: 67,
        start_game_day: 4_083,
        end_year: 2020,
        end_day: 77,
        end_game_day: 5_918,
        schedule: "linear",
        created_at_ms: 1_788_000_000_000,
        targets: [target],
      },
      state: "on_track",
      attainment_basis_points: 9_740,
      guardrail_breach_count: 0,
      targets: [
        {
          target,
          current_value: 58_137,
          scheduled_value: 58_400,
          directional_variance: -263,
          attainment_basis_points: 9_687,
          guardrail_breached: false,
          state: "on_track",
          context,
          points: [
            {
              year: 2015,
              day: 67,
              game_day: 4_083,
              observed_value: 55_000,
              scheduled_value: 55_000,
              exact_observation: null,
            },
            {
              year: 2015,
              day: 77,
              game_day: 4_093,
              observed_value: 58_137,
              scheduled_value: 58_400,
              exact_observation: {
                interpretation_id: "review-reading-2015-077",
                branch_id: "main",
                year: 2015,
                day: 77,
              },
            },
          ],
        },
      ],
    },
  };
}

export function reviewPopulationDataset(): PopulationDataset {
  return {
    analysis_context: { ...reviewContext },
    observations: [
      {
        interpretation_id: "review-reading-2015-077",
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
        exact_observation: {
          interpretation_id: "review-reading-2015-077",
          branch_id: "main",
          year: 2015,
          day: 77,
        },
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
    ? "review-reading-2015-067"
    : "review-reading-2015-077";
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
      const interpretationId = `review-reading-2015-0${day}`;
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
