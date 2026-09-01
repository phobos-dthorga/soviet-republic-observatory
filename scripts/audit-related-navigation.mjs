import { readFileSync } from "node:fs";

const violations = [];
const registryPath = "src/lib/navigation/relatedData.ts";
const registry = readFileSync(registryPath, "utf8");
const chartTypes = readFileSync("src/lib/charts/types.ts", "utf8");
const chartAdapter = readFileSync(
  "src/lib/charts/ObservatoryChart.svelte",
  "utf8",
);
const analysisPack = readFileSync("src/lib/extensions/analysisPack.ts", "utf8");
const analysisRuntime = readFileSync("src/lib/extensions/runtime.ts", "utf8");
const analysisSchema = readFileSync(
  "schemas/analysis-pack-v1.schema.json",
  "utf8",
);
const appCss = readFileSync("src/app.css", "utf8");
const broadcastWorkspace = readFileSync(
  "src/lib/workspaces/BroadcastWorkspace.svelte",
  "utf8",
);
const productionRoute = readFileSync(
  "src/lib/workspaces/ProductionRouteLaboratory.svelte",
  "utf8",
);

for (const token of [
  "http://",
  "https://",
  "querySelector",
  "callback",
  "command:",
  "selector:",
  "url:",
]) {
  if (registry.toLowerCase().includes(token.toLowerCase())) {
    fail(registryPath, `The allowlisted registry contains '${token}'.`);
  }
}

const publicChartContracts = chartTypes.match(
  /export type ChartSpec = \{[\s\S]*?\n\};[\s\S]*?export type SankeyChartSpec = \{[\s\S]*?\n\};/,
)?.[0];
if (!publicChartContracts) {
  fail(
    "src/lib/charts/types.ts",
    "The public chart contracts could not be audited.",
  );
} else if (
  /\b(?:navigation|route|selector|command|callback|url)\b/i.test(
    publicChartContracts,
  )
) {
  fail(
    "src/lib/charts/types.ts",
    "Public chart data cannot carry navigation authority.",
  );
}

for (const [path, source] of [
  ["src/lib/extensions/analysisPack.ts", analysisPack],
  ["src/lib/extensions/runtime.ts", analysisRuntime],
  ["schemas/analysis-pack-v1.schema.json", analysisSchema],
]) {
  if (/\b(?:navigation|route|selector|command|callback)\b/i.test(source)) {
    fail(path, "Analysis Packs cannot author navigation capabilities.");
  }
}
if (!analysisSchema.includes('"additionalProperties": false')) {
  fail(
    "schemas/analysis-pack-v1.schema.json",
    "The Analysis Pack schema must continue to reject unknown fields.",
  );
}

for (const required of [
  'value === "eletronics"',
  'value === "ecomponents"',
  "electronicsEconomyDestinations",
  'workspaceDestination("broadcast", "receivers"',
]) {
  if (!registry.includes(required)) {
    fail(
      registryPath,
      `Electronics-economy navigation is missing '${required}'.`,
    );
  }
}
if (
  registry
    .match(/export function electronicsEconomyDestinations[\s\S]*?\n\}/)?.[0]
    ?.includes('workspaceDestination("population"')
) {
  fail(
    registryPath,
    "Receiver ownership cannot link to demographics without a direct join.",
  );
}
if (!broadcastWorkspace.includes("electronicsEconomyDestinations")) {
  fail(
    "src/lib/workspaces/BroadcastWorkspace.svelte",
    "Broadcast must use the typed electronics-economy registry.",
  );
}
if (!productionRoute.includes("output_resource_id: outputResourceId")) {
  fail(
    "src/lib/workspaces/ProductionRouteLaboratory.svelte",
    "Related Materials navigation must filter recipes by exact output resource.",
  );
}

for (const required of [
  "chart-data-ledger",
  "chart-sankey-flow-table",
  "requestRelatedView",
  "related-nav-open",
]) {
  if (!chartAdapter.includes(required)) {
    fail(
      "src/lib/charts/ObservatoryChart.svelte",
      `Clickable charts require the shared accessible equivalent '${required}'.`,
    );
  }
}

if (
  /document\.documentElement\.scroll|document\.body\.scroll|window\.scrollTo|scrollIntoView/.test(
    readSources(),
  )
) {
  fail(
    "src",
    "Related and section navigation must scroll only the contained workspace canvas.",
  );
}
const narrowLayout =
  appCss.match(/@media \(max-width: 860px\) \{[\s\S]*?\n\}/)?.[0] ?? "";
if (/body\s*\{[\s\S]*?overflow:\s*auto/.test(narrowLayout)) {
  fail("src/app.css", "Narrow layouts cannot enable document-level scrolling.");
}

const componentByWorkspace = {
  briefing: "src/lib/workspaces/BriefingWorkspace.svelte",
  monitor: "src/lib/workspaces/MonitorWorkspace.svelte",
  broadcast: "src/lib/workspaces/BroadcastWorkspace.svelte",
  extensions: "src/lib/workspaces/ExtensionsWorkspace.svelte",
  plan: "src/lib/workspaces/PlanWorkspace.svelte",
  materials: "src/lib/workspaces/MaterialsWorkspace.svelte",
  population: "src/lib/workspaces/PopulationWorkspace.svelte",
  markets: "src/lib/workspaces/MarketsWorkspace.svelte",
  archive: "src/lib/workspaces/ArchiveWorkspace.svelte",
};
const sectionBlock = registry.match(
  /export const workspaceSections = \{([\s\S]*?)\n\} as const;/,
)?.[1];
if (!sectionBlock) {
  fail(registryPath, "Workspace section destinations could not be audited.");
} else {
  for (const [workspace, path] of Object.entries(componentByWorkspace)) {
    const match = sectionBlock.match(
      new RegExp(`${workspace}:\\s*\\[([\\s\\S]*?)\\](?:,|\\n)`),
    );
    if (!match) {
      fail(
        registryPath,
        `Workspace '${workspace}' has no allowlisted sections.`,
      );
      continue;
    }
    const sections = [...match[1].matchAll(/"([^"]+)"/g)].map(
      (item) => item[1],
    );
    if (new Set(sections).size !== sections.length) {
      fail(
        registryPath,
        `Workspace '${workspace}' contains a duplicate destination.`,
      );
    }
    const component = readFileSync(path, "utf8");
    for (const section of sections) {
      if (!component.includes(`id="${section}"`)) {
        fail(path, `Destination '${section}' cannot receive focus.`);
      }
    }
  }
}

if (violations.length > 0) {
  console.error("Related-data navigation audit failed:");
  for (const violation of violations) console.error(`- ${violation}`);
  process.exit(1);
}

console.log(
  "Related-data navigation audit passed: destinations are typed, focusable, contained, accessible, and host-owned.",
);

function readSources() {
  return [
    "src/App.svelte",
    "src/lib/navigation/containedSectionNavigation.ts",
    "src/lib/workspaces/MaterialsWorkspace.svelte",
  ]
    .map((path) => readFileSync(path, "utf8"))
    .join("\n");
}

function fail(path, message) {
  violations.push(`${path}: ${message}`);
}
