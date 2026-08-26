import type { EChartsCoreOption } from "echarts/core";
import type {
  ChartPoint,
  ChartSeries,
  ChartSpec,
  ChartTheme,
  Provenance,
} from "./types";

export const observatoryChartTheme: ChartTheme = {
  palette: ["#80c6d8", "#d8b86a", "#d88474", "#8da6c9", "#b6a8ce"],
  text: "#edf2f5",
  muted: "#98a8b2",
  line: "rgba(139, 159, 171, 0.18)",
  tooltipBackground: "rgba(9, 16, 24, 0.97)",
  tooltipBorder: "rgba(139, 159, 171, 0.32)",
};

export function expandedCategories(points: ChartPoint[]): string[] {
  return points.flatMap((point) =>
    point.gap_before
      ? [`${point.category} · no observation`, point.category]
      : [point.category],
  );
}

export function expandedValues(points: ChartPoint[]): Array<number | null> {
  return points.flatMap((point) =>
    point.gap_before ? [null, point.value] : [point.value],
  );
}

export function provenanceForSeries(
  spec: ChartSpec,
  series: ChartSeries,
): Provenance {
  return series.provenance ?? spec.provenance;
}

export function optionForChart(
  spec: ChartSpec,
  theme: ChartTheme = observatoryChartTheme,
  reducedMotion = false,
): EChartsCoreOption {
  const categories = expandedCategories(spec.series[0]?.points ?? []);
  const horizontal = spec.kind === "bar" && spec.orientation === "horizontal";
  const categoryAxis = {
    type: "category",
    name: horizontal ? undefined : spec.category_axis_label,
    nameLocation: "middle",
    nameGap: 31,
    data: categories,
    boundaryGap: spec.kind === "bar",
    inverse: horizontal,
    axisLine: { lineStyle: { color: theme.line } },
    axisTick: { show: false },
    axisLabel: {
      color: theme.muted,
      interval: spec.kind === "bar" ? 0 : "auto",
      fontSize: 10,
    },
    nameTextStyle: { color: theme.muted, fontSize: 10 },
  };
  const valueAxis = {
    type: "value",
    min: spec.value_domain?.min,
    max: spec.value_domain?.max,
    name: spec.value_axis_label,
    nameLocation: horizontal ? "middle" : "end",
    nameGap: horizontal ? 38 : 12,
    nameTextStyle: { color: theme.muted, fontSize: 10 },
    splitLine: { lineStyle: { color: theme.line } },
    axisLine: { show: false },
    axisLabel: { color: theme.muted, fontSize: 10 },
  };

  return {
    animationDuration: reducedMotion ? 0 : 420,
    color: theme.palette,
    grid: {
      left: horizontal ? 112 : 44,
      right: horizontal ? 22 : 16,
      top: spec.series.length > 1 ? 32 : 14,
      bottom: horizontal ? 38 : 46,
      containLabel: false,
    },
    legend: {
      show: spec.series.length > 1,
      top: 0,
      right: 0,
      itemWidth: 16,
      itemHeight: 3,
      textStyle: { color: theme.muted, fontSize: 10 },
    },
    tooltip: {
      trigger: "axis",
      backgroundColor: theme.tooltipBackground,
      borderColor: theme.tooltipBorder,
      textStyle: { color: theme.text },
      valueFormatter: (value: unknown) =>
        typeof value === "number"
          ? `${new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(value)}${spec.unit ? ` ${spec.unit}` : ""}`
          : String(value ?? "Unavailable"),
    },
    xAxis: horizontal ? valueAxis : categoryAxis,
    yAxis: horizontal ? categoryAxis : valueAxis,
    series: spec.series.map((series, index) => ({
      id: series.id,
      name: series.label,
      type: spec.kind === "bar" ? "bar" : "line",
      stack: series.stack_id,
      data: expandedValues(series.points),
      connectNulls: false,
      smooth: false,
      symbol: spec.kind === "bar" ? undefined : "circle",
      symbolSize: 5,
      barMaxWidth: 14,
      lineStyle: {
        width: 2,
        type: series.style === "dashed" ? "dashed" : "solid",
      },
      areaStyle: spec.kind === "area" ? { opacity: 0.13 } : undefined,
      markLine:
        index === 0 && spec.reference_lines?.length
          ? {
              silent: true,
              symbol: ["none", "none"],
              label: { color: theme.muted, formatter: "{b}" },
              lineStyle: { color: theme.muted, type: "dashed", width: 1 },
              data: spec.reference_lines.map((line) => ({
                id: line.id,
                name: line.label,
                ...(line.axis === "category"
                  ? horizontal
                    ? { yAxis: line.value }
                    : { xAxis: line.value }
                  : horizontal
                    ? { xAxis: line.value }
                    : { yAxis: line.value }),
              })),
            }
          : undefined,
    })),
  };
}
