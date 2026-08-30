import type {
  ChartSpec,
  EvidenceCoverage,
  EvidenceKind,
} from "../charts/types";
import {
  formatNumber,
  formatPercent,
  formatSignedNumber,
} from "../i18n/format";
import type { Translator } from "../i18n/runtime";

export type KpiPreview = {
  label: string;
  value: string;
  change: string;
  context: string;
  kind: EvidenceKind;
  coverage: EvidenceCoverage;
};

export type MaterialCell = {
  code: string;
  name: string;
  family:
    "raw" | "industrial" | "construction" | "consumer" | "energy" | "waste";
  value: string;
  delta: string;
  reliance: number;
  status: "stable" | "watch" | "exposed";
  note: string;
};

export function createBriefingPreview(t: Translator, locale: string) {
  const percent = (value: number) => formatPercent(value, locale);
  const signed = (value: number) =>
    formatSignedNumber(value, locale, { maximumFractionDigits: 1 });
  const planCategories = Array.from({ length: 14 }, (_, index) =>
    t("chart-category-plan-quarter", {
      year: Math.floor(index / 4) + 1,
      quarter: (index % 4) + 1,
    }),
  );
  const planPoints = (values: number[]) =>
    values.map((value, index) => ({ category: planCategories[index], value }));

  const kpis: KpiPreview[] = [
    {
      label: t("briefing-kpi-plan-attainment"),
      value: percent(92),
      change: t("briefing-kpi-behind-schedule", {
        value: formatNumber(3.1, locale, { maximumFractionDigits: 1 }),
      }),
      context: t("briefing-kpi-industrial-context"),
      kind: "calculation",
      coverage: "complete",
    },
    {
      label: t("briefing-kpi-external-dependency"),
      value: percent(31),
      change: t("briefing-kpi-change-window", {
        value: signed(-4.7),
        days: 180,
      }),
      context: t("briefing-kpi-critical-basket"),
      kind: "estimate",
      coverage: "experimental",
    },
    {
      label: t("briefing-kpi-demographic-resilience"),
      value: `${signed(4.8)}‰`,
      change: t("briefing-kpi-per-thousand", { value: signed(1.2) }),
      context: t("briefing-kpi-demographic-context"),
      kind: "calculation",
      coverage: "partial",
    },
  ];

  const planProgress: ChartSpec = {
    schema_version: 1,
    id: "plan-progress-preview",
    title: t("chart-plan-title"),
    description: t("chart-plan-description"),
    kind: "line",
    category_axis_label: t("chart-axis-plan-quarter"),
    value_axis_label: t("chart-axis-attainment"),
    unit: "%",
    reference_lines: [
      {
        id: "current",
        label: t("chart-reference-latest-save"),
        axis: "category",
        value: planCategories[13],
      },
    ],
    series: [
      {
        id: "actual",
        label: t("chart-series-observed"),
        points: planPoints([
          7, 13, 20, 27, 33, 39, 46, 52, 59, 64, 70, 76, 81, 85,
        ]),
      },
      {
        id: "scheduled",
        label: t("chart-series-scheduled"),
        style: "dashed",
        points: planPoints([
          6, 12, 18, 24, 30, 36, 42, 48, 55, 62, 69, 76, 84, 92,
        ]),
      },
    ],
    provenance: {
      kind: "calculation",
      source: t("synthetic-source-plan-preview"),
      observed_at: t("synthetic-observed-day-230"),
      coverage: "complete",
    },
  };

  const importDependency: ChartSpec = {
    schema_version: 1,
    id: "import-dependency-preview",
    title: t("chart-import-title"),
    description: t("chart-import-description"),
    kind: "bar",
    orientation: "horizontal",
    value_axis_label: t("chart-axis-import-share"),
    unit: "%",
    reference_lines: [
      {
        id: "review",
        label: t("chart-reference-review-threshold"),
        axis: "value",
        value: 50,
      },
    ],
    series: [
      {
        id: "reliance",
        label: t("chart-series-import-share"),
        points: [
          { category: t("material-electronic-components"), value: 78 },
          { category: t("material-chemicals"), value: 61 },
          { category: t("material-steel"), value: 44 },
          { category: t("material-fabric"), value: 33 },
          { category: t("material-fuel"), value: 17 },
          { category: t("material-crops"), value: 8 },
        ],
      },
    ],
    provenance: {
      kind: "estimate",
      source: t("synthetic-source-material-basket"),
      observed_at: t("synthetic-observed-day-230"),
      coverage: "experimental",
    },
  };

  const materialCells: MaterialCell[] = [
    material(
      t,
      percent,
      signed,
      "Crp",
      "material-crops",
      "raw",
      8,
      -2.1,
      "stable",
      "material-note-crops",
    ),
    material(
      t,
      percent,
      signed,
      "Wd",
      "material-wood",
      "raw",
      0,
      0,
      "stable",
      "material-note-wood",
    ),
    material(
      t,
      percent,
      signed,
      "Ol",
      "material-oil",
      "raw",
      3,
      -0.7,
      "stable",
      "material-note-oil",
    ),
    material(
      t,
      percent,
      signed,
      "Ch",
      "material-chemicals",
      "industrial",
      61,
      8.4,
      "exposed",
      "material-note-chemicals",
    ),
    material(
      t,
      percent,
      signed,
      "Mec",
      "material-mechanical-parts",
      "industrial",
      29,
      -3.2,
      "watch",
      "material-note-mechanical",
    ),
    material(
      t,
      percent,
      signed,
      "Ecp",
      "material-electronic-components",
      "industrial",
      78,
      2.6,
      "exposed",
      "material-note-electronic",
    ),
    material(
      t,
      percent,
      signed,
      "St",
      "material-steel",
      "construction",
      44,
      -5.3,
      "watch",
      "material-note-steel",
    ),
    material(
      t,
      percent,
      signed,
      "Br",
      "material-bricks",
      "construction",
      11,
      -1.8,
      "stable",
      "material-note-bricks",
    ),
    material(
      t,
      percent,
      signed,
      "Pf",
      "material-prefab-panels",
      "construction",
      23,
      1.2,
      "watch",
      "material-note-prefab",
    ),
    material(
      t,
      percent,
      signed,
      "Fd",
      "material-food",
      "consumer",
      6,
      -0.9,
      "stable",
      "material-note-food",
    ),
    material(
      t,
      percent,
      signed,
      "Cl",
      "material-clothes",
      "consumer",
      14,
      0.4,
      "stable",
      "material-note-clothes",
    ),
    material(
      t,
      percent,
      signed,
      "El",
      "material-electronics",
      "consumer",
      39,
      4.1,
      "watch",
      "material-note-electronics",
    ),
    material(
      t,
      percent,
      signed,
      "Fl",
      "material-fuel",
      "energy",
      17,
      -6,
      "stable",
      "material-note-fuel",
    ),
    material(
      t,
      percent,
      signed,
      "Pw",
      "material-power",
      "energy",
      4,
      0,
      "stable",
      "material-note-power",
    ),
    {
      code: "Mw",
      name: t("material-mixed-waste"),
      family: "waste",
      value: t("briefing-mass-kilotonne", { value: "1.9" }),
      delta: signed(5.7),
      reliance: 0,
      status: "watch",
      note: t("material-note-mixed-waste"),
    },
    {
      code: "Hw",
      name: t("material-hazardous-waste"),
      family: "waste",
      value: t("briefing-mass-tonne", { value: 84 }),
      delta: signed(12),
      reliance: 0,
      status: "exposed",
      note: t("material-note-hazardous-waste"),
    },
  ];

  const attention = [
    {
      level: t("attention-level-signal"),
      title: t("attention-chemical-title"),
      detail: t("attention-chemical-detail"),
    },
    {
      level: t("attention-level-plan"),
      title: t("attention-plan-title"),
      detail: t("attention-plan-detail"),
    },
    {
      level: t("attention-level-coverage"),
      title: t("attention-coverage-title"),
      detail: t("coverage-dependency-experimental"),
    },
  ];

  return { kpis, planProgress, importDependency, materialCells, attention };
}

function material(
  t: Translator,
  percent: (value: number) => string,
  signed: (value: number) => string,
  code: string,
  nameKey: Parameters<Translator>[0],
  family: MaterialCell["family"],
  reliance: number,
  delta: number,
  status: MaterialCell["status"],
  noteKey: Parameters<Translator>[0],
): MaterialCell {
  return {
    code,
    name: t(nameKey),
    family,
    value: percent(reliance),
    delta: signed(delta),
    reliance,
    status,
    note: t(noteKey),
  };
}
