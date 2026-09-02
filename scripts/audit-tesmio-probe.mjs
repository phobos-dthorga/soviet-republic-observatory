import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve("research/tesmioloader-probe");
const file = resolve(root, "observatory_probe.cpp");
const source = readFileSync(file, "utf8");
const probeConfiguration = readFileSync(
  resolve(root, "observatory_probe.ini"),
  "utf8",
);
const observationOnlyConfiguration = readFileSync(
  resolve(root, "tesmioloader.observation-only.ini.example"),
  "utf8",
);
const verifier = readFileSync(
  resolve(root, "verify-observation-only.ps1"),
  "utf8",
);
const downloader = readFileSync(
  resolve("src-tauri/src/research_source_download.rs"),
  "utf8",
);
const setupService = readFileSync(
  resolve("src-tauri/src/research_setup.rs"),
  "utf8",
);
const commands = readFileSync(resolve("src-tauri/src/commands.rs"), "utf8");
const sessionBuild = readFileSync(
  resolve(root, "build-observation-session.ps1"),
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
  'record_type\\":\\"resource_registry',
  'record_type\\":\\"resource_entry',
  "RVA_RESOURCE_VECTOR 0x9E11C0",
  "RVA_GAME_STATE 0x9D4F10",
  "RESOURCE_STRIDE 0x340",
  "RESOURCE_MAX_RECORDS 512",
  "TESTED_EXE_SIZE 11128832u",
  "ReadablePtr",
  "FaultFilter",
  'record_type\\\":\\\"probe_status',
  'probe_version\\\":\\\"0.2.3',
  "SetFilePointerEx(g_output, beginning, NULL, FILE_BEGIN)",
  "SetEndOfFile(g_output)",
  "rolled its bounded report forward",
];
for (const marker of required) {
  if (!source.includes(marker))
    failures.push(`missing required marker ${marker}`);
}
for (const setting of [
  "samples = 16",
  "every_days = 7",
  "max_records = 4096",
]) {
  if (!probeConfiguration.includes(setting)) {
    failures.push(`bounded probe defaults are missing ${setting}`);
  }
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

const requiredDownloadBoundary = [
  'const DOWNLOAD_HOST: &str = "codeload.github.com"',
  'const DOWNLOAD_PATH_PREFIX: &str = "/MaxLegend/TesmioLoader/zip/"',
  "Policy::none()",
  ".https_only(true)",
  "Duration::from_secs(30)",
  "const MAX_TRANSFER_BYTES: u64 = 8 * 1024 * 1024",
  '"src/tesmio_plugin.h"',
  '"src/tesmio_api.h"',
  '"src/tesmioloader.cpp"',
  '"src/tesmiolauncher.cpp"',
  '"LICENSE"',
  '"observatory-provenance.json"',
];
for (const marker of requiredDownloadBoundary) {
  if (!downloader.includes(marker)) {
    failures.push(`reviewed-source downloader is missing ${marker}`);
  }
}
for (const marker of [
  'pub const REVIEWED_TESMIO_REVISION: &str = "3baa141f9f08921aea9c95f0a400289cabd9960a"',
  "pub const RESEARCH_NOTICE_REVISION: u32 = 4",
  '"d886ac6550dd84031ee2ed3afab13a7f75e4ddf920d23183b93395440d3cff49"',
  '"33c9fae4acb1041708c7b1b4675b0eb4740f0af737e7a1968c0acb0c325fff3c"',
  "reviewed_header_hash",
]) {
  if (!setupService.includes(marker)) {
    failures.push(`research setup is missing ${marker}`);
  }
}
for (const marker of [
  'const MANAGED_SESSION_DIRECTORY: &str = "observatory"',
  "writes_save_data: false",
  "save_manifest = 0",
  "menu_patch = 0",
  "version_check = 1",
  "managed_session_is_valid",
  "observation_only_configuration_matches",
  "observatory_probe.provenance.json",
  "probe_source_contract_hash",
]) {
  if (!setupService.includes(marker)) {
    failures.push(`checked-session preparation is missing ${marker}`);
  }
}
for (const marker of [
  "game_directory_write_confirmed: bool",
  "running_game_memory_confirmed: bool",
]) {
  if (!commands.includes(marker)) {
    failures.push(`native consent boundary is missing ${marker}`);
  }
}
for (const marker of [
  "tesmioloader.cpp",
  "tesmiolauncher.cpp",
  "tesmioloader.dll",
  "tesmiolauncher.exe",
]) {
  if (!sessionBuild.includes(marker)) {
    failures.push(`fixed checked-session build is missing ${marker}`);
  }
}
if (
  /pub(?:\(crate\))?\s+fn\s+download_reviewed_source\s*\([^)]*(?:url|uri)/s.test(
    downloader,
  )
) {
  failures.push(
    "reviewed-source downloader exposes an arbitrary URL parameter",
  );
}

if (failures.length) {
  console.error("Tesmio probe source audit failed:");
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exit(1);
}

console.log(
  "Tesmio audit passed: the probe has no known save-write surface; source, game-folder preparation, and live launch remain pinned, checked, and separately consented.",
);
