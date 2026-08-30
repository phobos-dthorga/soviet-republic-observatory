import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve("research/tesmioloader-probe");
const file = resolve(root, "observatory_probe.cpp");
const source = readFileSync(file, "utf8");
const observationOnlyConfiguration = readFileSync(
  resolve(root, "tesmioloader.observation-only.ini.example"),
  "utf8",
);
const verifier = readFileSync(
  resolve(root, "verify-observation-only.ps1"),
  "utf8",
);
const failures = [];

const required = [
  "SPDX-License-Identifier: GPL-3.0-only",
  "org.republic-observatory.tesmio-readonly",
  "read_only",
  'writes_game_state\\":false',
  'writes_save_data\\":false',
  'writes_observatory_databases\\":false',
  'network_access\\":false',
  "ReadablePtr",
  "FaultFilter",
];
for (const marker of required) {
  if (!source.includes(marker))
    failures.push(`missing required marker ${marker}`);
}

const actualPatchCalls = [...source.matchAll(/\bPatchIat\s*\(/g)].length;
if (actualPatchCalls !== 1) {
  failures.push(
    `expected exactly one chained IAT observation hook; found ${actualPatchCalls}`,
  );
}

const forbidden = [
  ["inline hook", /\bInstallInlineHook\s*\(/],
  ["near executable allocation", /\bAllocNear\s*\(/],
  ["memory protection mutation", /\bVirtualProtect\s*\(/],
  ["cross-process memory write", /\bWriteProcessMemory\s*\(/],
  ["socket API", /\b(?:socket|connect|WinHttpOpen|InternetOpenA?)\s*\(/],
  ["database vocabulary", /\b(?:sqlite|duckdb|INSERT\s+INTO|UPDATE\s+\w+)\b/i],
  ["save archive target", /(?:stats\.ini|\.zip|\.sav|media_soviet)/i],
  [
    "arbitrary output path setting",
    /configString\([^\n]+(?:path|file|directory)/i,
  ],
];
for (const [label, pattern] of forbidden) {
  if (pattern.test(source)) failures.push(`forbidden ${label}`);
}

const requiredObservationOnlySettings = [
  "trace_reads = 0",
  "log_game = 0",
  "vfs = 0",
  "probe_map = 0",
  "probe_texel = 0",
  "save_manifest = 0",
  "plugins = 1",
  "menu_patch = 0",
  "version_check = 1",
  "observatory_probe = 1",
];
for (const setting of requiredObservationOnlySettings) {
  if (!observationOnlyConfiguration.includes(setting)) {
    failures.push(`observation-only baseline is missing ${setting}`);
  }
}
for (const marker of [
  "plugins must contain only observatory_probe.dll",
  "save_manifest",
  "version_check",
]) {
  if (!verifier.includes(marker)) {
    failures.push(`observation-only verifier is missing ${marker}`);
  }
}

if (failures.length) {
  console.error("Tesmio probe source audit failed:");
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exit(1);
}

console.log(
  "Tesmio probe audit passed: one chained observation hook, fixed output, no known game/save/database/network write surface, and a fail-closed observation-only loader profile.",
);
