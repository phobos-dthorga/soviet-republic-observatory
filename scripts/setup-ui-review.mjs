import { spawn, spawnSync } from "node:child_process";
import { mkdir, rm, stat, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

const root = resolve(".tools/native-ui-review");
const executable = join(
  root,
  "bin",
  `tauri-driver${process.platform === "win32" ? ".exe" : ""}`,
);

await mkdir(root, { recursive: true });
if (!(await tauriDriverInstalled(executable))) {
  await run("cargo", [
    "install",
    "tauri-driver",
    "--version",
    "2.0.6",
    "--locked",
    "--root",
    root,
  ]);
}

let edgeDriverPath = null;
let edgeVersion = null;
if (process.platform === "win32") {
  edgeVersion = detectWebViewVersion();
  if (!edgeVersion) {
    throw new Error(
      "The installed Microsoft WebView2 version could not be detected.",
    );
  }
  const major = edgeVersion.split(".")[0];
  const edgeRoot = join(root, "edge", major);
  edgeDriverPath = join(edgeRoot, "msedgedriver.exe");
  if (
    !(await expectedVersion(
      edgeDriverPath,
      new RegExp(`(?:MSEdgeDriver|Microsoft Edge WebDriver) ${major}\\.`),
    ))
  ) {
    await rm(edgeRoot, { recursive: true, force: true });
    await mkdir(edgeRoot, { recursive: true });
    const latestResponse = await fetch(
      `https://msedgedriver.microsoft.com/LATEST_RELEASE_${major}`,
    );
    if (!latestResponse.ok) {
      throw new Error(`Could not resolve Edge WebDriver ${major}.`);
    }
    const driverVersion = decodeMicrosoftVersion(
      Buffer.from(await latestResponse.arrayBuffer()),
    );
    if (!driverVersion.startsWith(`${major}.`)) {
      throw new Error(
        "Microsoft returned an incompatible Edge WebDriver version.",
      );
    }
    const architecture = process.arch === "x64" ? "win64" : "win32";
    const archivePath = join(edgeRoot, "edgedriver.zip");
    const archiveResponse = await fetch(
      `https://msedgedriver.microsoft.com/${driverVersion}/edgedriver_${architecture}.zip`,
    );
    if (!archiveResponse.ok) {
      throw new Error(`Could not download Edge WebDriver ${driverVersion}.`);
    }
    await writeFile(
      archivePath,
      Buffer.from(await archiveResponse.arrayBuffer()),
    );
    await run("tar", ["-xf", archivePath, "-C", edgeRoot]);
    await rm(archivePath, { force: true });
    if (
      !(await expectedVersion(
        edgeDriverPath,
        new RegExp(`(?:MSEdgeDriver|Microsoft Edge WebDriver) ${major}\\.`),
      ))
    ) {
      throw new Error(
        "The downloaded Edge WebDriver failed its version check.",
      );
    }
  }
}

const manifest = {
  schema_version: 1,
  tauri_driver_version: "2.0.6",
  tauri_driver_path: executable,
  edge_driver_path: edgeDriverPath,
  edge_version: edgeVersion,
};
await mkdir(dirname(join(root, "tooling.json")), { recursive: true });
await writeFile(
  join(root, "tooling.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
  "utf8",
);
console.log("Native UI review tooling is ready.");

function detectWebViewVersion() {
  const registryPaths = [
    "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKCU\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
  ];
  for (const registryPath of registryPaths) {
    const result = spawnSync("reg", ["query", registryPath, "/v", "pv"], {
      encoding: "utf8",
      windowsHide: true,
    });
    const match = result.stdout?.match(/pv\s+REG_SZ\s+([\d.]+)/);
    if (match) return match[1];
  }
  return null;
}

async function tauriDriverInstalled(path) {
  const result = spawnSync("cargo", ["install", "--list", "--root", root], {
    encoding: "utf8",
    windowsHide: true,
  });
  return result.status === 0 &&
    /tauri-driver v2\.0\.6:/.test(result.stdout ?? "")
    ? Boolean((await stat(path).catch(() => null))?.isFile())
    : false;
}

function decodeMicrosoftVersion(buffer) {
  const decoded = buffer.includes(0)
    ? buffer.toString("utf16le")
    : buffer.toString("utf8");
  return decoded.replaceAll("\0", "").trim();
}

async function expectedVersion(path, pattern) {
  if (!(await stat(path).catch(() => null))?.isFile()) return false;
  const result = spawnSync(path, ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  });
  return result.status === 0 && pattern.test(result.stdout ?? "");
}

function run(command, args) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, {
      cwd: process.cwd(),
      env: process.env,
      shell: false,
      stdio: "inherit",
    });
    child.once("error", rejectRun);
    child.once("exit", (code) => {
      if (code === 0) resolveRun();
      else rejectRun(new Error(`${command} exited with code ${code ?? 1}.`));
    });
  });
}
