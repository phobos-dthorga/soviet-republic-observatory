import type { ChartSpec, Provenance } from "../charts/types";
import type { Translator } from "../i18n/runtime";
import type {
  MarketIndexingProgress,
  MarketMetricContext,
  MarketPriceSeries,
  MarketWorkspace,
} from "../observations/types";
import type { TaskProgressStage, TaskProgressView } from "../tasks/progress";
import type { ContextHelpContent } from "../ui/types";

const phaseOrder: Record<MarketIndexingProgress["phase"], number> = {
  idle: -1,
  discovering: 0,
  matching: 0,
  reading_archive: 1,
  parsing_records: 2,
  persisting: 3,
  queueing_warehouse: 3,
  paused: -1,
  complete: 4,
  failed: -1,
};

export function marketIndexingProgressView(
  progress: MarketIndexingProgress,
  translate: Translator,
): TaskProgressView {
  const stageKeys = [
    "markets-index-stage-match",
    "markets-index-stage-read",
    "markets-index-stage-parse",
    "markets-index-stage-store",
  ] as const;
  const stages: TaskProgressStage[] = stageKeys.map((key, index) => {
    const failed = progress.phase === "failed";
    const order = phaseOrder[progress.phase];
    const state = failed
      ? index === Math.max(0, order)
        ? "failed"
        : "pending"
      : progress.phase === "complete" || order > index
        ? "complete"
        : order === index
          ? "active"
          : "pending";
    return {
      id: String(index),
      label: translate(key),
      state,
      stateLabel: translate(
        state === "active"
          ? "task-progress-stage-active"
          : state === "complete"
            ? "task-progress-stage-complete"
            : state === "failed"
              ? "task-progress-stage-failed"
              : "task-progress-stage-pending",
      ),
    };
  });
  const headingKey = {
    idle: "markets-index-idle",
    discovering: "markets-index-discovering",
    matching: "markets-index-matching",
    reading_archive: "markets-index-reading",
    parsing_records: "markets-index-parsing",
    persisting: "markets-index-persisting",
    queueing_warehouse: "markets-index-persisting",
    paused: "markets-index-paused",
    complete: "markets-index-complete",
    failed: "markets-index-failed",
  } as const;
  return {
    taskId: "markets.index-available-saves",
    runId: progress.job_id ?? "markets.index-idle",
    state:
      progress.phase === "failed"
        ? "failed"
        : progress.phase === "paused"
          ? "paused"
          : progress.phase === "complete"
            ? "complete"
            : "running",
    eyebrow: translate("markets-index-eyebrow"),
    heading: translate(headingKey[progress.phase]),
    progressPercent: progress.progress_percent,
    stages,
    meters: progress.total_archives
      ? [
          {
            id: "archives",
            label: translate("markets-index-archives"),
            completed: progress.current_archive,
            total: progress.total_archives,
          },
        ]
      : [],
    metrics: [
      {
        id: "records",
        label: translate("markets-index-records"),
        value: String(progress.records_processed),
      },
      {
        id: "rows",
        label: translate("markets-index-rows"),
        value: String(progress.rows_processed),
      },
      {
        id: "missing",
        label: translate("markets-index-missing"),
        value: String(progress.missing_archives),
      },
      {
        id: "changed",
        label: translate("markets-index-changed"),
        value: String(progress.changed_archives),
      },
      {
        id: "failed",
        label: translate("markets-index-failures"),
        value: String(progress.failed_archives),
      },
      {
        id: "duplicates",
        label: translate("markets-index-duplicates"),
        value: String(progress.duplicate_archives),
      },
      {
        id: "cache-records",
        label: translate("markets-index-cache-records"),
        value: String(progress.cache_records_reused),
      },
      {
        id: "cache-rows",
        label: translate("markets-index-cache-rows"),
        value: String(progress.cache_rows_avoided),
      },
      {
        id: "contention",
        label: translate("markets-index-contention"),
        value: translate("markets-index-contention-value", {
          retries: progress.contention_retries,
          seconds: Math.round(progress.contention_wait_ms / 1000),
        }),
      },
    ],
    currentItemLabel: progress.current_file
      ? translate("markets-index-current-file")
      : null,
    currentItem: progress.current_file,
    currentItemContext: progress.total_archives
      ? translate("markets-index-current-count", {
          current: progress.current_archive,
          total: progress.total_archives,
        })
      : null,
    notice:
      progress.phase === "failed"
        ? { tone: "error", text: translate("markets-index-failed-detail") }
        : progress.phase === "paused"
          ? { tone: "warning", text: translate("markets-index-paused-detail") }
          : null,
  };
}

export function marketMetricHelp(
  context: MarketMetricContext,
  translate: Translator,
): ContextHelpContent {
  const exclusionLabels: Record<string, string> = {
    channels_separate: translate("markets-context-exclusion-channels"),
    negative_exports_are_disposal: translate(
      "markets-context-exclusion-disposal",
    ),
    no_annualisation_or_interpolation: translate(
      "markets-context-exclusion-interpolation",
    ),
    standard_channel_only: translate("markets-context-exclusion-standard"),
    non_positive_exports_excluded: translate(
      "markets-context-exclusion-non-positive",
    ),
    positive_prices_only: translate(
      "markets-context-exclusion-positive-prices",
    ),
    city_republic_windows_separate: translate(
      "markets-context-exclusion-city-window",
    ),
    compatible_denominator_required: translate(
      "markets-context-exclusion-denominator",
    ),
    currencies_separate: translate("markets-context-exclusion-currencies"),
    same_base_record_required: translate(
      "markets-context-exclusion-base-record",
    ),
  };
  const formulaLabels: Record<string, string> = {
    "export_account_value - import_account_value": translate(
      "markets-formula-trade-result",
    ),
    positive_export_hhi: translate("markets-formula-hhi"),
    recorded_price_and_relative_index: translate("markets-formula-price-index"),
    robust_log_price_movement: translate("markets-formula-volatility"),
    recorded_source_value: translate("markets-formula-source-value"),
    "export_price_index / import_price_index * 100": translate(
      "markets-formula-terms",
    ),
  };
  const unitLabels: Record<string, string> = {
    source_currency_account_value: translate("markets-unit-source-account"),
    concentration_index: translate("markets-unit-concentration"),
    source_currency_per_resource_unit: translate("markets-unit-resource-price"),
    log_price_movement: translate("markets-unit-log-movement"),
    source_native: translate("markets-source-native"),
    index_base_100: translate("markets-unit-index-base"),
  };
  const timeLabels: Record<string, string> = {
    selected_head_source_window: translate("markets-time-selected-window"),
    selected_head_and_first_compatible_record: translate(
      "markets-time-selected-and-base",
    ),
    available_proven_history_through_selected_head: translate(
      "markets-time-proven-history",
    ),
    selected_head_city_snapshot: translate("markets-time-city-snapshot"),
    matched_baskets_same_base_record: translate("markets-time-matched-baskets"),
  };
  return {
    topic: context.metric_id,
    title: translate("markets-context-title"),
    text: translate("markets-context-description"),
    details: [
      {
        label: translate("metric-context-formula"),
        value: formulaLabels[context.formula] ?? context.formula,
      },
      {
        label: translate("metric-context-unit"),
        value: unitLabels[context.unit] ?? context.unit,
      },
      {
        label: translate("metric-context-time-basis"),
        value: timeLabels[context.time_basis] ?? context.time_basis,
      },
      {
        label: translate("markets-context-currency"),
        value:
          context.currency?.toUpperCase() ?? translate("chart-unavailable"),
      },
      {
        label: translate("metric-context-evidence"),
        value: context.evidence_class,
      },
      {
        label: translate("metric-context-source-fields"),
        value:
          context.source_fields.join(", ") || translate("chart-unavailable"),
      },
      {
        label: translate("metric-context-profile"),
        value: `${context.profile_id}@${context.profile_version}`,
      },
      {
        label: translate("metric-context-analytical-head"),
        value: context.analytical_head.slice(0, 12),
      },
      {
        label: translate("metric-context-exclusions"),
        value: context.exclusions
          .map((item) => exclusionLabels[item] ?? item)
          .join(" · "),
      },
    ],
  };
}

function provenance(
  workspace: MarketWorkspace,
  translate: Translator,
): Provenance {
  return {
    kind: "save_fact",
    source: translate("markets-source-stats-profile", {
      profile: workspace.currencies[0]
        ? `${workspace.currencies[0].context.profile_id}@${workspace.currencies[0].context.profile_version}`
        : translate("chart-unavailable"),
    }),
    observed_at:
      workspace.analysis_context.head_interpretation_id?.slice(0, 12) ??
      translate("chart-unavailable"),
    coverage: workspace.partial ? "partial" : "complete",
  };
}

export function createMarketTradeChart(
  workspace: MarketWorkspace,
  currency: string,
  channel: string,
  translate: Translator,
): ChartSpec {
  const rows = workspace.trade_history.filter(
    (point) => point.currency === currency && point.channel === channel,
  );
  const chartProvenance = provenance(workspace, translate);
  const seriesFor = (
    id: "import_value" | "export_value" | "trade_result",
    label: string,
  ) => ({
    id,
    label,
    style: id === "trade_result" ? ("dashed" as const) : ("solid" as const),
    provenance: chartProvenance,
    points: rows.map((point) => ({
      category: translate("observation-game-date-compact", {
        year: point.year,
        day: String(point.day).padStart(3, "0"),
      }),
      category_value: point.game_day,
      value: point[id],
    })),
  });
  return {
    schema_version: 1,
    id: `markets-trade-${currency}-${channel}`,
    title: translate("markets-trade-chart-title", {
      currency: currency.toUpperCase(),
      channel: translate(
        channel === "international"
          ? "markets-channel-international"
          : "markets-channel-standard",
      ),
    }),
    description: translate("markets-trade-chart-description"),
    kind: "line",
    category_axis_scale: "game_day",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("markets-axis-account-value"),
    unit: currency.toUpperCase(),
    series: [
      seriesFor("import_value", translate("markets-imports")),
      seriesFor("export_value", translate("markets-exports")),
      seriesFor("trade_result", translate("markets-trade-result")),
    ],
    provenance: chartProvenance,
  };
}

export function createMarketPriceHistoryChart(
  workspace: MarketWorkspace,
  priceSeries: MarketPriceSeries | null,
  translate: Translator,
): ChartSpec {
  const chartProvenance = provenance(workspace, translate);
  const rows = priceSeries?.points ?? [];
  const seriesFor = (
    id: "purchase_price" | "sell_price" | "base_price",
    label: string,
  ) => ({
    id,
    label,
    style: id === "base_price" ? ("dashed" as const) : ("solid" as const),
    provenance: chartProvenance,
    points: rows.flatMap((point) => {
      const value = point[id];
      return value == null
        ? []
        : [
            {
              category: translate("observation-game-date-compact", {
                year: point.year,
                day: String(point.day).padStart(3, "0"),
              }),
              category_value: point.game_day,
              value,
            },
          ];
    }),
  });
  const currency = priceSeries?.currency.toUpperCase() ?? "—";
  return {
    schema_version: 1,
    id: `markets-price-history-${priceSeries?.currency ?? "none"}-${priceSeries?.resource_token ?? "none"}`,
    title: translate("markets-price-history-chart-title", {
      resource: priceSeries?.resource_token ?? translate("chart-unavailable"),
      currency,
    }),
    description: translate("markets-price-history-chart-description"),
    kind: "line",
    category_axis_scale: "game_day",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("markets-axis-resource-price"),
    unit: currency,
    series: [
      seriesFor("purchase_price", translate("markets-purchase-price")),
      seriesFor("sell_price", translate("markets-sell-price")),
      seriesFor("base_price", translate("markets-base-price")),
    ],
    provenance: chartProvenance,
  };
}

export function createPositiveExportChart(
  workspace: MarketWorkspace,
  currency: string,
  translate: Translator,
): ChartSpec {
  const rows = workspace.resource_ledger
    .filter(
      (row) =>
        row.currency === currency &&
        row.channel === "standard" &&
        row.export_account_value > 0,
    )
    .sort(
      (left, right) => right.export_account_value - left.export_account_value,
    )
    .slice(0, 20);
  const chartProvenance = provenance(workspace, translate);
  return {
    schema_version: 1,
    id: `markets-positive-exports-${currency}`,
    title: translate("markets-concentration-chart-title", {
      currency: currency.toUpperCase(),
    }),
    description: translate("markets-concentration-chart-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("markets-resource-token"),
    value_axis_label: translate("markets-positive-export-value"),
    unit: currency.toUpperCase(),
    series: [
      {
        id: "positive-exports",
        label: translate("markets-positive-exports"),
        provenance: chartProvenance,
        points: rows.map((row) => ({
          category: row.resource_token,
          value: row.export_account_value,
        })),
      },
    ],
    provenance: chartProvenance,
  };
}

export function createCityTradeChart(
  workspace: MarketWorkspace,
  currency: string,
  translate: Translator,
): ChartSpec {
  const rows = workspace.cities
    .filter((row) => row.currency === currency && row.channel === "standard")
    .sort(
      (left, right) =>
        Math.abs(right.trade_result) - Math.abs(left.trade_result),
    )
    .slice(0, 20);
  const chartProvenance = provenance(workspace, translate);
  return {
    schema_version: 1,
    id: `markets-city-${currency}`,
    title: translate("markets-city-chart-title", {
      currency: currency.toUpperCase(),
    }),
    description: translate("markets-city-chart-description"),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("markets-city-source"),
    value_axis_label: translate("markets-trade-result"),
    unit: currency.toUpperCase(),
    series: [
      {
        id: "city-trade-result",
        label: translate("markets-trade-result"),
        provenance: chartProvenance,
        points: rows.map((row) => ({
          category: translate("markets-city-neutral", { id: row.source_id }),
          value: row.trade_result,
        })),
      },
    ],
    provenance: chartProvenance,
  };
}
