import { describe, expect, it } from "vitest";
import type { Translator } from "../i18n/runtime";
import {
  reviewMarketIndexingProgress,
  reviewMarketWorkspace,
} from "../ui-review/fixtures";
import {
  createMarketTradeChart,
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
});
