import { spawn, spawnSync } from "node:child_process";
import { createHash, randomBytes } from "node:crypto";
import { createReadStream } from "node:fs";
import {
  cp,
  lstat,
  mkdir,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
import { createServer } from "node:net";

const scenarios = [
  "workspace-briefing",
  "workspace-monitor",
  "workspace-broadcast",
  "workspace-extensions",
  "workspace-materials",
  "materials-warehouse-attention",
  "workspace-population",
  "population-probe-missing",
  "archive-latest",
  "archive-historical",
  "critical-task-loading",
  "critical-task-failed",
  "dialog-language",
  "dialog-theme",
  "dialog-observation",
  "dialog-diagnostics",
  "dialog-legal",
  "dialog-research",
  "notification-error",
  "tooltip-contextual",
  "attention-cue",
  "keyboard-focus",
  "native-dropdown",
];
const [command, ...arguments_] = process.argv.slice(2);

if (command === "list") {
  rejectArguments(arguments_);
  console.log(scenarios.join("\n"));
  process.exit(0);
}
if (command !== "run" && command !== "live") usage();

let suite = command === "live" ? "smoke" : "smoke";
let liveAcknowledged = false;
for (let index = 0; index < arguments_.length; index += 1) {
  const argument = arguments_[index];
  if (argument === "--suite" && command === "run") {
    suite = arguments_[++index];
    if (!suite) fail("--suite requires 'smoke' or 'full'.");
  } else if (argument === "--acknowledge-live-data" && command === "live") {
    liveAcknowledged = true;
  } else {
    fail(`Unknown UI review option '${argument}'.`);
  }
}
if (!new Set(["smoke", "full"]).has(suite)) {
  fail("--suite must be either 'smoke' or 'full'.");
}
if (command === "live" && !liveAcknowledged) {
  fail("Live review requires --acknowledge-live-data.");
}

const tooling = await readTooling();
const binary = join(
  process.env.CARGO_TARGET_DIR ??
    join(tmpdir(), "republic-observatory-cargo-target"),
  "release",
  `republic-observatory${process.platform === "win32" ? ".exe" : ""}`,
);
await requireFile(
  binary,
  "Build the desktop binary first with 'npm run desktop:build:binary'.",
);

if (command === "live") ensureOrdinaryApplicationClosed();

const runId = `review-${Date.now().toString(36)}-${randomBytes(4).toString("hex")}`;
const reviewBase = join(tmpdir(), "republic-observatory-ui-review");
await cleanupStaleReviewRoots(reviewBase);
const reviewRoot = join(reviewBase, runId);
const dataRoot = join(reviewRoot, "data");
const artifactRoot = resolve("artifacts/native-ui-review", runId);
await mkdir(dataRoot, { recursive: true });
await mkdir(artifactRoot, { recursive: true });
await writeFile(
  join(reviewRoot, ".observatory-ui-review.json"),
  `${JSON.stringify({ run_id: runId, data_state: command === "live" ? "live" : "fixture" })}\n`,
  "utf8",
);

let sourceHashes = new Map();
if (command === "live") {
  const sourceRoot = appDataDirectory();
  sourceHashes = await hashReviewSources(sourceRoot);
  await copyLiveState(sourceRoot, dataRoot);
  await writeFile(
    join(artifactRoot, "SENSITIVE-LIVE-REVIEW.txt"),
    "This local artifact came from an explicitly requested clone of current app data. Review it before sharing.\n",
    "utf8",
  );
}

const port = await ephemeralPort();
const edgeDirectory = tooling.edge_driver_path
  ? dirname(tooling.edge_driver_path)
  : null;
const environment = {
  ...process.env,
  UI_REVIEW_SUITE: suite,
  UI_REVIEW_STATE: command === "live" ? "live" : "fixture",
  UI_REVIEW_RUN_ID: runId,
  UI_REVIEW_ROOT: reviewRoot,
  UI_REVIEW_ARTIFACT_ROOT: artifactRoot,
  UI_REVIEW_BINARY: binary,
  UI_REVIEW_TAURI_DRIVER: tooling.tauri_driver_path,
  UI_REVIEW_NATIVE_DRIVER: tooling.edge_driver_path,
  UI_REVIEW_DRIVER_PORT: String(port),
  PATH: edgeDirectory
    ? `${edgeDirectory}${process.platform === "win32" ? ";" : ":"}${process.env.PATH ?? ""}`
    : process.env.PATH,
};

let exitCode = 1;
let runFailure = null;
try {
  exitCode = await runWdio(environment, artifactRoot, [
    resolve(process.cwd()),
    reviewRoot,
    process.env.USERPROFILE,
  ]);
  if (command === "live") {
    const after = await hashReviewSources(appDataDirectory());
    if (!sameHashes(sourceHashes, after)) {
      throw new Error(
        "The source app-data files changed during live review; integrity check failed.",
      );
    }
  }
} catch (error) {
  runFailure = error instanceof Error ? error.message : String(error);
  console.error(redact(runFailure, [resolve(process.cwd()), reviewRoot]));
} finally {
  await collectAppDiagnostics(reviewRoot, artifactRoot, [
    resolve(process.cwd()),
    reviewRoot,
    process.env.USERPROFILE,
  ]);
  await removeReviewRoot(reviewBase, reviewRoot);
}

console.log(
  exitCode === 0 && !runFailure
    ? `Native UI review passed. Artifacts: ${artifactRoot}`
    : `Native UI review failed. Artifacts: ${artifactRoot}`,
);
process.exit(exitCode === 0 && !runFailure ? 0 : 1);

async function cleanupStaleReviewRoots(reviewBase) {
  await mkdir(reviewBase, { recursive: true });
  const entries = await readdir(reviewBase, { withFileTypes: true }).catch(
    () => [],
  );
  const cutoff = Date.now() - 24 * 60 * 60 * 1_000;
  for (const entry of entries) {
    if (!entry.isDirectory() || !safeRunId(entry.name)) continue;
    const root = join(reviewBase, entry.name);
    const rootStatus = await lstat(root).catch(() => null);
    const markerPath = join(root, ".observatory-ui-review.json");
    const markerStatus = await lstat(markerPath).catch(() => null);
    if (
      !rootStatus?.isDirectory() ||
      rootStatus.isSymbolicLink() ||
      !markerStatus?.isFile() ||
      markerStatus.isSymbolicLink() ||
      markerStatus.mtimeMs >= cutoff
    )
      continue;
    const marker = await readMarker(markerPath);
    if (marker?.run_id !== entry.name) continue;
    await removeReviewRoot(reviewBase, root);
  }
}

async function removeReviewRoot(reviewBase, reviewRoot) {
  const base = resolve(reviewBase);
  const target = resolve(reviewRoot);
  const status = await lstat(target).catch(() => null);
  if (
    dirname(target) !== base ||
    !target.startsWith(`${base}${sep}`) ||
    !status?.isDirectory() ||
    status.isSymbolicLink()
  ) {
    if (status) throw new Error("Refused unsafe native UI review cleanup.");
    return;
  }
  await rm(target, { recursive: true, force: true });
}

async function readMarker(markerPath) {
  try {
    const marker = JSON.parse(await readFile(markerPath, "utf8"));
    if (
      !marker ||
      Object.keys(marker).sort().join(",") !== "data_state,run_id" ||
      !safeRunId(marker.run_id) ||
      !new Set(["fixture", "live"]).has(marker.data_state)
    )
      return null;
    return marker;
  } catch {
    return null;
  }
}

async function collectAppDiagnostics(reviewRoot, artifactRoot, privateValues) {
  const source = join(
    reviewRoot,
    "data",
    "republic-observatory-diagnostics.jsonl",
  );
  const status = await lstat(source).catch(() => null);
  if (!status?.isFile() || status.isSymbolicLink()) return;
  const document = (await readFile(source, "utf8")).slice(-250_000);
  await writeFile(
    join(artifactRoot, "app-diagnostics.jsonl"),
    redact(document, privateValues),
    "utf8",
  );
}

function safeRunId(value) {
  return (
    typeof value === "string" &&
    value.length >= 8 &&
    value.length <= 64 &&
    /^[A-Za-z0-9_-]+$/.test(value)
  );
}

async function readTooling() {
  const manifestPath = resolve(".tools/native-ui-review/tooling.json");
  try {
    const value = JSON.parse(await readFile(manifestPath, "utf8"));
    if (
      value.schema_version !== 1 ||
      value.tauri_driver_version !== "2.0.6" ||
      typeof value.tauri_driver_path !== "string"
    ) {
      throw new Error("incompatible tooling manifest");
    }
    await requireFile(value.tauri_driver_path, setupMessage());
    if (process.platform === "win32") {
      await requireFile(value.edge_driver_path, setupMessage());
    }
    return value;
  } catch (error) {
    fail(
      `${setupMessage()} (${error instanceof Error ? error.message : error})`,
    );
  }
}

async function copyLiveState(sourceRoot, destinationRoot) {
  const sourceRootStatus = await lstat(sourceRoot).catch(() => null);
  if (!sourceRootStatus?.isDirectory() || sourceRootStatus.isSymbolicLink()) {
    fail("The application data source is not a safe ordinary directory.");
  }
  const entries = await readdir(sourceRoot, { withFileTypes: true }).catch(
    () => [],
  );
  const allowedFiles = new Set([
    "republic-observatory.sqlite3",
    "republic-observatory.sqlite3-wal",
    "republic-observatory.sqlite3-shm",
    "republic-observatory.duckdb",
    "republic-observatory.duckdb.wal",
  ]);
  for (const entry of entries) {
    if (entry.isFile() && allowedFiles.has(entry.name)) {
      const source = join(sourceRoot, entry.name);
      const sourceStatus = await lstat(source);
      if (!sourceStatus.isFile() || sourceStatus.isSymbolicLink()) {
        fail(`Live review refused the unsafe source entry '${entry.name}'.`);
      }
      await cp(source, join(destinationRoot, entry.name));
    } else if (entry.isDirectory() && entry.name === "compatibility") {
      const compatibilityRoot = join(sourceRoot, entry.name);
      const compatibilityStatus = await lstat(compatibilityRoot).catch(
        () => null,
      );
      const source = join(compatibilityRoot, "local.rocompat.json");
      const sourceStatus = await lstat(source).catch(() => null);
      if (
        compatibilityStatus?.isDirectory() &&
        !compatibilityStatus.isSymbolicLink() &&
        sourceStatus?.isFile() &&
        !sourceStatus.isSymbolicLink()
      ) {
        const destination = join(destinationRoot, entry.name);
        await mkdir(destination, { recursive: true });
        await cp(source, join(destination, "local.rocompat.json"));
      }
    }
  }
}

async function hashReviewSources(sourceRoot) {
  const result = new Map();
  const entries = await readdir(sourceRoot, { withFileTypes: true }).catch(
    () => [],
  );
  for (const entry of entries) {
    if (!entry.isFile() || !/\.(?:sqlite3|duckdb|wal|shm)$/.test(entry.name))
      continue;
    const path = join(sourceRoot, entry.name);
    result.set(entry.name, await hashFile(path));
  }
  return result;
}

function hashFile(path) {
  return new Promise((resolveHash, rejectHash) => {
    const hash = createHash("sha256");
    const stream = createReadStream(path);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.once("error", rejectHash);
    stream.once("end", () => resolveHash(hash.digest("hex")));
  });
}

function sameHashes(first, second) {
  return (
    first.size === second.size &&
    [...first].every(([name, hash]) => second.get(name) === hash)
  );
}

function ensureOrdinaryApplicationClosed() {
  if (process.platform !== "win32") return;
  const result = spawnSync(
    "tasklist",
    ["/FI", "IMAGENAME eq republic-observatory.exe", "/FO", "CSV", "/NH"],
    { encoding: "utf8", windowsHide: true },
  );
  if (/republic-observatory\.exe/i.test(result.stdout ?? "")) {
    fail(
      "Close Republic Observatory before starting clone-based live review. The CLI will not terminate it.",
    );
  }
}

async function runWdio(environment, artifactRoot, privateValues) {
  const runner = resolve("node_modules/tsx/dist/cli.mjs");
  const reviewScript = resolve("tests/native-ui/run-native-review.ts");
  const child = spawn(process.execPath, [runner, reviewScript], {
    cwd: process.cwd(),
    env: environment,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const logs = [];
  let logLength = 0;
  const accept = (chunk, stream) => {
    const sanitized = redact(chunk.toString(), privateValues);
    stream.write(sanitized);
    logs.push(sanitized);
    logLength += sanitized.length;
    while (logLength > 250_000 && logs.length > 1) {
      logLength -= logs.shift().length;
    }
  };
  child.stdout.on("data", (chunk) => accept(chunk, process.stdout));
  child.stderr.on("data", (chunk) => accept(chunk, process.stderr));
  const code = await new Promise((resolveExit, rejectExit) => {
    child.once("error", rejectExit);
    child.once("exit", (status) => resolveExit(status ?? 1));
  });
  await writeFile(
    join(artifactRoot, "native-review.log"),
    logs.join(""),
    "utf8",
  );
  return code;
}

function redact(value, privateValues) {
  return privateValues
    .filter(Boolean)
    .sort((first, second) => second.length - first.length)
    .reduce(
      (result, item) => result.split(item).join("<redacted-path>"),
      value,
    );
}

function appDataDirectory() {
  if (process.platform === "win32") {
    return join(
      process.env.LOCALAPPDATA ?? fail("LOCALAPPDATA is unavailable."),
      "org.phobosdthorga.republic-observatory",
    );
  }
  fail("Live native review is currently supported on Windows only.");
}

function ephemeralPort() {
  return new Promise((resolvePort, rejectPort) => {
    const server = createServer();
    server.once("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((error) => (error ? rejectPort(error) : resolvePort(port)));
    });
  });
}

async function requireFile(path, remediation) {
  if (!path || !(await stat(path).catch(() => null))?.isFile())
    fail(remediation);
}

function rejectArguments(values) {
  if (values.length > 0) fail(`Unexpected UI review argument '${values[0]}'.`);
}

function setupMessage() {
  return "Native UI review tooling is missing or incompatible. Run 'npm run ui:review:setup'.";
}

function usage() {
  fail(
    "Use 'npm run ui:review -- list', 'run --suite smoke|full', or 'live --acknowledge-live-data'.",
  );
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
