import type { SankeyChartSpec } from "../charts/types";
import type { Translator } from "../i18n/runtime";

export function createMaterialFlowPreview(t: Translator): SankeyChartSpec {
  return {
    schema_version: 1,
    id: "steel-allocation-preview",
    title: t("chart-material-flow-title"),
    description: t("chart-material-flow-description"),
    takeaway: t("chart-material-flow-takeaway"),
    kind: "sankey",
    unit: t("chart-material-flow-unit"),
    balance: "conserved",
    nodes: [
      {
        id: "domestic-output",
        label: t("chart-material-flow-domestic"),
        role: "source",
      },
      {
        id: "imports",
        label: t("chart-material-flow-imports"),
        role: "source",
      },
      {
        id: "available-steel",
        label: t("chart-material-flow-pool"),
        role: "process",
      },
      {
        id: "construction",
        label: t("chart-material-flow-construction"),
        role: "sink",
      },
      {
        id: "mechanical-components",
        label: t("chart-material-flow-mechanical"),
        role: "sink",
      },
      {
        id: "vehicle-production",
        label: t("chart-material-flow-vehicles"),
        role: "sink",
      },
      {
        id: "exports",
        label: t("chart-material-flow-exports"),
        role: "sink",
      },
      {
        id: "unaccounted",
        label: t("chart-material-flow-residual"),
        role: "residual",
      },
    ],
    links: [
      {
        id: "domestic-to-pool",
        source: "domestic-output",
        target: "available-steel",
        value: 68,
      },
      {
        id: "imports-to-pool",
        source: "imports",
        target: "available-steel",
        value: 32,
      },
      {
        id: "pool-to-construction",
        source: "available-steel",
        target: "construction",
        value: 42,
      },
      {
        id: "pool-to-mechanical",
        source: "available-steel",
        target: "mechanical-components",
        value: 24,
      },
      {
        id: "pool-to-vehicles",
        source: "available-steel",
        target: "vehicle-production",
        value: 18,
      },
      {
        id: "pool-to-exports",
        source: "available-steel",
        target: "exports",
        value: 10,
      },
      {
        id: "pool-to-unaccounted",
        source: "available-steel",
        target: "unaccounted",
        value: 6,
      },
    ],
    provenance: {
      kind: "estimate",
      source: t("chart-material-flow-source"),
      observed_at: t("chart-material-flow-window"),
      coverage: "experimental",
    },
  };
}
