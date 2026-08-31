import { describe, expect, it } from "vitest";
import { validateSankeySpec } from "../charts/sankey";
import type { Translator } from "../i18n/runtime";
import type { ProductionPathwayModel } from "../observations/types";
import { createProductionPathwayChart } from "./productionPathway";

const t = ((key: string, values: Record<string, unknown> = {}) => {
  if (key === "production-route-unit-source-rate")
    return "definition-rate units";
  return `${key} ${Object.values(values).join(" ")}`.trim();
}) as Translator;

const mapping = {
  mapping_id: "core.definition.production_input",
  catalogue_scope_id: null,
  mapping_classification: "reviewed_mapping",
  scope_state: null,
  update_policy: null,
  acknowledged_content_hash: null,
  current_content_hash: null,
};

const pathway: ProductionPathwayModel = {
  schema_version: 1,
  status: "ready",
  root_recipe_entity_id: "base::recipe::fuel",
  output_resource_id: "resource::fuel",
  target_quantity: 10,
  unit: "source_rate",
  max_depth: 4,
  mapping_classification: "reviewed_mapping",
  nodes: [
    {
      id: "oil",
      kind: "resource",
      display_name: "Oil",
      resource_id: "resource::oil",
      recipe_entity_id: null,
      package_name: null,
      depth: 2,
    },
    {
      id: "refinery",
      kind: "process",
      display_name: "Refinery",
      resource_id: null,
      recipe_entity_id: "base::recipe::oil",
      package_name: "Base game",
      depth: 1,
    },
    {
      id: "bitumen",
      kind: "resource",
      display_name: "Bitumen",
      resource_id: "resource::bitumen",
      recipe_entity_id: null,
      package_name: null,
      depth: 1,
    },
    {
      id: "fuel-plant",
      kind: "process",
      display_name: "Fuel plant",
      resource_id: null,
      recipe_entity_id: "base::recipe::fuel",
      package_name: "Base game",
      depth: 0,
    },
    {
      id: "fuel",
      kind: "resource",
      display_name: "Fuel",
      resource_id: "resource::fuel",
      recipe_entity_id: null,
      package_name: null,
      depth: 0,
    },
  ],
  links: [
    {
      id: "one",
      source: "oil",
      target: "refinery",
      resource_id: "resource::oil",
      quantity: 20,
      unit: "source_rate",
      source_directive: "$CONSUMPTION",
      source_line: 1,
      mapping,
    },
    {
      id: "two",
      source: "refinery",
      target: "bitumen",
      resource_id: "resource::bitumen",
      quantity: 10,
      unit: "source_rate",
      source_directive: "$PRODUCTION",
      source_line: 2,
      mapping,
    },
    {
      id: "three",
      source: "bitumen",
      target: "fuel-plant",
      resource_id: "resource::bitumen",
      quantity: 10,
      unit: "source_rate",
      source_directive: "$CONSUMPTION",
      source_line: 3,
      mapping,
    },
    {
      id: "four",
      source: "fuel-plant",
      target: "fuel",
      resource_id: "resource::fuel",
      quantity: 10,
      unit: "source_rate",
      source_directive: "$PRODUCTION",
      source_line: 4,
      mapping,
    },
  ],
  choices: [],
  terminal_requirements: [
    {
      resource_id: "resource::oil",
      display_name: "Oil",
      quantity: 20,
      unit: "source_rate",
      reason: "external_input",
    },
  ],
  auxiliary_requirements: [],
  diagnostics: [],
  snapshot: {
    catalogue_generation_id: "a".repeat(64),
    compatibility_profile_id: "org.republic-observatory.wrsr-1.1.1.9",
    compatibility_profile_version: "1.0.0",
    compatibility_profile_hash: "b".repeat(64),
    mapping_classification: "reviewed_mapping",
    overlay_profile_id: null,
    overlay_revision: null,
    observation_watermark: null,
    warehouse_schema_version: 5,
    projector_version: "test",
  },
};

describe("production pathway Sankey transformation", () => {
  it("renders multi-stage coefficients while retaining the intermediate resource", () => {
    const chart = createProductionPathwayChart(pathway, t, "en-AU");

    expect(chart).not.toBeNull();
    expect(chart?.nodes.find((node) => node.id === "bitumen")?.role).toBe(
      "intermediate",
    );
    expect(chart?.links.map((link) => link.value)).toEqual([20, 10, 10, 10]);
    expect(validateSankeySpec(chart!)).toEqual({ valid: true, errors: [] });
  });

  it("refuses mixed or invalid link geometry", () => {
    expect(
      createProductionPathwayChart(
        { ...pathway, links: [{ ...pathway.links[0], unit: "per_second" }] },
        t,
        "en-AU",
      ),
    ).toBeNull();
    expect(
      createProductionPathwayChart(
        { ...pathway, links: [{ ...pathway.links[0], quantity: 0 }] },
        t,
        "en-AU",
      ),
    ).toBeNull();
  });

  it("marks bounded or player-mapped pathways as partial evidence", () => {
    const chart = createProductionPathwayChart(
      {
        ...pathway,
        status: "bounded",
        mapping_classification: "player_mapped",
      },
      t,
      "en-AU",
    );
    expect(chart?.provenance.coverage).toBe("partial");
  });
});
