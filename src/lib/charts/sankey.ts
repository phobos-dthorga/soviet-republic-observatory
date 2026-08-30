import type { EChartsCoreOption } from "echarts/core";
import { formatNumber } from "../i18n/format";
import { observatoryChartTheme } from "./chartOptions";
import type {
  ChartTheme,
  Provenance,
  SankeyChartSpec,
  SankeyLink,
  SankeyNodeRole,
} from "./types";

export const SANKEY_LIMITS = {
  nodes: 64,
  links: 128,
} as const;

const localId = /^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$/;
const plainText = /^[^<>\u0000-\u001f\u007f]+$/;
const roleColours: Record<SankeyNodeRole, string> = {
  source: "#80c6d8",
  process: "#d8b86a",
  intermediate: "#8da6c9",
  sink: "#b6a8ce",
  residual: "#d88474",
};

export type SankeyValidation = {
  valid: boolean;
  errors: string[];
};

export type SankeySummary = {
  sourceTotal: number;
  sinkTotal: number;
  largestLink: SankeyLink | null;
};

function hasOnlyKeys(value: object, allowed: readonly string[]): boolean {
  const allowlist = new Set(allowed);
  return Object.keys(value).every((key) => allowlist.has(key));
}

function validText(value: string, maximum: number): boolean {
  return value.length > 0 && value.length <= maximum && plainText.test(value);
}

function validProvenance(provenance: Provenance): boolean {
  return (
    hasOnlyKeys(provenance, ["kind", "source", "observed_at", "coverage"]) &&
    [
      "save_fact",
      "game_definition",
      "calculation",
      "extension_calculation",
      "player_override",
      "player_definition",
      "estimate",
      "recommendation",
    ].includes(provenance.kind) &&
    validText(provenance.source, 240) &&
    validText(provenance.observed_at, 80) &&
    ["complete", "partial", "experimental"].includes(provenance.coverage)
  );
}

export function provenanceForLink(
  spec: SankeyChartSpec,
  link: SankeyLink,
): Provenance {
  return link.provenance ?? spec.provenance;
}

export function summariseSankey(spec: SankeyChartSpec): SankeySummary {
  const incoming = new Map<string, number>();
  const outgoing = new Map<string, number>();
  let largestLink: SankeyLink | null = null;
  for (const link of spec.links) {
    outgoing.set(link.source, (outgoing.get(link.source) ?? 0) + link.value);
    incoming.set(link.target, (incoming.get(link.target) ?? 0) + link.value);
    if (!largestLink || link.value > largestLink.value) largestLink = link;
  }
  const sourceTotal = spec.nodes
    .filter((node) => !incoming.has(node.id))
    .reduce((sum, node) => sum + (outgoing.get(node.id) ?? 0), 0);
  const sinkTotal = spec.nodes
    .filter((node) => !outgoing.has(node.id))
    .reduce((sum, node) => sum + (incoming.get(node.id) ?? 0), 0);
  return { sourceTotal, sinkTotal, largestLink };
}

export function validateSankeySpec(spec: SankeyChartSpec): SankeyValidation {
  const errors: string[] = [];
  if (
    !hasOnlyKeys(spec, [
      "schema_version",
      "id",
      "title",
      "description",
      "takeaway",
      "kind",
      "unit",
      "balance",
      "nodes",
      "links",
      "provenance",
    ])
  )
    errors.push("unknown chart field");
  if (spec.kind !== "sankey") errors.push("kind must be sankey");
  if (spec.schema_version !== 1) errors.push("unsupported schema version");
  if (!localId.test(spec.id) || spec.id.length > 64)
    errors.push("invalid chart ID");
  if (spec.nodes.length < 2 || spec.nodes.length > SANKEY_LIMITS.nodes)
    errors.push("node limit exceeded");
  if (spec.links.length < 1 || spec.links.length > SANKEY_LIMITS.links)
    errors.push("link limit exceeded");
  if (!validText(spec.title, 100)) errors.push("invalid chart title");
  if (!validText(spec.description, 500))
    errors.push("invalid chart description");
  if (!validText(spec.takeaway, 500)) errors.push("invalid chart takeaway");
  if (!validText(spec.unit, 32)) errors.push("invalid flow unit");
  if (!["conserved", "open_boundary"].includes(spec.balance))
    errors.push("invalid balance mode");
  if (!validProvenance(spec.provenance))
    errors.push("invalid chart provenance");

  const nodes = new Map<string, SankeyNodeRole>();
  for (const node of spec.nodes) {
    if (!hasOnlyKeys(node, ["id", "label", "role"]))
      errors.push(`unknown node field: ${node.id}`);
    if (!localId.test(node.id) || node.id.length > 64)
      errors.push(`invalid node ID: ${node.id}`);
    if (!validText(node.label, 100))
      errors.push(`invalid node label: ${node.id}`);
    if (
      !["source", "process", "intermediate", "sink", "residual"].includes(
        node.role,
      )
    )
      errors.push(`invalid node role: ${node.id}`);
    if (nodes.has(node.id)) errors.push(`duplicate node ID: ${node.id}`);
    nodes.set(node.id, node.role);
  }

  const linkIds = new Set<string>();
  const pairs = new Set<string>();
  const adjacency = new Map<string, string[]>();
  const incoming = new Map<string, number>();
  const outgoing = new Map<string, number>();
  for (const link of spec.links) {
    if (!hasOnlyKeys(link, ["id", "source", "target", "value", "provenance"]))
      errors.push(`unknown link field: ${link.id}`);
    if (!localId.test(link.id) || link.id.length > 64)
      errors.push(`invalid link ID: ${link.id}`);
    if (linkIds.has(link.id)) errors.push(`duplicate link ID: ${link.id}`);
    linkIds.add(link.id);
    if (!nodes.has(link.source) || !nodes.has(link.target))
      errors.push(`unknown link endpoint: ${link.id}`);
    if (link.source === link.target) errors.push(`self link: ${link.id}`);
    if (!Number.isFinite(link.value) || link.value <= 0)
      errors.push(`invalid link value: ${link.id}`);
    if (link.provenance && !validProvenance(link.provenance))
      errors.push(`invalid link provenance: ${link.id}`);
    const pair = `${link.source}\u0000${link.target}`;
    if (pairs.has(pair)) errors.push(`duplicate link endpoints: ${link.id}`);
    pairs.add(pair);
    adjacency.set(link.source, [
      ...(adjacency.get(link.source) ?? []),
      link.target,
    ]);
    incoming.set(link.target, (incoming.get(link.target) ?? 0) + link.value);
    outgoing.set(link.source, (outgoing.get(link.source) ?? 0) + link.value);
  }

  for (const node of spec.nodes) {
    if (!incoming.has(node.id) && !outgoing.has(node.id))
      errors.push(`disconnected node: ${node.id}`);
    if (node.role === "source" && incoming.has(node.id))
      errors.push(`source has incoming flow: ${node.id}`);
    if (
      (node.role === "sink" || node.role === "residual") &&
      outgoing.has(node.id)
    )
      errors.push(`terminal node has outgoing flow: ${node.id}`);
    if (
      spec.balance === "conserved" &&
      incoming.has(node.id) &&
      outgoing.has(node.id)
    ) {
      const tolerance = Math.max(1, incoming.get(node.id)!) * 1e-9;
      if (Math.abs(incoming.get(node.id)! - outgoing.get(node.id)!) > tolerance)
        errors.push(`unbalanced internal node: ${node.id}`);
    }
  }

  const visiting = new Set<string>();
  const visited = new Set<string>();
  const hasCycle = (nodeId: string): boolean => {
    if (visiting.has(nodeId)) return true;
    if (visited.has(nodeId)) return false;
    visiting.add(nodeId);
    for (const target of adjacency.get(nodeId) ?? []) {
      if (hasCycle(target)) return true;
    }
    visiting.delete(nodeId);
    visited.add(nodeId);
    return false;
  };
  if (spec.nodes.some((node) => hasCycle(node.id)))
    errors.push("cyclic flow is not supported");

  if (spec.balance === "conserved") {
    const summary = summariseSankey(spec);
    const tolerance = Math.max(1, summary.sourceTotal) * 1e-9;
    if (Math.abs(summary.sourceTotal - summary.sinkTotal) > tolerance)
      errors.push("source and sink totals do not balance");
  }

  return { valid: errors.length === 0, errors };
}

export function optionForSankey(
  spec: SankeyChartSpec,
  theme: ChartTheme = observatoryChartTheme,
  reducedMotion = false,
  locale = "en-AU",
): EChartsCoreOption {
  const validation = validateSankeySpec(spec);
  if (!validation.valid)
    throw new Error(
      `Invalid Sankey specification: ${validation.errors.join("; ")}`,
    );

  const labels = new Map(spec.nodes.map((node) => [node.id, node.label]));
  return {
    animationDuration: reducedMotion ? 0 : 460,
    tooltip: {
      trigger: "item",
      renderMode: "richText",
      backgroundColor: theme.tooltipBackground,
      borderColor: theme.tooltipBorder,
      textStyle: { color: theme.text, fontSize: 12 },
      formatter: (raw: unknown) => {
        const item = raw as {
          dataType?: string;
          data?: {
            source?: string;
            target?: string;
            value?: number;
            name?: string;
          };
          value?: number;
          name?: string;
        };
        if (
          item.dataType === "edge" &&
          item.data?.source &&
          item.data?.target
        ) {
          const value = formatNumber(item.data.value ?? 0, locale, {
            maximumFractionDigits: 2,
          });
          return `${labels.get(item.data.source) ?? item.data.source} → ${labels.get(item.data.target) ?? item.data.target}\n${value} ${spec.unit}`;
        }
        return item.name ?? item.data?.name ?? "";
      },
    },
    series: [
      {
        id: spec.id,
        type: "sankey",
        left: 12,
        right: 24,
        top: 18,
        bottom: 18,
        nodeWidth: 16,
        nodeGap: 14,
        draggable: false,
        emphasis: { focus: "adjacency" },
        layoutIterations: 32,
        label: {
          color: theme.text,
          fontSize: 12,
          lineHeight: 16,
          formatter: (item: { name: string }) =>
            labels.get(item.name) ?? item.name,
        },
        lineStyle: {
          color: "source",
          opacity: 0.36,
          curveness: 0.5,
        },
        data: spec.nodes.map((node) => ({
          name: node.id,
          label: {
            formatter: node.label,
            position:
              node.role === "sink" || node.role === "residual"
                ? "left"
                : "right",
            align:
              node.role === "sink" || node.role === "residual"
                ? "right"
                : "left",
          },
          itemStyle: {
            color: roleColours[node.role],
            borderColor: theme.text,
            borderWidth: node.role === "residual" ? 2 : 1,
            borderType: node.role === "residual" ? "dashed" : "solid",
          },
        })),
        links: spec.links.map((link) => ({
          id: link.id,
          source: link.source,
          target: link.target,
          value: link.value,
        })),
      },
    ],
  };
}
