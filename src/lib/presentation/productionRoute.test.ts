import { describe, expect, it } from "vitest";
import { validateSankeySpec } from "../charts/sankey";
import type { Translator } from "../i18n/runtime";
import type { ProductionRouteModel } from "../observations/types";
import {
  createProductionRouteChart,
  productionResourceLabel,
} from "./productionRoute";

const t = ((key: string, values: Record<string, unknown> = {}) => {
  if (key === "production-route-unit-source-rate")
    return "definition-rate units";
  if (key === "production-route-unit-per-second") return "units / second";
  return `${key} ${Object.values(values).join(" ")}`.trim();
}) as Translator;

const route: ProductionRouteModel = {
  schema_version: 2,
  route_id: "base::recipe::chemicals",
  revision_hash: "a".repeat(64),
  building_entity_id: "base::building::chemical-plant",
  display_name: "Chemical <plant>",
  package_name: "Base game",
  coverage: "complete",
  status: "ready",
  relation_count: 3,
  primary_flow_count: 3,
  auxiliary_flow_count: 0,
  unit: "source_rate",
  selected_output_resource_id: "resource::chemicals",
  target_quantity: 10,
  scale_factor: 20,
  mapping_classification: "reviewed_mapping",
  flows: [
    {
      id: "production_input-0",
      direction: "production_input",
      resource_id: "resource::oil",
      display_name: "Oil",
      source_quantity: 2,
      scaled_quantity: 40,
      unit: "source_rate",
      basis_role: "primary",
      basis_exclusion: null,
      resolution: "source_coefficient",
      source_directive: "$CONSUMPTION",
      source_line: 10,
      mapping: {
        mapping_id: "core.definition.production_input",
        catalogue_scope_id: null,
        mapping_classification: "reviewed_mapping",
        scope_state: null,
        update_policy: null,
        acknowledged_content_hash: null,
        current_content_hash: null,
      },
    },
    {
      id: "production_input-1",
      direction: "production_input",
      resource_id: "resource::power",
      display_name: "Power",
      source_quantity: 1,
      scaled_quantity: 20,
      unit: "source_rate",
      basis_role: "primary",
      basis_exclusion: null,
      resolution: "source_coefficient",
      source_directive: "$CONSUMPTION",
      source_line: 11,
      mapping: {
        mapping_id: "core.definition.production_input",
        catalogue_scope_id: null,
        mapping_classification: "reviewed_mapping",
        scope_state: null,
        update_policy: null,
        acknowledged_content_hash: null,
        current_content_hash: null,
      },
    },
    {
      id: "production_output-0",
      direction: "production_output",
      resource_id: "resource::chemicals",
      display_name: "Chemicals",
      source_quantity: 0.5,
      scaled_quantity: 10,
      unit: "source_rate",
      basis_role: "primary",
      basis_exclusion: null,
      resolution: "source_coefficient",
      source_directive: "$PRODUCTION",
      source_line: 12,
      mapping: {
        mapping_id: "core.definition.production_output",
        catalogue_scope_id: null,
        mapping_classification: "reviewed_mapping",
        scope_state: null,
        update_policy: null,
        acknowledged_content_hash: null,
        current_content_hash: null,
      },
    },
  ],
  snapshot: {
    catalogue_generation_id: "b".repeat(64),
    compatibility_profile_id: "org.republic-observatory.wrsr-1.1.1.9",
    compatibility_profile_version: "1.0.0",
    compatibility_profile_hash: "c".repeat(64),
    mapping_classification: "reviewed_mapping",
    overlay_profile_id: null,
    overlay_revision: null,
    observation_watermark: null,
    warehouse_schema_version: 4,
    projector_version: "test",
  },
};

describe("production route Sankey transformation", () => {
  it("uses the host-supplied label without maintaining a fixed inventory", () => {
    expect(
      productionResourceLabel("resource::eletronics", "eletronics", t),
    ).toBe("eletronics");
    expect(
      productionResourceLabel("resource::ecomponents", "ecomponents", t),
    ).toBe("ecomponents");
  });

  it("renders compatible definition coefficients as an open-boundary flow", () => {
    const chart = createProductionRouteChart(route, t, "en-AU");

    expect(chart).not.toBeNull();
    expect(chart).toMatchObject({
      balance: "open_boundary",
      unit: "definition-rate units",
      provenance: { kind: "game_definition", coverage: "complete" },
    });
    expect(chart?.links.map((link) => link.value)).toEqual([40, 20, 10]);
    expect(chart?.links[0].provenance?.source).toContain("$CONSUMPTION");
    expect(chart?.title).not.toContain("<");
    expect(validateSankeySpec(chart!)).toEqual({ valid: true, errors: [] });
  });

  it("keeps unavailable primary relations out of ribbon geometry", () => {
    expect(
      createProductionRouteChart(
        {
          ...route,
          status: "no_comparable_input",
          unit: null,
          scale_factor: null,
        },
        t,
        "en-AU",
      ),
    ).toBeNull();
    expect(
      createProductionRouteChart(
        {
          ...route,
          flows: route.flows.map((flow, index) =>
            index === 0 ? { ...flow, scaled_quantity: null } : flow,
          ),
        },
        t,
        "en-AU",
      ),
    ).toBeNull();
  });

  it("draws the selected basis while preserving auxiliary requirements outside the ribbons", () => {
    const mixed = structuredClone(route);
    mixed.status = "ready_with_auxiliary";
    mixed.primary_flow_count = 2;
    mixed.auxiliary_flow_count = 1;
    mixed.flows[1] = {
      ...mixed.flows[1],
      resource_id: "resource::eletric",
      display_name: "eletric",
      unit: "per_second",
      basis_role: "auxiliary",
      basis_exclusion: "different_unit",
    };

    const chart = createProductionRouteChart(mixed, t, "en-AU");

    expect(chart?.links.map((link) => link.value)).toEqual([40, 10]);
    expect(chart?.nodes.map((node) => node.label)).not.toContain("eletric");
    expect(validateSankeySpec(chart!)).toEqual({ valid: true, errors: [] });
  });

  it("marks tracked mod updates as partial evidence", () => {
    const updated = structuredClone(route);
    updated.mapping_classification = "player_mapped";
    updated.flows[0].mapping.scope_state = "updated_unreviewed";

    const chart = createProductionRouteChart(updated, t, "en-AU");

    expect(chart?.provenance.coverage).toBe("partial");
    expect(chart?.links[0].provenance?.coverage).toBe("partial");
  });
});
