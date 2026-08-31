import type {
  EvidenceCoverage,
  Provenance,
  SankeyChartSpec,
} from "../charts/types";
import { formatNumber } from "../i18n/format";
import type { Translator } from "../i18n/runtime";
import type {
  ProductionPathwayLink,
  ProductionPathwayModel,
} from "../observations/types";
import {
  productionResourceLabel,
  productionRouteUnit,
} from "./productionRoute";

function boundedPlainText(value: string, maximum: number): string {
  const clean = value
    .replace(/[<>\u0000-\u001f\u007f]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return clean.slice(0, maximum).trim() || "Unavailable";
}

function coverageFor(pathway: ProductionPathwayModel): EvidenceCoverage {
  return pathway.status === "ready" &&
    pathway.links.every(
      (link) =>
        link.mapping.scope_state == null ||
        link.mapping.scope_state === "matched",
    )
    ? "complete"
    : "partial";
}

function provenanceForLink(
  pathway: ProductionPathwayModel,
  link: ProductionPathwayLink,
  t: Translator,
): Provenance {
  return {
    kind: "game_definition",
    source: boundedPlainText(
      t("production-route-link-source", {
        directive: link.source_directive,
        line: link.source_line,
        mapping: link.mapping.mapping_id,
      }),
      240,
    ),
    observed_at: boundedPlainText(
      t("production-route-generation", {
        generation: pathway.snapshot.catalogue_generation_id.slice(0, 12),
      }),
      80,
    ),
    coverage:
      link.mapping.scope_state == null || link.mapping.scope_state === "matched"
        ? "complete"
        : "partial",
  };
}

export function createProductionPathwayChart(
  pathway: ProductionPathwayModel,
  t: Translator,
  locale: string,
): SankeyChartSpec | null {
  if (
    pathway.nodes.length < 2 ||
    pathway.links.length === 0 ||
    pathway.links.some(
      (link) =>
        !Number.isFinite(link.quantity) ||
        link.quantity <= 0 ||
        link.unit !== pathway.unit,
    )
  ) {
    return null;
  }
  const incoming = new Set(pathway.links.map((link) => link.target));
  const outgoing = new Set(pathway.links.map((link) => link.source));
  const unit = productionRouteUnit(pathway.unit, t);
  const largestTerminal = pathway.terminal_requirements.reduce<
    ProductionPathwayModel["terminal_requirements"][number] | null
  >(
    (largest, requirement) =>
      !largest || requirement.quantity > largest.quantity
        ? requirement
        : largest,
    null,
  );

  return {
    schema_version: 1,
    id: "production-pathway",
    title: boundedPlainText(t("production-pathway-chart-title"), 100),
    description: boundedPlainText(
      t("production-pathway-chart-description", {
        quantity: formatNumber(pathway.target_quantity, locale),
        unit,
        stages: pathway.nodes.filter((node) => node.kind === "process").length,
      }),
      500,
    ),
    takeaway: boundedPlainText(
      largestTerminal
        ? t("production-pathway-chart-takeaway", {
            resource: productionResourceLabel(
              largestTerminal.resource_id,
              largestTerminal.display_name,
              t,
            ),
            quantity: formatNumber(largestTerminal.quantity, locale),
            unit: productionRouteUnit(largestTerminal.unit, t),
          })
        : t("production-pathway-chart-takeaway-none"),
      500,
    ),
    kind: "sankey",
    unit,
    balance: "open_boundary",
    nodes: pathway.nodes.map((node) => ({
      id: node.id,
      label:
        node.kind === "resource" && node.resource_id
          ? productionResourceLabel(node.resource_id, node.display_name, t)
          : boundedPlainText(node.display_name, 100),
      role:
        node.kind === "process"
          ? ("process" as const)
          : incoming.has(node.id) && outgoing.has(node.id)
            ? ("intermediate" as const)
            : outgoing.has(node.id)
              ? ("source" as const)
              : ("sink" as const),
    })),
    links: pathway.links.map((link) => ({
      id: link.id,
      source: link.source,
      target: link.target,
      value: link.quantity,
      provenance: provenanceForLink(pathway, link, t),
    })),
    provenance: {
      kind: "game_definition",
      source: boundedPlainText(
        t("production-pathway-chart-source", {
          mapping:
            pathway.mapping_classification === "player_mapped"
              ? t("compatibility-player-mapped")
              : t("compatibility-reviewed"),
        }),
        240,
      ),
      observed_at: boundedPlainText(
        t("production-route-generation", {
          generation: pathway.snapshot.catalogue_generation_id.slice(0, 12),
        }),
        80,
      ),
      coverage: coverageFor(pathway),
    },
  };
}
