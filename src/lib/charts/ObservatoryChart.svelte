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
  import { optionForChart, provenanceForSeries } from "./chartOptions";
  import type { ChartSpec } from "./types";

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
    eyebrow = "Planning instrument",
  }: {
    spec: ChartSpec;
    height?: string;
    eyebrow?: string;
  } = $props();

  let container = $state<HTMLDivElement>();
  let chart = $state.raw<ECharts>();
  const resolvedHeight = $derived(
    spec.kind === "bar" && spec.orientation === "horizontal"
      ? `${Math.max(220, spec.series[0]?.points.length * 29 + 82)}px`
      : height,
  );

  const numberFormatter = new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 2,
  });

  function formatValue(value: number): string {
    return `${numberFormatter.format(value)}${spec.unit ? ` ${spec.unit}` : ""}`;
  }

  $effect(() => {
    if (!chart) return;
    const reducedMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    chart.setOption(optionForChart(spec, undefined, reducedMotion), {
      notMerge: true,
    });
  });

  onMount(() => {
    if (!container) return;

    const reducedMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    chart = echarts.init(container, undefined, { renderer: "canvas" });
    chart.setOption(optionForChart(spec, undefined, reducedMotion));

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
    <div class="badges" aria-label="Chart evidence">
      <span class="badge" data-kind={spec.provenance.kind}>
        {spec.provenance.kind.replaceAll("_", " ")}
      </span>
      <span class="coverage">{spec.provenance.coverage}</span>
    </div>
  </header>

  <p>{spec.description}</p>

  {#if spec.series.length === 0}
    <div class="chart-state" style:height={resolvedHeight}>
      No data available
    </div>
  {:else}
    <div
      bind:this={container}
      class="chart"
      style:height={resolvedHeight}
      role="img"
      aria-label={`${spec.title}. ${spec.description}`}
    ></div>
    <div class="screen-reader-summary">
      {#each spec.series as series}
        {series.label}: {series.points
          .map(
            (point) =>
              `${point.gap_before ? "gap before, " : ""}${point.category}, ${formatValue(point.value)}`,
          )
          .join("; ")}. Evidence: {provenanceForSeries(
          spec,
          series,
        ).kind.replaceAll("_", " ")}, {provenanceForSeries(spec, series)
          .coverage} coverage.
      {/each}
      {#each spec.reference_lines ?? [] as line}
        {line.label}: {typeof line.value === "number"
          ? formatValue(line.value)
          : line.value}.
      {/each}
    </div>
  {/if}

  <footer>
    <span>{spec.provenance.source}</span>
    <time datetime={spec.provenance.observed_at}
      >{spec.provenance.observed_at}</time
    >
  </footer>
</article>
