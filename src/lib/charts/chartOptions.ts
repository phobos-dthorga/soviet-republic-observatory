import type { EChartsCoreOption } from "echarts/core";
import { formatNumber } from "../i18n/format";
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

export function expandedCategories(
  points: ChartPoint[],
  noObservationLabel = "no observation",
): string[] {
  return points.flatMap((point) =>
    point.gap_before
      ? [`${point.category} · ${noObservationLabel}`, point.category]
      : [point.category],
  );
}

export function expandedValues(points: ChartPoint[]): Array<number | null> {
  return points.flatMap((point) =>
    point.gap_before ? [null, point.value] : [point.value],
  );
}

export type PositionedChartValue = {
  name: string;
  value: [number, number | null];
};

export function expandedGameDayValues(
  points: ChartPoint[],
  noObservationLabel = "no observation",
): PositionedChartValue[] {
  return points.flatMap((point, index) => {
    if (point.category_value === undefined) return [];
    const current = {
      name: point.category,
      value: [point.category_value, point.value] as [number, number],
    };
    if (!point.gap_before) return [current];
    const previous = points[index - 1]?.category_value;
    const gapPosition =
      previous === undefined
        ? point.category_value - 0.5
        : previous + (point.category_value - previous) / 2;
    return [
      {
        name: `${point.category} · ${noObservationLabel}`,
        value: [gapPosition, null] as [number, null],
      },
      current,
    ];
  });
}

export function formatGameDayValue(value: number): string {
  const wholeDay = Math.round(value);
  const year = Math.floor(wholeDay / 365);
  const day = wholeDay - year * 365;
  return `${year} · ${String(day).padStart(3, "0")}`;
}

export type CondensedSeriesSummary = {
  count: number;
  first: ChartPoint;
  minimum: ChartPoint;
  maximum: ChartPoint;
  latest: ChartPoint;
  gapCount: number;
};

export function condensedSeriesSummary(
  points: ChartPoint[],
  threshold = 24,
): CondensedSeriesSummary | null {
  if (points.length <= threshold || points.length === 0) return null;

  let minimum = points[0];
  let maximum = points[0];
  let gapCount = 0;
  for (const point of points) {
    if (point.value < minimum.value) minimum = point;
    if (point.value > maximum.value) maximum = point;
    if (point.gap_before) gapCount += 1;
  }

  return {
    count: points.length,
    first: points[0],
    minimum,
    maximum,
    latest: points.at(-1)!,
    gapCount,
  };
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
  locale = "en-AU",
  unavailableLabel = "Unavailable",
  noObservationLabel = "no observation",
): EChartsCoreOption {
  const categories = expandedCategories(
    spec.series[0]?.points ?? [],
    noObservationLabel,
  );
  const horizontal = spec.kind === "bar" && spec.orientation === "horizontal";
  const positionedGameDays =
    spec.category_axis_scale === "game_day" && !horizontal;
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
  const gameDayAxis = {
    type: "value",
    name: spec.category_axis_label,
    nameLocation: "middle",
    nameGap: 31,
    boundaryGap: false,
    axisLine: { lineStyle: { color: theme.line } },
    axisTick: { show: false },
    splitLine: { show: false },
    axisLabel: {
      color: theme.muted,
      fontSize: 10,
      formatter: (value: number) => formatGameDayValue(value),
    },
    axisPointer: {
      label: {
        formatter: ({ value }: { value: number }) => formatGameDayValue(value),
      },
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
      valueFormatter: (rawValue: unknown) => {
        const value = Array.isArray(rawValue) ? rawValue[1] : rawValue;
        return typeof value === "number"
          ? `${formatNumber(value, locale, { maximumFractionDigits: 2 })}${spec.unit ? ` ${spec.unit}` : ""}`
          : String(value ?? unavailableLabel);
      },
    },
    xAxis: horizontal
      ? valueAxis
      : positionedGameDays
        ? gameDayAxis
        : categoryAxis,
    yAxis: horizontal ? categoryAxis : valueAxis,
    series: spec.series.map((series, index) => ({
      id: series.id,
      name: series.label,
      type: spec.kind === "bar" ? "bar" : "line",
      stack: series.stack_id,
      data: positionedGameDays
        ? expandedGameDayValues(series.points, noObservationLabel)
        : expandedValues(series.points),
      connectNulls: false,
      smooth: false,
      symbol: spec.kind === "bar" ? undefined : "circle",
      showSymbol: spec.kind === "bar" ? undefined : series.points.length <= 120,
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
              label: {
                color: theme.muted,
                formatter: "{b}",
                position: "insideStartTop",
                rotate: 0,
              },
              lineStyle: { color: theme.muted, type: "dashed", width: 1 },
              data: spec.reference_lines.map((line) => ({
                id: line.id,
                name: line.label,
                label: { position: "insideStartTop", rotate: 0 },
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
