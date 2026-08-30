<script lang="ts">
  import { BarChart, LineChart, SankeyChart } from "echarts/charts";
  import {
    GridComponent,
    LegendComponent,
    MarkLineComponent,
    TooltipComponent,
  } from "echarts/components";
  import * as echarts from "echarts/core";
  import type { ECharts } from "echarts/core";
  import { CanvasRenderer } from "echarts/renderers";
  import { onMount } from "svelte";
  import type { TranslationKey } from "../i18n/catalog";
  import { formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import {
    condensedSeriesSummary,
    optionForChart,
    provenanceForSeries,
  } from "./chartOptions";
  import {
    optionForSankey,
    provenanceForLink,
    summariseSankey,
  } from "./sankey";
  import type {
    ChartPoint,
    EvidenceCoverage,
    EvidenceKind,
    ObservatoryChartSpec,
  } from "./types";

  echarts.use([
    BarChart,
    LineChart,
    SankeyChart,
    GridComponent,
    LegendComponent,
    MarkLineComponent,
    TooltipComponent,
    CanvasRenderer,
  ]);

  let {
    spec,
    height = "250px",
    eyebrow,
  }: {
    spec: ObservatoryChartSpec;
    height?: string;
    eyebrow: string;
  } = $props();
  let container = $state<HTMLDivElement>();
  let chart = $state.raw<ECharts>();
  let flowLedgerOpen = $state(false);
  const resolvedHeight = $derived(
    spec.kind === "sankey"
      ? "360px"
      : spec.kind === "bar" && spec.orientation === "horizontal"
        ? `${Math.max(220, spec.series[0]?.points.length * 29 + 82)}px`
        : height,
  );
  const empty = $derived(
    spec.kind === "sankey" ? spec.links.length === 0 : spec.series.length === 0,
  );
  const evidenceKeys: Record<EvidenceKind, TranslationKey> = {
    save_fact: "evidence-save-fact",
    game_definition: "evidence-game-definition",
    calculation: "evidence-calculation",
    extension_calculation: "evidence-extension-calculation",
    player_override: "evidence-player-override",
    player_definition: "evidence-player-definition",
    estimate: "evidence-estimate",
    recommendation: "evidence-recommendation",
  };
  const coverageKeys: Record<EvidenceCoverage, TranslationKey> = {
    complete: "coverage-complete",
    partial: "coverage-partial",
    experimental: "coverage-experimental",
  };

  function formatValue(value: number): string {
    return `${formatNumber(value, $activeLocale, { maximumFractionDigits: 2 })}${spec.unit ? ` ${spec.unit}` : ""}`;
  }

  function formatPoint(point: ChartPoint): string {
    return $translation(
      point.gap_before ? "chart-summary-gap-point" : "chart-summary-point",
      { category: point.category, value: formatValue(point.value) },
    );
  }

  function accessiblePointSummary(points: ChartPoint[]): string {
    const condensed = condensedSeriesSummary(points);
    if (!condensed) return points.map(formatPoint).join("; ");

    const summary = $translation("chart-summary-condensed-points", {
      count: condensed.count,
      first: formatPoint(condensed.first),
      minimum: formatPoint(condensed.minimum),
      maximum: formatPoint(condensed.maximum),
      latest: formatPoint(condensed.latest),
    });
    return condensed.gapCount > 0
      ? `${summary} ${$translation("chart-summary-gap-count", { count: condensed.gapCount })}`
      : summary;
  }

  function refreshChart(): void {
    if (!chart) return;
    const reducedMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    chart.setOption(
      spec.kind === "sankey"
        ? optionForSankey(spec, undefined, reducedMotion, $activeLocale)
        : optionForChart(
            spec,
            undefined,
            reducedMotion,
            $activeLocale,
            $translation("chart-unavailable"),
            $translation("chart-no-observation"),
          ),
      { notMerge: true },
    );
  }

  function nodeLabel(nodeId: string): string {
    if (spec.kind !== "sankey") return nodeId;
    return spec.nodes.find((node) => node.id === nodeId)?.label ?? nodeId;
  }

  $effect(() => {
    spec;
    $activeLocale;
    $translation;
    refreshChart();
  });

  onMount(() => {
    if (!container) return;
    const narrowLayout = window.matchMedia("(max-width: 560px)");
    const resizeObserver = new ResizeObserver(() => chart?.resize());
    const updateFlowFallback = () => {
      const useFlowLedger = spec.kind === "sankey" && narrowLayout.matches;
      if (spec.kind === "sankey") flowLedgerOpen = useFlowLedger;
      if (useFlowLedger) {
        chart?.dispose();
        chart = undefined;
        return;
      }
      if (!chart) {
        chart = echarts.init(container, undefined, { renderer: "canvas" });
        refreshChart();
      } else {
        chart.resize();
      }
    };
    updateFlowFallback();
    narrowLayout.addEventListener("change", updateFlowFallback);
    resizeObserver.observe(container);
    return () => {
      resizeObserver.disconnect();
      narrowLayout.removeEventListener("change", updateFlowFallback);
      chart?.dispose();
      chart = undefined;
    };
  });
</script>

<article class="chart-card" data-chart-kind={spec.kind}>
  <header>
    <div>
      <span class="eyebrow">{eyebrow}</span>
      <h3>{spec.title}</h3>
    </div>
    <div class="badges" aria-label={$translation("chart-evidence-label")}>
      <span class="badge" data-kind={spec.provenance.kind}
        >{$translation(evidenceKeys[spec.provenance.kind])}</span
      >
      <span class="coverage"
        >{$translation(coverageKeys[spec.provenance.coverage])}</span
      >
    </div>
  </header>
  <p>{spec.description}</p>

  {#if empty}
    <div class="chart-state" style:height={resolvedHeight}>
      {$translation("chart-no-data")}
    </div>
  {:else}
    <div
      bind:this={container}
      class="chart"
      style:height={resolvedHeight}
      role="img"
      aria-label={$translation("chart-accessible-label", {
        title: spec.title,
        description: spec.description,
      })}
    ></div>
    {#if spec.kind === "sankey"}
      <aside class="chart-takeaway">
        <strong>{$translation("chart-sankey-takeaway-label")}</strong>
        <span>{spec.takeaway}</span>
      </aside>
      <details class="flow-ledger" bind:open={flowLedgerOpen}>
        <summary>{$translation("chart-sankey-flow-table")}</summary>
        <div class="flow-table-wrap">
          <table>
            <thead>
              <tr>
                <th scope="col">{$translation("chart-sankey-source")}</th>
                <th scope="col">{$translation("chart-sankey-target")}</th>
                <th scope="col">{$translation("chart-sankey-value")}</th>
                <th scope="col">{$translation("chart-sankey-evidence")}</th>
              </tr>
            </thead>
            <tbody>
              {#each spec.links as link}
                <tr>
                  <td>{nodeLabel(link.source)}</td>
                  <td>{nodeLabel(link.target)}</td>
                  <td>{formatValue(link.value)}</td>
                  <td>
                    {$translation(
                      evidenceKeys[provenanceForLink(spec, link).kind],
                    )}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </details>
      {@const summary = summariseSankey(spec)}
      <div class="screen-reader-summary">
        {$translation("chart-sankey-summary", {
          sources: formatValue(summary.sourceTotal),
          sinks: formatValue(summary.sinkTotal),
          largestSource: summary.largestLink
            ? nodeLabel(summary.largestLink.source)
            : $translation("chart-unavailable"),
          largestTarget: summary.largestLink
            ? nodeLabel(summary.largestLink.target)
            : $translation("chart-unavailable"),
          largestValue: summary.largestLink
            ? formatValue(summary.largestLink.value)
            : $translation("chart-unavailable"),
        })}
      </div>
    {:else}
      <div class="screen-reader-summary">
        {#each spec.series as series}
          <span>
            {$translation("chart-summary-series", {
              label: series.label,
              points: accessiblePointSummary(series.points),
              evidence: $translation(
                evidenceKeys[provenanceForSeries(spec, series).kind],
              ),
              coverage: $translation(
                coverageKeys[provenanceForSeries(spec, series).coverage],
              ),
            })}
          </span>
        {/each}
        {#each spec.reference_lines ?? [] as line}
          <span>
            {$translation("chart-summary-reference", {
              label: line.label,
              value:
                typeof line.value === "number"
                  ? formatValue(line.value)
                  : line.value,
            })}
          </span>
        {/each}
      </div>
    {/if}
  {/if}
  <footer>
    <span>{spec.provenance.source}</span><time
      >{spec.provenance.observed_at}</time
    >
  </footer>
</article>
