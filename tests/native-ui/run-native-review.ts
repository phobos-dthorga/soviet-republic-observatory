import { spawn, type ChildProcess } from "node:child_process";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { createTauriCapabilities } from "@wdio/tauri-service";
import { remote } from "webdriverio";
import { runNativeReview } from "./native-review";

const binary = required("UI_REVIEW_BINARY");
const reviewRoot = required("UI_REVIEW_ROOT");
const runId = required("UI_REVIEW_RUN_ID");
const dataState = required("UI_REVIEW_STATE");
const port = Number(required("UI_REVIEW_DRIVER_PORT"));
const artifactRoot = required("UI_REVIEW_ARTIFACT_ROOT");
if (!Number.isSafeInteger(port) || port < 1024 || port > 65_535) {
  throw new Error("The native UI review driver port is invalid.");
}

const appArgs = [
  "--ui-review",
  `--ui-review-run=${runId}`,
  `--ui-review-root=${reviewRoot}`,
  `--ui-review-state=${dataState}`,
];
const capabilities = createTauriCapabilities(binary, {
  appArgs,
  tauriDriverPort: port,
  logLevel: "info",
  commandTimeout: 30_000,
  startTimeout: 60_000,
  driverProvider: "external",
  autoInstallTauriDriver: false,
});
const driver = startExternalDriver(
  required("UI_REVIEW_TAURI_DRIVER"),
  required("UI_REVIEW_NATIVE_DRIVER"),
  port,
  artifactRoot,
);
let client: WebdriverIO.Browser | undefined;
try {
  await waitForDriver(driver, port);
  client = await remote({
    hostname: "127.0.0.1",
    port,
    logLevel: "warn",
    connectionRetryTimeout: 90_000,
    connectionRetryCount: 2,
    capabilities: {
      "tauri:options": capabilities["tauri:options"],
    },
  });
  await runNativeReview(client);
} finally {
  if (client) await client.deleteSession().catch(() => undefined);
  await stopDriver(driver);
}

function startExternalDriver(
  driverPath: string,
  nativeDriverPath: string,
  driverPort: number,
  logRoot: string,
): ChildProcess {
  const child = spawn(
    driverPath,
    ["--port", String(driverPort), "--native-driver", nativeDriverPath],
    {
      cwd: process.cwd(),
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  const output: string[] = [];
  for (const stream of [child.stdout, child.stderr]) {
    stream?.on("data", (chunk) => {
      const value = chunk.toString();
      output.push(value);
      while (output.join("").length > 100_000) output.shift();
    });
  }
  child.once("exit", () => {
    void writeFile(join(logRoot, "tauri-driver.log"), output.join(""), "utf8");
  });
  return child;
}

async function waitForDriver(
  child: ChildProcess,
  driverPort: number,
): Promise<void> {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(
        `tauri-driver exited before becoming ready (${child.exitCode}).`,
      );
    }
    try {
      const response = await fetch(`http://127.0.0.1:${driverPort}/status`, {
        signal: AbortSignal.timeout(1_000),
      });
      if (response.ok) return;
    } catch {
      // The bounded retry loop handles normal driver startup delay.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
  }
  throw new Error("tauri-driver did not become ready within 30 seconds.");
}

async function stopDriver(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null) return;
  child.kill();
  await Promise.race([
    new Promise<void>((resolveExit) => child.once("exit", () => resolveExit())),
    new Promise<void>((resolveDelay) => setTimeout(resolveDelay, 5_000)),
  ]);
  if (child.exitCode === null && child.pid && process.platform === "win32") {
    const cleanup = spawn("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
      windowsHide: true,
      stdio: "ignore",
    });
    await new Promise<void>((resolveExit) =>
      cleanup.once("exit", () => resolveExit()),
    );
  }
}

function required(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`Missing native UI review setting ${name}.`);
  return value;
}
