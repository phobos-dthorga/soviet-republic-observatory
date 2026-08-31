import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdir, unlink, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { performance } from "node:perf_hooks";
import { releaseGatePhases } from "./release-gate-workflow.mjs";

const arguments_ = process.argv.slice(2);
if (arguments_.some((argument) => argument !== "--plan")) {
  throw new Error(
    `Unknown release-gate option '${arguments_.find((argument) => argument !== "--plan")}'.`,
  );
}
if (arguments_.filter((argument) => argument === "--plan").length > 1) {
  throw new Error("--plan may be provided only once.");
}

if (arguments_.includes("--plan")) {
  console.log("Final release gate plan (the desktop package is created once):");
  for (const [index, phase] of releaseGatePhases.entries()) {
    console.log(`${index + 1}. ${phase.label} [${phase.id}]`);
  }
  process.exit(0);
}

const startedAt = new Date();
const started = performance.now();
const results = [];
let failedPhase = null;

console.log(
  "Starting the final release gate. Expensive desktop packaging begins only after all earlier checks pass.",
);

for (const [index, phase] of releaseGatePhases.entries()) {
  const phaseStarted = performance.now();
  console.log(
    `\n[${index + 1}/${releaseGatePhases.length}] ${phase.label} (${phase.id})`,
  );
  let handoff = null;
  let exitCode = 1;
  let errorMessage = null;
  try {
    handoff = phase.reuseAuditedWeb ? await createWebHandoff() : null;
    exitCode = await runPhase(
      phase,
      handoff ? { OBSERVATORY_PREBUILT_WEB_TOKEN: handoff.token } : undefined,
    );
  } catch (error) {
    errorMessage = error instanceof Error ? error.message : String(error);
    console.error(`${phase.label} could not run: ${errorMessage}`);
  } finally {
    if (handoff) await unlink(handoff.path).catch(() => undefined);
  }
  const elapsedSeconds = secondsSince(phaseStarted);
  results.push({
    id: phase.id,
    label: phase.label,
    status: exitCode === 0 ? "passed" : "failed",
    elapsed_seconds: elapsedSeconds,
    error: errorMessage,
  });
  console.log(
    `${phase.label} ${exitCode === 0 ? "passed" : "failed"} in ${formatDuration(elapsedSeconds)}.`,
  );
  if (exitCode !== 0) {
    failedPhase = phase;
    break;
  }
}

const totalSeconds = secondsSince(started);
const report = {
  schema_version: 1,
  started_at: startedAt.toISOString(),
  finished_at: new Date().toISOString(),
  status: failedPhase ? "failed" : "passed",
  failed_phase: failedPhase?.id ?? null,
  total_seconds: totalSeconds,
  phases: results,
};
const artifactRoot = resolve("artifacts/release-gate");
await mkdir(artifactRoot, { recursive: true });
await writeFile(
  resolve(artifactRoot, "last-run.json"),
  `${JSON.stringify(report, null, 2)}\n`,
  "utf8",
);

if (failedPhase) {
  console.error(
    `\nFinal release gate stopped at '${failedPhase.label}' after ${formatDuration(totalSeconds)}. No later expensive phase was run.`,
  );
  process.exit(1);
}

console.log(
  `\nFinal release gate passed in ${formatDuration(totalSeconds)}. Timing report: artifacts/release-gate/last-run.json`,
);

function runPhase(phase, environment) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(phase.command, phase.args, {
      stdio: "inherit",
      shell: process.platform === "win32",
      env: { ...process.env, ...(environment ?? {}) },
    });
    child.once("error", rejectPromise);
    child.once("exit", (code, signal) => {
      if (signal) {
        console.error(`${phase.label} was interrupted by ${signal}.`);
      }
      resolvePromise(code ?? 1);
    });
  });
}

async function createWebHandoff() {
  const token = randomBytes(32).toString("hex");
  const path = resolve(`dist/.observatory-release-gate-${token}.json`);
  await writeFile(
    path,
    `${JSON.stringify({
      schema_version: 1,
      token,
      entry_point: "dist/index.html",
    })}\n`,
    { encoding: "utf8", flag: "wx" },
  );
  return { path, token };
}

function secondsSince(start) {
  return Math.round((performance.now() - start) / 10) / 100;
}

function formatDuration(seconds) {
  if (seconds < 60) return `${seconds.toFixed(2)} s`;
  const minutes = Math.floor(seconds / 60);
  return `${minutes} min ${(seconds - minutes * 60).toFixed(1)} s`;
}
