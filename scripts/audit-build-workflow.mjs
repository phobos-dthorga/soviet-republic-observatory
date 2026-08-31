import { readFile } from "node:fs/promises";
import { releaseGatePhases } from "./release-gate-workflow.mjs";

const packageJson = JSON.parse(await readFile("package.json", "utf8"));
const tauriConfig = JSON.parse(
  await readFile("src-tauri/tauri.conf.json", "utf8"),
);
const scripts = packageJson.scripts ?? {};

const requiredScripts = new Map([
  ["build", "npm run verify:browser"],
  ["desktop:build", "npm run verify:release"],
  [
    "verify:fast",
    "npm run format:check && npm run rust:format:check && npm run check && npm test && npm run rust:check",
  ],
  ["verify:browser", "npm run build:web && npm run audit:ui"],
  ["verify:release", "node scripts/run-release-gate.mjs"],
]);
for (const [name, expected] of requiredScripts) {
  if (scripts[name] !== expected) {
    fail(`package script '${name}' must remain '${expected}'.`);
  }
}

if (
  tauriConfig.build?.beforeBuildCommand !==
  "node scripts/prepare-tauri-web-build.mjs"
) {
  fail(
    "Tauri beforeBuildCommand must prepare web assets without running the complete browser audit.",
  );
}

const phaseIds = releaseGatePhases.map((phase) => phase.id);
const expectedPhaseIds = [
  "fast-contracts",
  "rust-tests",
  "rust-clippy",
  "browser-interface",
  "desktop-package",
  "native-smoke",
];
if (JSON.stringify(phaseIds) !== JSON.stringify(expectedPhaseIds)) {
  fail(
    `The final release gate order changed. Expected ${expectedPhaseIds.join(", ")}.`,
  );
}

const packageIndex = phaseIds.indexOf("desktop-package");
if (
  packageIndex < phaseIds.indexOf("fast-contracts") ||
  packageIndex < phaseIds.indexOf("browser-interface") ||
  packageIndex > phaseIds.indexOf("native-smoke")
) {
  fail(
    "Desktop packaging must follow fast/browser checks and precede native smoke.",
  );
}
if (releaseGatePhases[packageIndex]?.reuseAuditedWeb !== true) {
  fail("Desktop packaging must use the one-use audited web-artifact handoff.");
}

console.log(
  "Build workflow audit passed: fast, browser, package, and native responsibilities remain separated.",
);

function fail(message) {
  console.error(`Build workflow audit failed: ${message}`);
  process.exit(1);
}
