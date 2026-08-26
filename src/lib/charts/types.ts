export const CHART_SCHEMA_VERSION = 1 as const;

export type EvidenceKind =
  | "save_fact"
  | "game_definition"
  | "calculation"
  | "extension_calculation"
  | "estimate"
  | "recommendation";

export type EvidenceCoverage = "complete" | "partial" | "experimental";

export type Provenance = {
  kind: EvidenceKind;
  source: string;
  observed_at: string;
  coverage: EvidenceCoverage;
};

export type ChartPoint = {
  category: string;
  value: number;
  gap_before?: boolean;
};

export type ChartSeries = {
  id: string;
  label: string;
  style?: "solid" | "dashed";
  stack_id?: string;
  provenance?: Provenance;
  points: ChartPoint[];
};

export type ChartReferenceLine = {
  id: string;
  label: string;
  axis: "category" | "value";
  value: string | number;
};

export type ChartSpec = {
  schema_version: typeof CHART_SCHEMA_VERSION;
  id: string;
  title: string;
  description: string;
  kind: "line" | "area" | "bar";
  orientation?: "vertical" | "horizontal";
  category_axis_label?: string;
  value_axis_label?: string;
  unit?: string;
  value_domain?: {
    min: number;
    max: number;
  };
  reference_lines?: ChartReferenceLine[];
  series: ChartSeries[];
  provenance: Provenance;
};

export type ChartTheme = {
  palette: string[];
  text: string;
  muted: string;
  line: string;
  tooltipBackground: string;
  tooltipBorder: string;
};
