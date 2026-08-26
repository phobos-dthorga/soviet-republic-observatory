import type { ChartSpec } from "../charts/types";

const dates = [
  "Y3 D050",
  "Y3 D090",
  "Y3 D130",
  "Y3 D170",
  "Y3 D210",
  "Y3 D250",
  "Y3 D290",
  "Y3 D330",
  "Y4 D010",
  "Y4 D050",
];

function points(values: number[]) {
  return dates.map((category, index) => ({ category, value: values[index] }));
}

export const receiverLadderSpec: ChartSpec = {
  schema_version: 1,
  id: "receiver-ladder-preview",
  title: "Receiver-class composition",
  description:
    "Synthetic 100% composition of citizens classified as having no receiver, radio, television, or computer. The denominator is the four recorded classes, not total population.",
  kind: "area",
  category_axis_label: "Synthetic observation",
  value_axis_label: "Classified population share",
  unit: "%",
  value_domain: { min: 0, max: 100 },
  series: [
    {
      id: "none",
      label: "No receiver",
      stack_id: "receiver_classes",
      points: points([28, 26, 24, 22, 20, 18, 16, 14, 12, 10]),
    },
    {
      id: "radio",
      label: "Radio",
      stack_id: "receiver_classes",
      points: points([42, 41, 39, 37, 35, 33, 31, 29, 27, 25]),
    },
    {
      id: "television",
      label: "Television",
      stack_id: "receiver_classes",
      points: points([25, 27, 29, 31, 33, 35, 37, 39, 42, 45]),
    },
    {
      id: "computer",
      label: "Computer",
      stack_id: "receiver_classes",
      points: points([5, 6, 8, 10, 12, 14, 16, 18, 19, 20]),
    },
  ],
  provenance: {
    kind: "extension_calculation",
    source:
      "Synthetic mirror of Receiver Adoption Laboratory · pack not loaded",
    observed_at: "Preview branch · Y4 D050",
    coverage: "experimental",
  },
};

export const audienceReachSpec: ChartSpec = {
  schema_version: 1,
  id: "audience-reach-preview",
  title: "Potential and current audience",
  description:
    "Illustrative radio and television reach. Station audience telemetry is not decoded and remains a binary-research candidate.",
  kind: "line",
  category_axis_label: "Synthetic observation",
  value_axis_label: "Citizens",
  unit: "citizens",
  series: [
    {
      id: "radio-potential",
      label: "Radio potential",
      style: "dashed",
      points: points([
        4200, 4420, 4600, 4800, 5050, 5300, 5480, 5630, 5790, 5960,
      ]),
    },
    {
      id: "radio-current",
      label: "Radio current",
      points: points([
        2520, 2740, 2870, 3010, 3220, 3450, 3510, 3690, 3770, 3910,
      ]),
    },
    {
      id: "tv-potential",
      label: "TV potential",
      style: "dashed",
      points: points([
        2450, 2680, 2920, 3210, 3480, 3810, 4090, 4380, 4700, 5030,
      ]),
    },
    {
      id: "tv-current",
      label: "TV current",
      points: points([
        970, 1120, 1290, 1510, 1740, 2030, 2290, 2510, 2810, 3140,
      ]),
    },
  ],
  provenance: {
    kind: "estimate",
    source: "Synthetic interface concept · binary station evidence unavailable",
    observed_at: "Preview branch · Y4 D050",
    coverage: "experimental",
  },
};

export const influenceAssaySpec: ChartSpec = {
  schema_version: 1,
  id: "influence-assay-preview",
  title: "Expected influence assay",
  description:
    "Signed, illustrative effect directions for the selected programming formulation. These are neither calibrated game coefficients nor causal findings.",
  kind: "bar",
  orientation: "horizontal",
  value_axis_label: "Expected directional effect",
  unit: "index pts",
  value_domain: { min: -4, max: 4 },
  reference_lines: [
    { id: "neutral", label: "No expected effect", axis: "value", value: 0 },
  ],
  series: [
    {
      id: "expected-effect",
      label: "Expected effect",
      points: [
        { category: "Government loyalty", value: 3.2 },
        { category: "Education", value: 1.5 },
        { category: "Culture enjoyment", value: 1.1 },
        { category: "Happiness", value: 0.7 },
        { category: "Alcohol addiction", value: -0.6 },
        { category: "Criminality", value: -1.3 },
        { category: "Religion sympathy", value: -2.1 },
      ],
    },
  ],
  provenance: {
    kind: "estimate",
    source: "Synthetic uncalibrated influence model",
    observed_at: "Preview branch · Y4 D050",
    coverage: "experimental",
  },
};

export const outcomeLaboratorySpec: ChartSpec = {
  schema_version: 1,
  id: "broadcast-outcomes-preview",
  title: "Outcomes around a programming change",
  description:
    "Annotated trends around a synthetic broadcast change. Movement after the event is an association for investigation, not proof that broadcasting caused it.",
  kind: "line",
  category_axis_label: "Synthetic observation",
  value_axis_label: "Citizen status",
  unit: "%",
  value_domain: { min: 60, max: 90 },
  reference_lines: [
    {
      id: "programme-change",
      label: "New evening schedule",
      axis: "category",
      value: "Y3 D250",
    },
  ],
  series: [
    {
      id: "loyalty",
      label: "Government loyalty",
      points: points([
        68.1, 68.4, 68.2, 68.7, 69.0, 69.2, 70.0, 70.6, 71.1, 71.5,
      ]),
    },
    {
      id: "happiness",
      label: "Happiness",
      points: points([
        78.4, 78.1, 78.7, 79.0, 78.8, 79.1, 79.3, 79.0, 79.5, 79.7,
      ]),
    },
    {
      id: "culture",
      label: "Culture enjoyment",
      points: points([
        72.2, 72.6, 72.5, 73.0, 73.4, 73.5, 74.1, 74.6, 74.8, 75.2,
      ]),
    },
  ],
  provenance: {
    kind: "calculation",
    source: "Synthetic aligned status observations and player annotation",
    observed_at: "Preview branch · Y4 D050",
    coverage: "experimental",
  },
};

export const programmeMixPreview = [
  { label: "Soviet propaganda", value: 30 },
  { label: "Education", value: 22 },
  { label: "Culture", value: 18 },
  { label: "Sport", value: 14 },
  { label: "Anti-religious", value: 10 },
  { label: "Alcohol", value: 6 },
];

export const broadcastNotebookPreview = [
  {
    hypothesis:
      "A steadier education block precedes higher loyalty without depressing happiness.",
    intervention: "Education 16 → 22; propaganda 24 → 30",
    window: "Y3 D250 → Y4 D050",
    status: "Observe 4 more saves",
  },
  {
    hypothesis:
      "Television adoption is widening the reachable evening audience.",
    intervention: "No schedule change; receiver cohort watch",
    window: "Rolling 180 days",
    status: "Telemetry blocked",
  },
];

export const stationPreview = {
  Radio: {
    workers: "86 / 100",
    professors: "43 / 50",
    rating: "65.6%",
    potential: "5,960",
    current: "3,910",
    availability: "Synthetic · research required",
  },
  Television: {
    workers: "98 / 120",
    professors: "51 / 70",
    rating: "62.4%",
    potential: "5,030",
    current: "3,140",
    availability: "Synthetic · research required",
  },
} as const;
