import type {
  ChartSpec,
  EvidenceCoverage,
  EvidenceKind,
} from "../charts/types";

export type KpiPreview = {
  label: string;
  value: string;
  change: string;
  context: string;
  kind: EvidenceKind;
  coverage: EvidenceCoverage;
};

export const kpiPreview: KpiPreview[] = [
  {
    label: "Plan attainment",
    value: "92%",
    change: "−3.1 pts to schedule",
    context: "Industrial plan · Year 4 of 5",
    kind: "calculation",
    coverage: "complete",
  },
  {
    label: "External dependency",
    value: "31%",
    change: "−4.7 pts over 180 days",
    context: "Critical-resource basket",
    kind: "estimate",
    coverage: "experimental",
  },
  {
    label: "Demographic resilience",
    value: "+4.8‰",
    change: "+1.2 per 1,000",
    context: "Births + immigration − losses",
    kind: "calculation",
    coverage: "partial",
  },
];

export const planProgressSpec: ChartSpec = {
  schema_version: 1,
  id: "plan-progress-preview",
  title: "Industrial plan progress",
  description:
    "Cumulative, target-normalised progress. The preview shows how actual observations remain separate from the scheduled path.",
  kind: "line",
  category_axis_label: "Plan quarter",
  value_axis_label: "Attainment",
  unit: "%",
  reference_lines: [
    { id: "current", label: "Latest save", axis: "category", value: "Y4 Q2" },
  ],
  series: [
    {
      id: "actual",
      label: "Observed",
      points: [
        { category: "Y1 Q1", value: 7 },
        { category: "Y1 Q2", value: 13 },
        { category: "Y1 Q3", value: 20 },
        { category: "Y1 Q4", value: 27 },
        { category: "Y2 Q1", value: 33 },
        { category: "Y2 Q2", value: 39 },
        { category: "Y2 Q3", value: 46 },
        { category: "Y2 Q4", value: 52 },
        { category: "Y3 Q1", value: 59 },
        { category: "Y3 Q2", value: 64 },
        { category: "Y3 Q3", value: 70 },
        { category: "Y3 Q4", value: 76 },
        { category: "Y4 Q1", value: 81 },
        { category: "Y4 Q2", value: 85 },
      ],
    },
    {
      id: "scheduled",
      label: "Scheduled",
      style: "dashed",
      points: [
        { category: "Y1 Q1", value: 6 },
        { category: "Y1 Q2", value: 12 },
        { category: "Y1 Q3", value: 18 },
        { category: "Y1 Q4", value: 24 },
        { category: "Y2 Q1", value: 30 },
        { category: "Y2 Q2", value: 36 },
        { category: "Y2 Q3", value: 42 },
        { category: "Y2 Q4", value: 48 },
        { category: "Y3 Q1", value: 55 },
        { category: "Y3 Q2", value: 62 },
        { category: "Y3 Q3", value: 69 },
        { category: "Y3 Q4", value: 76 },
        { category: "Y4 Q1", value: 84 },
        { category: "Y4 Q2", value: 92 },
      ],
    },
  ],
  provenance: {
    kind: "calculation",
    source: "Synthetic plan preview · no save connected",
    observed_at: "2004 · day 230",
    coverage: "complete",
  },
};

export const importDependencySpec: ChartSpec = {
  schema_version: 1,
  id: "import-dependency-preview",
  title: "Critical import exposure",
  description:
    "Resource-level recorded reliance, kept separate by material rather than hidden inside one composite score.",
  kind: "bar",
  orientation: "horizontal",
  value_axis_label: "Recorded import share",
  unit: "%",
  reference_lines: [
    { id: "review", label: "Review threshold", axis: "value", value: 50 },
  ],
  series: [
    {
      id: "reliance",
      label: "Import share",
      points: [
        { category: "Electronic components", value: 78 },
        { category: "Chemicals", value: 61 },
        { category: "Steel", value: 44 },
        { category: "Fabric", value: 33 },
        { category: "Fuel", value: 17 },
        { category: "Crops", value: 8 },
      ],
    },
  ],
  provenance: {
    kind: "estimate",
    source: "Synthetic material basket · production coverage illustrative",
    observed_at: "2004 · day 230",
    coverage: "experimental",
  },
};

export type MaterialCell = {
  code: string;
  name: string;
  family:
    "Raw" | "Industrial" | "Construction" | "Consumer" | "Energy" | "Waste";
  value: string;
  delta: string;
  reliance: number;
  status: "stable" | "watch" | "exposed";
  note: string;
};

export const materialCells: MaterialCell[] = [
  {
    code: "Crp",
    name: "Crops",
    family: "Raw",
    value: "8%",
    delta: "−2.1",
    reliance: 8,
    status: "stable",
    note: "Domestic supply covers most recorded demand.",
  },
  {
    code: "Wd",
    name: "Wood",
    family: "Raw",
    value: "0%",
    delta: "0.0",
    reliance: 0,
    status: "stable",
    note: "No recorded imports in the selected window.",
  },
  {
    code: "Ol",
    name: "Oil",
    family: "Raw",
    value: "3%",
    delta: "−0.7",
    reliance: 3,
    status: "stable",
    note: "Low external exposure in the preview basket.",
  },
  {
    code: "Ch",
    name: "Chemicals",
    family: "Industrial",
    value: "61%",
    delta: "+8.4",
    reliance: 61,
    status: "exposed",
    note: "Largest worsening critical-material exposure.",
  },
  {
    code: "Mec",
    name: "Mechanical parts",
    family: "Industrial",
    value: "29%",
    delta: "−3.2",
    reliance: 29,
    status: "watch",
    note: "Improving, but still material to vehicle production.",
  },
  {
    code: "Ecp",
    name: "Electronic parts",
    family: "Industrial",
    value: "78%",
    delta: "+2.6",
    reliance: 78,
    status: "exposed",
    note: "Highest recorded external dependency.",
  },
  {
    code: "St",
    name: "Steel",
    family: "Construction",
    value: "44%",
    delta: "−5.3",
    reliance: 44,
    status: "watch",
    note: "Construction demand remains the main recorded use.",
  },
  {
    code: "Br",
    name: "Bricks",
    family: "Construction",
    value: "11%",
    delta: "−1.8",
    reliance: 11,
    status: "stable",
    note: "A low but non-zero import bridge remains.",
  },
  {
    code: "Pf",
    name: "Prefab panels",
    family: "Construction",
    value: "23%",
    delta: "+1.2",
    reliance: 23,
    status: "watch",
    note: "Exposure rose during the current building pulse.",
  },
  {
    code: "Fd",
    name: "Food",
    family: "Consumer",
    value: "6%",
    delta: "−0.9",
    reliance: 6,
    status: "stable",
    note: "Small imports provide a consumer reserve.",
  },
  {
    code: "Cl",
    name: "Clothes",
    family: "Consumer",
    value: "14%",
    delta: "+0.4",
    reliance: 14,
    status: "stable",
    note: "Stable recorded reliance.",
  },
  {
    code: "El",
    name: "Electronics",
    family: "Consumer",
    value: "39%",
    delta: "+4.1",
    reliance: 39,
    status: "watch",
    note: "Consumer supply inherits upstream component risk.",
  },
  {
    code: "Fl",
    name: "Fuel",
    family: "Energy",
    value: "17%",
    delta: "−6.0",
    reliance: 17,
    status: "stable",
    note: "Exposure fell across the recent observation window.",
  },
  {
    code: "Pw",
    name: "Power",
    family: "Energy",
    value: "4%",
    delta: "0.0",
    reliance: 4,
    status: "stable",
    note: "External supply is a small recorded share.",
  },
  {
    code: "Mw",
    name: "Mixed waste",
    family: "Waste",
    value: "1.9 kt",
    delta: "+5.7",
    reliance: 0,
    status: "watch",
    note: "Factory generation is the leading source in the preview.",
  },
  {
    code: "Hw",
    name: "Hazardous waste",
    family: "Waste",
    value: "84 t",
    delta: "+12.0",
    reliance: 0,
    status: "exposed",
    note: "Growth exceeds the illustrative baseline range.",
  },
];

export const attentionPreview = [
  {
    level: "Signal",
    title: "Chemical exposure moved outside baseline",
    detail: "+8.4 points across the latest 180-day comparison.",
  },
  {
    level: "Plan",
    title: "Industrial plan is 3.1 points behind schedule",
    detail: "The shortfall widened in the last two plan quarters.",
  },
  {
    level: "Coverage",
    title: "Production evidence remains partial",
    detail:
      "Dependency values are experimental until resource coverage is validated.",
  },
];
