import type { ChartSpec } from "../charts/types";
import { formatNumber, formatPercent } from "../i18n/format";
import type { Translator } from "../i18n/runtime";

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
const points = (values: number[]) =>
  dates.map((category, index) => ({ category, value: values[index] }));

export function createBroadcastPreview(t: Translator, locale: string) {
  const receiverLadder: ChartSpec = {
    schema_version: 1,
    id: "receiver-ladder-preview",
    title: t("chart-receiver-title"),
    description: t("chart-receiver-description"),
    kind: "area",
    category_axis_label: t("chart-axis-synthetic-observation"),
    value_axis_label: t("chart-axis-classified-share"),
    unit: "%",
    value_domain: { min: 0, max: 100 },
    series: [
      {
        id: "none",
        label: t("receiver-none"),
        stack_id: "receiver_classes",
        points: points([28, 26, 24, 22, 20, 18, 16, 14, 12, 10]),
      },
      {
        id: "radio",
        label: t("receiver-radio"),
        stack_id: "receiver_classes",
        points: points([42, 41, 39, 37, 35, 33, 31, 29, 27, 25]),
      },
      {
        id: "television",
        label: t("receiver-television"),
        stack_id: "receiver_classes",
        points: points([25, 27, 29, 31, 33, 35, 37, 39, 42, 45]),
      },
      {
        id: "computer",
        label: t("receiver-computer"),
        stack_id: "receiver_classes",
        points: points([5, 6, 8, 10, 12, 14, 16, 18, 19, 20]),
      },
    ],
    provenance: {
      kind: "extension_calculation",
      source: t("synthetic-source-receiver-pack"),
      observed_at: t("synthetic-observed-y4-d050"),
      coverage: "experimental",
    },
  };

  const audienceReach: ChartSpec = {
    schema_version: 1,
    id: "audience-reach-preview",
    title: t("chart-audience-title"),
    description: t("chart-audience-description"),
    kind: "line",
    category_axis_label: t("chart-axis-synthetic-observation"),
    value_axis_label: t("chart-axis-citizens"),
    unit: t("unit-citizens"),
    series: [
      {
        id: "radio-potential",
        label: t("chart-series-radio-potential"),
        style: "dashed",
        points: points([
          4200, 4420, 4600, 4800, 5050, 5300, 5480, 5630, 5790, 5960,
        ]),
      },
      {
        id: "radio-current",
        label: t("chart-series-radio-current"),
        points: points([
          2520, 2740, 2870, 3010, 3220, 3450, 3510, 3690, 3770, 3910,
        ]),
      },
      {
        id: "tv-potential",
        label: t("chart-series-tv-potential"),
        style: "dashed",
        points: points([
          2450, 2680, 2920, 3210, 3480, 3810, 4090, 4380, 4700, 5030,
        ]),
      },
      {
        id: "tv-current",
        label: t("chart-series-tv-current"),
        points: points([
          970, 1120, 1290, 1510, 1740, 2030, 2290, 2510, 2810, 3140,
        ]),
      },
    ],
    provenance: {
      kind: "estimate",
      source: t("synthetic-source-audience"),
      observed_at: t("synthetic-observed-y4-d050"),
      coverage: "experimental",
    },
  };

  const influenceAssay: ChartSpec = {
    schema_version: 1,
    id: "influence-assay-preview",
    title: t("chart-influence-title"),
    description: t("chart-influence-description"),
    kind: "bar",
    orientation: "horizontal",
    value_axis_label: t("chart-axis-expected-effect"),
    unit: t("unit-index-points"),
    value_domain: { min: -4, max: 4 },
    reference_lines: [
      {
        id: "neutral",
        label: t("chart-reference-no-effect"),
        axis: "value",
        value: 0,
      },
    ],
    series: [
      {
        id: "expected-effect",
        label: t("chart-series-expected-effect"),
        points: [
          { category: t("outcome-government-loyalty"), value: 3.2 },
          { category: t("outcome-education"), value: 1.5 },
          { category: t("outcome-culture-enjoyment"), value: 1.1 },
          { category: t("outcome-happiness"), value: 0.7 },
          { category: t("outcome-alcohol-addiction"), value: -0.6 },
          { category: t("outcome-criminality"), value: -1.3 },
          { category: t("outcome-religion-sympathy"), value: -2.1 },
        ],
      },
    ],
    provenance: {
      kind: "estimate",
      source: t("synthetic-source-influence-model"),
      observed_at: t("synthetic-observed-y4-d050"),
      coverage: "experimental",
    },
  };

  const outcomeLaboratory: ChartSpec = {
    schema_version: 1,
    id: "broadcast-outcomes-preview",
    title: t("chart-outcomes-title"),
    description: t("causality-chart-outcomes-description"),
    kind: "line",
    category_axis_label: t("chart-axis-synthetic-observation"),
    value_axis_label: t("chart-axis-citizen-status"),
    unit: "%",
    value_domain: { min: 60, max: 90 },
    reference_lines: [
      {
        id: "programme-change",
        label: t("chart-reference-new-schedule"),
        axis: "category",
        value: "Y3 D250",
      },
    ],
    series: [
      {
        id: "loyalty",
        label: t("outcome-government-loyalty"),
        points: points([
          68.1, 68.4, 68.2, 68.7, 69, 69.2, 70, 70.6, 71.1, 71.5,
        ]),
      },
      {
        id: "happiness",
        label: t("outcome-happiness"),
        points: points([
          78.4, 78.1, 78.7, 79, 78.8, 79.1, 79.3, 79, 79.5, 79.7,
        ]),
      },
      {
        id: "culture",
        label: t("outcome-culture-enjoyment"),
        points: points([
          72.2, 72.6, 72.5, 73, 73.4, 73.5, 74.1, 74.6, 74.8, 75.2,
        ]),
      },
    ],
    provenance: {
      kind: "calculation",
      source: t("synthetic-source-outcomes"),
      observed_at: t("synthetic-observed-y4-d050"),
      coverage: "experimental",
    },
  };

  const programmeMix = [
    { label: t("programme-soviet-propaganda"), value: 30 },
    { label: t("programme-education"), value: 22 },
    { label: t("programme-culture"), value: 18 },
    { label: t("programme-sport"), value: 14 },
    { label: t("programme-anti-religious"), value: 10 },
    { label: t("programme-alcohol"), value: 6 },
  ];
  const notebook = [
    {
      hypothesis: t("notebook-hypothesis-education"),
      intervention: t("notebook-intervention-education"),
      window: "Y3 D250 → Y4 D050",
      status: t("notebook-status-observe"),
    },
    {
      hypothesis: t("notebook-hypothesis-television"),
      intervention: t("notebook-intervention-cohort"),
      window: t("notebook-window-rolling"),
      status: t("notebook-status-blocked"),
    },
  ];
  const station = {
    radio: {
      name: t("station-radio"),
      workers: "86 / 100",
      professors: "43 / 50",
      rating: formatPercent(65.6, locale),
      potential: formatNumber(5960, locale),
      current: formatNumber(3910, locale),
      availability: t("synthetic-research-required"),
    },
    television: {
      name: t("station-television"),
      workers: "98 / 120",
      professors: "51 / 70",
      rating: formatPercent(62.4, locale),
      potential: formatNumber(5030, locale),
      current: formatNumber(3140, locale),
      availability: t("synthetic-research-required"),
    },
  } as const;

  return {
    receiverLadder,
    audienceReach,
    influenceAssay,
    outcomeLaboratory,
    programmeMix,
    notebook,
    station,
  };
}
