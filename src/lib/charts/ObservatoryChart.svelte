<script lang="ts">
  import { BarChart, LineChart } from "echarts/charts";
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
  import type {
    ChartPoint,
    ChartSpec,
    EvidenceCoverage,
    EvidenceKind,
  } from "./types";

  echarts.use([
    BarChart,
    LineChart,
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
  }: { spec: ChartSpec; height?: string; eyebrow: string } = $props();
  let container = $state<HTMLDivElement>();
  let chart = $state.raw<ECharts>();
  const resolvedHeight = $derived(
    spec.kind === "bar" && spec.orientation === "horizontal"
      ? `${Math.max(220, spec.series[0]?.points.length * 29 + 82)}px`
      : height,
  );
  const evidenceKeys: Record<EvidenceKind, TranslationKey> = {
    save_fact: "evidence-save-fact",
    game_definition: "evidence-game-definition",
    calculation: "evidence-calculation",
    extension_calculation: "evidence-extension-calculation",
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
      optionForChart(
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

  $effect(() => {
    spec;
    $activeLocale;
    $translation;
    refreshChart();
  });

  onMount(() => {
    if (!container) return;
    chart = echarts.init(container, undefined, { renderer: "canvas" });
    refreshChart();
    const resizeObserver = new ResizeObserver(() => chart?.resize());
    resizeObserver.observe(container);
    return () => {
      resizeObserver.disconnect();
      chart?.dispose();
      chart = undefined;
    };
  });
</script>

<article class="chart-card">
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

  {#if spec.series.length === 0}
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
  <footer>
    <span>{spec.provenance.source}</span><time
      >{spec.provenance.observed_at}</time
    >
  </footer>
</article>
