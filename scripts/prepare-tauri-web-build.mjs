import { spawnSync } from "node:child_process";
import { readFile, unlink } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const entryPoint = resolve("dist/index.html");
const handoffToken = process.env.OBSERVATORY_PREBUILT_WEB_TOKEN;

if (handoffToken) {
  if (!/^[a-f0-9]{64}$/.test(handoffToken)) {
    throw new Error("The audited web-artifact handoff token is malformed.");
  }
  const handoffPath = resolve(
    `dist/.observatory-release-gate-${handoffToken}.json`,
  );
  const handoff = JSON.parse(await readFile(handoffPath, "utf8"));
  if (
    handoff.schema_version !== 1 ||
    handoff.token !== handoffToken ||
    handoff.entry_point !== "dist/index.html"
  ) {
    throw new Error(
      "The audited web-artifact handoff is invalid or belongs to another release-gate run.",
    );
  }
  await readFile(entryPoint);
  await unlink(handoffPath);
  console.log(
    "Reusing the web artifact that passed the browser interface audit.",
  );
  process.exit(0);
}

console.log(
  "Preparing web assets for a binary-only build. Use 'npm run desktop:build' for the complete final gate.",
);
const executable = process.platform === "win32" ? process.execPath : "npm";
const arguments_ =
  process.platform === "win32"
    ? [
        resolve(
          dirname(process.execPath),
          "node_modules",
          "npm",
          "bin",
          "npm-cli.js",
        ),
        "run",
        "build:web",
      ]
    : ["run", "build:web"];
const result = spawnSync(executable, arguments_, {
  stdio: "inherit",
  shell: false,
  env: process.env,
});

if (result.error) throw result.error;
process.exit(result.status ?? 1);
