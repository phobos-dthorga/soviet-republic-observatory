import { readFile } from "node:fs/promises";

const workspaceFiles = [
  "BriefingWorkspace.svelte",
  "MonitorWorkspace.svelte",
  "BroadcastWorkspace.svelte",
  "ExtensionsWorkspace.svelte",
  "PlanWorkspace.svelte",
  "MaterialsWorkspace.svelte",
  "PopulationWorkspace.svelte",
  "EnvironmentWorkspace.svelte",
  "MarketsWorkspace.svelte",
  "ArchiveWorkspace.svelte",
];

const taskOwners = new Map([
  ["broadcast-outcome-laboratory", "BroadcastWorkspace.svelte"],
  ["plan-editor", "PlanWorkspace.svelte"],
  ["materials-pathway-study", "MaterialsWorkspace.svelte"],
  ["materials-overlay-editor", "MaterialsWorkspace.svelte"],
  ["environment-carbon-study", "EnvironmentWorkspace.svelte"],
  ["environment-recording-management", "EnvironmentWorkspace.svelte"],
  ["markets-basket-laboratory", "MarketsWorkspace.svelte"],
  ["markets-scenario-laboratory", "MarketsWorkspace.svelte"],
  ["archive-comparison", "ArchiveWorkspace.svelte"],
]);

const failures = [];
const sources = new Map();
for (const file of workspaceFiles) {
  const path = `src/lib/workspaces/${file}`;
  const source = await readFile(path, "utf8");
  sources.set(file, source);
  if (!source.includes("WorkspaceSectionHeader")) {
    failures.push(`${path}: missing the shared workspace heading`);
  }
  if (!source.includes('level="page"')) {
    failures.push(`${path}: page title does not use the shared page level`);
  }
  if (!source.includes('class="section-list"')) {
    failures.push(`${path}: missing contained section navigation`);
  }
  if (/<header\s+class="page-heading"/.test(source)) {
    failures.push(`${path}: legacy page heading bypasses the shared layout`);
  }
}

const registry = await readFile("src/lib/tasks/workspaceTaskRoutes.ts", "utf8");
const app = await readFile("src/App.svelte", "utf8");
const scenarios = await readFile("src/lib/ui-review/scenarios.ts", "utf8");
for (const [route, owner] of taskOwners) {
  const source = sources.get(owner) ?? "";
  if (!registry.includes(`"${route}"`)) {
    failures.push(`workspace task registry: missing ${route}`);
  }
  if (!source.includes(`route="${route}"`)) {
    failures.push(`src/lib/workspaces/${owner}: missing task drawer ${route}`);
  }
  const openAction = new RegExp(`onopentask\\(\\s*"${route}"\\s*(?:,|\\))`);
  if (!openAction.test(source)) {
    failures.push(
      `src/lib/workspaces/${owner}: ${route} has no section action`,
    );
  }
  const reviewName =
    route === "broadcast-outcome-laboratory"
      ? "broadcast-outcome-task"
      : route === "plan-editor"
        ? "plan-editor-task"
        : route === "materials-pathway-study"
          ? "production-pathway"
          : route === "materials-overlay-editor"
            ? "materials-overlay-task"
            : route === "environment-carbon-study"
              ? "environment-carbon-task"
              : route === "environment-recording-management"
                ? "environment-recording-management"
                : route === "markets-basket-laboratory"
                  ? "markets-basket-task"
                  : route === "markets-scenario-laboratory"
                    ? "markets-scenario-task"
                    : "archive-comparison-task";
  if (!scenarios.includes(`"${reviewName}"`)) {
    failures.push(`UI review catalogue: missing ${reviewName}`);
  }
  if (!app.includes(`workspaceTaskTrail = ["${route}"]`)) {
    failures.push(`UI review setup: ${route} is not opened deterministically`);
  }
}

const drawer = await readFile(
  "src/lib/tasks/WorkspaceTaskDrawer.svelte",
  "utf8",
);
for (const required of [
  "data-workspace-task={route}",
  'role="dialog"',
  'aria-modal="true"',
  "max-width: 62ch",
  "overflow-y: auto",
]) {
  if (!drawer.includes(required)) {
    failures.push(`WorkspaceTaskDrawer: missing '${required}'`);
  }
}

const environment = sources.get("EnvironmentWorkspace.svelte") ?? "";
const dangerIndex = environment.indexOf('class="danger-zone"');
const destructiveDrawerIndex = environment.indexOf(
  'route="environment-recording-management"',
);
if (dangerIndex < destructiveDrawerIndex || destructiveDrawerIndex < 0) {
  failures.push(
    "EnvironmentWorkspace.svelte: destructive recording controls are not isolated in their task drawer",
  );
}

if (failures.length) {
  console.error(
    "Workspace layout audit failed:\n" +
      failures.map((item) => `- ${item}`).join("\n"),
  );
  process.exit(1);
}

console.log(
  `Workspace layout audit passed: ${workspaceFiles.length} workspaces and ${taskOwners.size} task routes follow the shared placement contract.`,
);
