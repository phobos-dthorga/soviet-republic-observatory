import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import type { AnySchema } from "ajv";
import { describe, expect, it } from "vitest";
import {
  optionForSankey,
  provenanceForLink,
  summariseSankey,
  validateSankeySpec,
} from "./sankey";
import type { SankeyChartSpec } from "./types";

const schema = JSON.parse(
  readFileSync(
    new URL(
      "../../../schemas/sankey-chart-spec-v1.schema.json",
      import.meta.url,
    ),
    "utf8",
  ),
) as AnySchema;
const validateSchema = new Ajv2020({ allErrors: true, strict: true }).compile(
  schema,
);

const baseSpec: SankeyChartSpec = {
  schema_version: 1,
  id: "steel-flow",
  title: "Steel flow",
  description: "Where did steel originate and where was it allocated?",
  takeaway: "The complete illustrative flow balances.",
  kind: "sankey",
  unit: "kt / window",
  balance: "conserved",
  nodes: [
    { id: "domestic", label: "Domestic", role: "source" },
    { id: "pool", label: "Available steel", role: "process" },
    { id: "construction", label: "Construction", role: "sink" },
    { id: "residual", label: "Unaccounted", role: "residual" },
  ],
  links: [
    { id: "domestic-pool", source: "domestic", target: "pool", value: 10 },
    {
      id: "pool-construction",
      source: "pool",
      target: "construction",
      value: 8,
    },
    { id: "pool-residual", source: "pool", target: "residual", value: 2 },
  ],
  provenance: {
    kind: "estimate",
    source: "Synthetic fixture",
    observed_at: "Illustrative window",
    coverage: "experimental",
  },
};

describe("Sankey chart contract", () => {
  it("accepts a strict, conserved flow and summarises it", () => {
    expect(
      validateSchema(baseSpec),
      new Ajv2020().errorsText(validateSchema.errors),
    ).toBe(true);
    expect(validateSankeySpec(baseSpec)).toEqual({ valid: true, errors: [] });
    expect(summariseSankey(baseSpec)).toMatchObject({
      sourceTotal: 10,
      sinkTotal: 10,
      largestLink: baseSpec.links[0],
    });
  });

  it("rejects unknown renderer options and markup", () => {
    const rendererInjection = { ...baseSpec, echarts: { animation: true } };
    expect(validateSchema(rendererInjection)).toBe(false);
    expect(
      validateSankeySpec(rendererInjection as SankeyChartSpec).errors,
    ).toContain("unknown chart field");

    const markup = structuredClone(baseSpec);
    markup.nodes[0].label = "<script>alert(1)</script>";
    expect(validateSchema(markup)).toBe(false);
    expect(validateSankeySpec(markup).errors).toContain(
      "invalid node label: domestic",
    );
  });

  it("rejects cycles, duplicate endpoints, and unbalanced internal nodes", () => {
    const invalid = structuredClone(baseSpec);
    invalid.links.push({
      id: "construction-pool",
      source: "construction",
      target: "pool",
      value: 2,
    });
    invalid.links.push({
      id: "duplicate-flow",
      source: "pool",
      target: "residual",
      value: 1,
    });
    const result = validateSankeySpec(invalid);

    expect(result.valid).toBe(false);
    expect(result.errors).toContain("cyclic flow is not supported");
    expect(result.errors).toContain("duplicate link endpoints: duplicate-flow");
    expect(result.errors).toContain("unbalanced internal node: pool");
  });

  it("uses a bounded host-owned renderer and honours reduced motion", () => {
    const option = optionForSankey(baseSpec, undefined, true, "de-DE") as {
      animationDuration: number;
      tooltip: {
        renderMode: string;
        formatter: (item: unknown) => string;
      };
      series: Array<{
        type: string;
        draggable: boolean;
        lineStyle: { color: string };
        data: Array<{ itemStyle: { borderType: string } }>;
      }>;
    };

    expect(option.animationDuration).toBe(0);
    expect(option.tooltip.renderMode).toBe("richText");
    expect(option.series[0]).toMatchObject({
      type: "sankey",
      draggable: false,
      lineStyle: { color: "source" },
    });
    expect(option.series[0].data.at(-1)?.itemStyle.borderType).toBe("dashed");
    expect(
      option.tooltip.formatter({
        dataType: "edge",
        data: { source: "domestic", target: "pool", value: 1234.5 },
      }),
    ).toContain("1.234,5 kt / window");
  });

  it("inherits chart provenance unless a link supplies its own", () => {
    expect(provenanceForLink(baseSpec, baseSpec.links[0])).toBe(
      baseSpec.provenance,
    );
    const link = {
      ...baseSpec.links[0],
      provenance: {
        kind: "calculation" as const,
        source: "Flow reconciliation",
        observed_at: "Window 1",
        coverage: "complete" as const,
      },
    };
    expect(provenanceForLink(baseSpec, link).kind).toBe("calculation");
  });
});
