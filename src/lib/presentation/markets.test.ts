import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import {
  reviewMarketIndexingProgress,
  reviewMarketWorkspace,
} from "../ui-review/fixtures";
import {
  createMarketTradeChart,
  createMarketPriceHistoryChart,
  createPositiveExportChart,
  marketIndexingProgressView,
  marketMetricHelp,
} from "./markets";

const translate = ((key: string, arguments_?: Record<string, unknown>) =>
  arguments_ ? `${key}:${JSON.stringify(arguments_)}` : key) as Translator;

describe("Markets presentation", () => {
  it("keeps currency and channel selections out of one another's series", () => {
    const workspace = reviewMarketWorkspace();
    const standard = createMarketTradeChart(
      workspace,
      "rub",
      "standard",
      translate,
    );
    const international = createMarketTradeChart(
      workspace,
      "rub",
      "international",
      translate,
    );
    const dollars = createMarketTradeChart(
      workspace,
      "usd",
      "standard",
      translate,
    );

    expect(standard.series[0].points).toHaveLength(2);
    expect(international.series[0].points).toHaveLength(0);
    expect(dollars.series[0].points).toHaveLength(0);
  });

  it("excludes imports, zero exports, and disposal entries from concentration", () => {
    const workspace = reviewMarketWorkspace();
    workspace.resource_ledger.push(
      {
        ...workspace.resource_ledger[0],
        resource_token: "zero-export",
        import_account_value: 0,
        export_account_value: 0,
      },
      {
        ...workspace.resource_ledger[0],
        resource_token: "disposal",
        import_account_value: 0,
        export_account_value: -10,
        disposal_cost: 10,
      },
    );
    const chart = createPositiveExportChart(workspace, "rub", translate);

    expect(chart.series[0].points.map((point) => point.category)).toEqual([
      "steel",
    ]);
  });

  it("keeps queueing in the bounded indexing progress contract", () => {
    const progress = {
      ...reviewMarketIndexingProgress(),
      phase: "queueing_warehouse" as const,
      progress_percent: 97,
    };
    const view = marketIndexingProgressView(progress, translate);

    expect(view.progressPercent).toBe(97);
    expect(view.stages.at(-1)?.state).toBe("active");
  });

  it("presents storage contention as a resumable pause", () => {
    const progress = {
      ...reviewMarketIndexingProgress(),
      phase: "paused" as const,
      error_code: "storage_occupied",
    };
    const view = marketIndexingProgressView(progress, translate);

    expect(view.state).toBe("paused");
    expect(view.notice?.tone).toBe("warning");
  });

  it("renders Rust metric units and time bases instead of assuming trade units", () => {
    const workspace = reviewMarketWorkspace();
    const price = workspace.metric_contexts.find(
      (context) => context.metric_id === "market.price.rub",
    )!;
    const help = marketMetricHelp(price, translate);

    expect(
      help.details?.find((detail) => detail.label === "metric-context-unit")
        ?.value,
    ).toBe("markets-unit-resource-price");
  });

  it("renders only recorded values in a bounded selected-resource price history", () => {
    const workspace = reviewMarketWorkspace();
    const chart = createMarketPriceHistoryChart(
      workspace,
      {
        available: true,
        currency: "rub",
        resource_token: "oil",
        limitation: null,
        context: workspace.metric_contexts.find(
          (context) => context.metric_id === "market.price.rub",
        )!,
        points: [
          {
            record_hash: "a".repeat(64),
            year: 2015,
            day: 60,
            game_day: 735360,
            purchase_price: 110,
            sell_price: null,
            base_price: 100,
          },
          {
            record_hash: "b".repeat(64),
            year: 2015,
            day: 77,
            game_day: 735377,
            purchase_price: 112,
            sell_price: 97,
            base_price: 100,
          },
        ],
      },
      translate,
    );

    expect(chart.series[0].points).toHaveLength(2);
    expect(chart.series[1].points).toHaveLength(1);
    expect(chart.series[2].points).toHaveLength(2);
  });
});
