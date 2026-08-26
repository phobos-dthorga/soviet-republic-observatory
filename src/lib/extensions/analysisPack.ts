export const ANALYSIS_PACK_SCHEMA_VERSION = 1 as const;
export const ANALYSIS_PACK_HOST_API_VERSION = 1 as const;

export const RECEIVER_CORE_METRICS = [
  "core.citizens.electronics.none",
  "core.citizens.electronics.radio",
  "core.citizens.electronics.television",
  "core.citizens.electronics.computer",
] as const;

export type MetricReference =
  { core_metric: string } | { derived_metric: string };

export type AnalysisOperation =
  | { kind: "sum" | "product"; operands: MetricReference[] }
  | {
      kind: "difference";
      minuend: MetricReference;
      subtrahend: MetricReference;
    }
  | {
      kind: "safe_ratio";
      numerator: MetricReference;
      denominator: MetricReference;
      scale?: number;
    }
  | { kind: "scale"; operand: MetricReference; factor: number };

export type DerivedMetricDeclaration = {
  id: string;
  label: string;
  unit: string;
  description?: string;
  operation: AnalysisOperation;
};

export type AnalysisChartTemplate = {
  schema_version: 1;
  id: string;
  title: string;
  description: string;
  kind: "line" | "area" | "bar";
  orientation?: "vertical" | "horizontal";
  category_axis_label?: string;
  value_axis_label?: string;
  unit?: string;
  value_domain?: { min: number; max: number };
  series: Array<{
    id: string;
    label: string;
    metric: MetricReference;
    style?: "solid" | "dashed";
    stack_id?: string;
  }>;
};

export type AnalysisPack = {
  schema_version: 1;
  id: string;
  version: string;
  host_api_version: 1;
  name: string;
  author: string;
  description: string;
  derived_metrics: DerivedMetricDeclaration[];
  charts: AnalysisChartTemplate[];
};

export type AnalysisPackSemanticIssue = {
  code:
    | "duplicate_derived_metric"
    | "duplicate_chart"
    | "duplicate_series"
    | "forward_or_unknown_derived_metric"
    | "unknown_core_metric"
    | "invalid_chart_domain";
  path: string;
  message: string;
};

function referencesForOperation(
  operation: AnalysisOperation,
): MetricReference[] {
  switch (operation.kind) {
    case "sum":
    case "product":
      return operation.operands;
    case "difference":
      return [operation.minuend, operation.subtrahend];
    case "safe_ratio":
      return [operation.numerator, operation.denominator];
    case "scale":
      return [operation.operand];
  }
}

function validateReference(
  reference: MetricReference,
  path: string,
  availableCoreMetrics: ReadonlySet<string>,
  availableDerivedMetrics: ReadonlySet<string>,
  issues: AnalysisPackSemanticIssue[],
): void {
  if ("core_metric" in reference) {
    if (!availableCoreMetrics.has(reference.core_metric)) {
      issues.push({
        code: "unknown_core_metric",
        path,
        message: `Core metric ${reference.core_metric} is not published by this host API.`,
      });
    }
    return;
  }

  if (!availableDerivedMetrics.has(reference.derived_metric)) {
    issues.push({
      code: "forward_or_unknown_derived_metric",
      path,
      message: `Derived metric ${reference.derived_metric} must be declared earlier in this pack.`,
    });
  }
}

export function validateAnalysisPackSemantics(
  pack: AnalysisPack,
  availableCoreMetrics: ReadonlySet<string>,
): AnalysisPackSemanticIssue[] {
  const issues: AnalysisPackSemanticIssue[] = [];
  const priorDerivedMetrics = new Set<string>();

  pack.derived_metrics.forEach((metric, metricIndex) => {
    if (priorDerivedMetrics.has(metric.id)) {
      issues.push({
        code: "duplicate_derived_metric",
        path: `/derived_metrics/${metricIndex}/id`,
        message: `Derived metric ID ${metric.id} is duplicated.`,
      });
    }

    referencesForOperation(metric.operation).forEach(
      (reference, referenceIndex) =>
        validateReference(
          reference,
          `/derived_metrics/${metricIndex}/operation/references/${referenceIndex}`,
          availableCoreMetrics,
          priorDerivedMetrics,
          issues,
        ),
    );

    priorDerivedMetrics.add(metric.id);
  });

  const chartIds = new Set<string>();
  pack.charts.forEach((chart, chartIndex) => {
    if (chartIds.has(chart.id)) {
      issues.push({
        code: "duplicate_chart",
        path: `/charts/${chartIndex}/id`,
        message: `Chart ID ${chart.id} is duplicated.`,
      });
    }
    chartIds.add(chart.id);

    if (
      chart.value_domain &&
      chart.value_domain.min >= chart.value_domain.max
    ) {
      issues.push({
        code: "invalid_chart_domain",
        path: `/charts/${chartIndex}/value_domain`,
        message: "Chart value-domain minimum must be less than its maximum.",
      });
    }

    const seriesIds = new Set<string>();
    chart.series.forEach((series, seriesIndex) => {
      if (seriesIds.has(series.id)) {
        issues.push({
          code: "duplicate_series",
          path: `/charts/${chartIndex}/series/${seriesIndex}/id`,
          message: `Series ID ${series.id} is duplicated within chart ${chart.id}.`,
        });
      }
      seriesIds.add(series.id);
      validateReference(
        series.metric,
        `/charts/${chartIndex}/series/${seriesIndex}/metric`,
        availableCoreMetrics,
        priorDerivedMetrics,
        issues,
      );
    });
  });

  return issues;
}

export type MetricValueLookup = (
  reference: MetricReference,
) => number | null | undefined;

function finiteValue(
  reference: MetricReference,
  lookup: MetricValueLookup,
): number | null {
  const value = lookup(reference);
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function evaluateAnalysisOperation(
  operation: AnalysisOperation,
  lookup: MetricValueLookup,
): number | null {
  const values = referencesForOperation(operation).map((reference) =>
    finiteValue(reference, lookup),
  );
  if (values.some((value) => value === null)) return null;
  const finiteValues = values as number[];

  switch (operation.kind) {
    case "sum":
      return finiteValues.reduce((total, value) => total + value, 0);
    case "difference":
      return finiteValues[0] - finiteValues[1];
    case "product":
      return finiteValues.reduce((total, value) => total * value, 1);
    case "safe_ratio":
      return finiteValues[1] === 0
        ? null
        : (finiteValues[0] / finiteValues[1]) * (operation.scale ?? 1);
    case "scale":
      return finiteValues[0] * operation.factor;
  }
}
