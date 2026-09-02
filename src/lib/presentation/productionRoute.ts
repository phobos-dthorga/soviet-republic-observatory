import type {
  EvidenceCoverage,
  Provenance,
  SankeyChartSpec,
} from "../charts/types";
import { formatNumber } from "../i18n/format";
import type { Translator } from "../i18n/runtime";
import type {
  ProductionRouteFlow,
  ProductionRouteModel,
} from "../observations/types";

function boundedPlainText(value: string, maximum: number): string {
  const clean = value
    .replace(/[<>\u0000-\u001f\u007f]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return clean.slice(0, maximum).trim() || "Unavailable";
}

export function productionRouteUnit(unit: string, t: Translator): string {
  if (unit === "source_rate")
    return boundedPlainText(t("production-route-unit-source-rate"), 32);
  if (unit === "per_second")
    return boundedPlainText(t("production-route-unit-per-second"), 32);
  return boundedPlainText(unit, 32);
}

export function productionResourceLabel(
  _resourceId: string,
  fallback: string,
  _t: Translator,
): string {
  return boundedPlainText(fallback, 100);
}

function coverageFor(route: ProductionRouteModel): EvidenceCoverage {
  return route.coverage === "complete" &&
    route.flows
      .filter((flow) => flow.basis_role === "primary")
      .every(
        (flow) =>
          flow.mapping.scope_state == null ||
          flow.mapping.scope_state === "matched",
      )
    ? "complete"
    : "partial";
}

function provenanceForFlow(
  route: ProductionRouteModel,
  flow: ProductionRouteFlow,
  t: Translator,
): Provenance {
  return {
    kind: "game_definition",
    source: boundedPlainText(
      t("production-route-link-source", {
        directive: flow.source_directive,
        line: flow.source_line,
        mapping: flow.mapping.mapping_id,
      }),
      240,
    ),
    observed_at: boundedPlainText(
      t("production-route-generation", {
        generation: route.snapshot.catalogue_generation_id.slice(0, 12),
      }),
      80,
    ),
    coverage:
      flow.mapping.scope_state == null || flow.mapping.scope_state === "matched"
        ? route.coverage === "complete"
          ? "complete"
          : "partial"
        : "partial",
  };
}

/**
 * Converts an authoritative, bounded production-route model into the separate
 * host-owned Sankey contract. Unavailable or semantically mixed relations stay
 * in the evidence table and are never coerced into ribbon widths.
 */
export function createProductionRouteChart(
  route: ProductionRouteModel,
  t: Translator,
  locale: string,
): SankeyChartSpec | null {
  if (
    !["ready", "ready_with_auxiliary"].includes(route.status) ||
    route.unit == null ||
    route.target_quantity == null ||
    route.scale_factor == null
  ) {
    return null;
  }
  const primaryFlows = route.flows.filter(
    (flow) => flow.basis_role === "primary",
  );
  if (
    primaryFlows.length === 0 ||
    primaryFlows.some(
      (flow) => flow.scaled_quantity == null || flow.scaled_quantity <= 0,
    )
  ) {
    return null;
  }

  const unit = productionRouteUnit(route.unit, t);
  const displayName = boundedPlainText(route.display_name, 100);
  const inputs = primaryFlows.filter(
    (flow) => flow.direction !== "production_output",
  );
  const largestInput = inputs.reduce<ProductionRouteFlow | null>(
    (largest, flow) =>
      !largest || (flow.scaled_quantity ?? 0) > (largest.scaled_quantity ?? 0)
        ? flow
        : largest,
    null,
  );
  const nodes = [
    ...primaryFlows.map((flow, index) => ({
      id: `resource-${index}`,
      label: productionResourceLabel(flow.resource_id, flow.display_name, t),
      role:
        flow.direction === "production_output"
          ? ("sink" as const)
          : ("source" as const),
    })),
    { id: "production-process", label: displayName, role: "process" as const },
  ];
  const links = primaryFlows.map((flow, index) => ({
    id: `flow-${index}`,
    source:
      flow.direction === "production_output"
        ? "production-process"
        : `resource-${index}`,
    target:
      flow.direction === "production_output"
        ? `resource-${index}`
        : "production-process",
    value: flow.scaled_quantity!,
    provenance: provenanceForFlow(route, flow, t),
  }));
  const mappingLabel =
    route.mapping_classification === "player_mapped"
      ? t("compatibility-player-mapped")
      : t("compatibility-reviewed");

  return {
    schema_version: 1,
    id: "production-route",
    title: boundedPlainText(
      t("production-route-chart-title", { route: displayName }),
      100,
    ),
    description: boundedPlainText(
      t("production-route-chart-description", {
        target: formatNumber(route.target_quantity, locale),
        unit,
        route: displayName,
      }),
      500,
    ),
    takeaway: boundedPlainText(
      largestInput
        ? t("production-route-chart-takeaway", {
            resource: productionResourceLabel(
              largestInput.resource_id,
              largestInput.display_name,
              t,
            ),
            quantity: formatNumber(largestInput.scaled_quantity ?? 0, locale),
            unit,
          })
        : t("production-route-chart-takeaway-none"),
      500,
    ),
    kind: "sankey",
    unit,
    balance: "open_boundary",
    nodes,
    links,
    provenance: {
      kind: "game_definition",
      source: boundedPlainText(
        t("production-route-chart-source", {
          package: boundedPlainText(route.package_name, 100),
          mapping: mappingLabel,
        }),
        240,
      ),
      observed_at: boundedPlainText(
        t("production-route-generation", {
          generation: route.snapshot.catalogue_generation_id.slice(0, 12),
        }),
        80,
      ),
      coverage: coverageFor(route),
    },
  };
}
