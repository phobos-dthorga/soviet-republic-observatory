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
  import { activeTheme } from "../theme/runtime";
  import {
    condensedSeriesSummary,
    optionForChart,
    provenanceForSeries,
    sourcePointIndex,
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
  import { chartThemeFor } from "./themeAdapter";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import type { ContextHelpContent } from "../ui/types";
  import type {
    ChartNavigationBinding,
    RelatedDataDestination,
  } from "../navigation/relatedData";

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
    help = null,
    navigation = [],
    onrelatednavigate,
  }: {
    spec: ObservatoryChartSpec;
    height?: string;
    eyebrow: string;
    help?: ContextHelpContent | null;
    navigation?: ChartNavigationBinding[];
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
  } = $props();
  let container = $state<HTMLDivElement>();
  let chart = $state.raw<ECharts>();
  let flowLedgerOpen = $state(false);
  let chartDataOpen = $state(false);
  let chartDataPage = $state(0);
  const chartDataPageSize = 50;
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

  const chartDataRows = $derived.by(() =>
    spec.kind === "sankey"
      ? []
      : spec.series.flatMap((series) =>
          series.points.map((point, pointIndex) => ({
            id: `${series.id}:${pointIndex}`,
            seriesId: series.id,
            pointIndex,
            seriesLabel: series.label,
            point,
            destinations: destinationsFor(series.id, pointIndex),
          })),
        ),
  );
  const chartDataPageCount = $derived(
    Math.max(1, Math.ceil(chartDataRows.length / chartDataPageSize)),
  );
  const visibleChartDataRows = $derived(
    chartDataRows.slice(
      chartDataPage * chartDataPageSize,
      (chartDataPage + 1) * chartDataPageSize,
    ),
  );

  function destinationsFor(
    seriesId: string,
    pointIndex: number,
  ): RelatedDataDestination[] {
    return (
      navigation.find(
        (binding) =>
          binding.seriesId === seriesId && binding.pointIndex === pointIndex,
      )?.destinations ?? []
    );
  }

  function requestRelatedView(
    destinations: RelatedDataDestination[],
    origin: HTMLElement | null,
  ): void {
    if (destinations.length === 0) return;
    onrelatednavigate?.(destinations, origin);
  }

  function handleChartClick(event: {
    seriesId?: string;
    dataIndex?: number;
    dataType?: string;
  }): void {
    if (!event.seriesId || typeof event.dataIndex !== "number") return;
    if (spec.kind === "sankey") {
      if (event.dataType !== "edge") return;
      requestRelatedView(
        destinationsFor(event.seriesId, event.dataIndex),
        container ?? null,
      );
      return;
    }
    const series = spec.series.find((item) => item.id === event.seriesId);
    if (!series) return;
    const pointIndex = sourcePointIndex(series.points, event.dataIndex);
    if (pointIndex === null) return;
    requestRelatedView(
      destinationsFor(event.seriesId, pointIndex),
      container ?? null,
    );
  }

  function initialiseChart(): void {
    if (!container || chart) return;
    chart = echarts.init(container, undefined, { renderer: "canvas" });
    chart.on("click", handleChartClick);
    refreshChart();
  }

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
        ? optionForSankey(
            spec,
            chartThemeFor($activeTheme),
            reducedMotion,
            $activeLocale,
          )
        : optionForChart(
            spec,
            chartThemeFor($activeTheme),
            reducedMotion,
            $activeLocale,
            $translation("chart-unavailable"),
            $translation("chart-no-observation"),
            navigation,
            $translation("related-nav-open"),
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
    $activeTheme;
    $translation;
    refreshChart();
  });

  $effect(() => {
    chartDataRows.length;
    chartDataPage = 0;
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
        initialiseChart();
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
      {#if help}
        <ContextHelp
          topic={help.topic}
          title={help.title}
          text={help.text}
          details={help.details}
          placement="left"
        />
      {/if}
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
                {#if navigation.length > 0}
                  <th scope="col">{$translation("chart-related-view")}</th>
                {/if}
              </tr>
            </thead>
            <tbody>
              {#each spec.links as link, linkIndex}
                {@const destinations = destinationsFor(spec.id, linkIndex)}
                <tr>
                  <td>{nodeLabel(link.source)}</td>
                  <td>{nodeLabel(link.target)}</td>
                  <td>{formatValue(link.value)}</td>
                  <td>
                    {$translation(
                      evidenceKeys[provenanceForLink(spec, link).kind],
                    )}
                  </td>
                  {#if navigation.length > 0}
                    <td>
                      {#if destinations.length > 0}
                        <button
                          id={`chart-related-${spec.id}-${link.id}`}
                          class="table-link"
                          onclick={(event) =>
                            requestRelatedView(
                              destinations,
                              event.currentTarget,
                            )}>{$translation("related-nav-open")}</button
                        >
                      {:else}
                        <span class="unavailable"
                          >{$translation("chart-no-related-view")}</span
                        >
                      {/if}
                    </td>
                  {/if}
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
      <details class="flow-ledger chart-data-ledger" bind:open={chartDataOpen}>
        <summary>{$translation("chart-data-open")}</summary>
        <p>{$translation("chart-data-description")}</p>
        <div class="flow-table-wrap">
          <table>
            <thead>
              <tr>
                <th scope="col">{$translation("chart-data-point")}</th>
                <th scope="col">{$translation("chart-data-series")}</th>
                <th scope="col">{$translation("chart-data-value")}</th>
                <th scope="col">{$translation("chart-related-view")}</th>
              </tr>
            </thead>
            <tbody>
              {#each visibleChartDataRows as row (row.id)}
                <tr>
                  <td>{row.point.category}</td>
                  <td>{row.seriesLabel}</td>
                  <td>{formatValue(row.point.value)}</td>
                  <td>
                    {#if row.destinations.length > 0}
                      <button
                        id={`chart-related-${spec.id}-${row.seriesId}-${row.pointIndex}`}
                        class="table-link"
                        onclick={(event) =>
                          requestRelatedView(
                            row.destinations,
                            event.currentTarget,
                          )}>{$translation("related-nav-open")}</button
                      >
                    {:else}
                      <span class="unavailable"
                        >{$translation("chart-no-related-view")}</span
                      >
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if chartDataPageCount > 1}
          <nav
            class="chart-data-pages"
            aria-label={$translation("chart-data-pages")}
          >
            <button
              disabled={chartDataPage === 0}
              onclick={() => (chartDataPage -= 1)}
              >{$translation("chart-data-previous")}</button
            >
            <span>
              {$translation("chart-data-page", {
                page: chartDataPage + 1,
                pages: chartDataPageCount,
              })}
            </span>
            <button
              disabled={chartDataPage + 1 >= chartDataPageCount}
              onclick={() => (chartDataPage += 1)}
              >{$translation("chart-data-next")}</button
            >
          </nav>
        {/if}
      </details>
    {/if}
  {/if}
  <footer>
    <span>{spec.provenance.source}</span><time
      >{spec.provenance.observed_at}</time
    >
  </footer>
</article>
