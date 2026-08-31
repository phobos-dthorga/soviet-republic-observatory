import { readFileSync, readdirSync, statSync } from "node:fs";
import { relative, resolve, sep } from "node:path";

const root = resolve("src");
const files = walk(root).filter((file) => /\.(?:ts|svelte|css)$/.test(file));
const violations = [];

for (const file of files) {
  const path = normalise(relative(process.cwd(), file));
  const source = readFileSync(file, "utf8");

  if (
    /from\s+["']@tauri-apps\//.test(source) &&
    !/^src\/lib\/[^/]+\/desktopClient\.ts$/.test(path)
  ) {
    fail(path, "Tauri access belongs in a feature desktopClient.ts boundary.");
  }

  if (
    /from\s+["']echarts(?:\/|["'])/.test(source) &&
    ![
      "src/lib/charts/ObservatoryChart.svelte",
      "src/lib/charts/chartOptions.ts",
      "src/lib/charts/sankey.ts",
    ].includes(path)
  ) {
    fail(
      path,
      "ECharts access belongs in the application-owned chart adapter.",
    );
  }

  if (
    /\b(?:localStorage|sessionStorage|indexedDB)\b/.test(source) &&
    path !== "src/lib/i18n/service.ts"
  ) {
    fail(
      path,
      "Browser storage is restricted to the documented legacy language handover service.",
    );
  }

  if (
    /(?:style\.setProperty|document\.documentElement\.style)/.test(source) &&
    path !== "src/lib/theme/runtime.ts"
  ) {
    fail(
      path,
      "Theme-variable mutation belongs only in the validated theme runtime.",
    );
  }

  if (path.endsWith(".svelte")) {
    const imports = [...source.matchAll(/from\s+["']([^"']+)["']/g)].map(
      (match) => match[1],
    );
    for (const imported of imports) {
      if (
        path.startsWith("src/lib/workspaces/") &&
        /(?:^|\/)(?:sample|broadcastPreview|materialFlowPreview)$/.test(
          imported,
        )
      ) {
        fail(
          path,
          `Ordinary workspaces cannot import synthetic republic fallbacks: ${imported}`,
        );
      }
      if (
        /(?:^|\/)(?:storage|parsers?|domain)(?:\/|$)/.test(imported) ||
        /\/(?:analysisPack|planningOverlay|profileSchema)$/.test(imported)
      ) {
        fail(
          path,
          `Presentation components cannot import domain policy directly: ${imported}`,
        );
      }
    }
  }

  if (
    [
      "src/lib/presentation/sample.ts",
      "src/lib/presentation/broadcastPreview.ts",
      "src/lib/presentation/materialFlowPreview.ts",
    ].includes(path)
  ) {
    fail(
      path,
      "Synthetic republic previews belong in the typed UI-review fixture boundary, not production presentation modules.",
    );
  }

  if (path.startsWith("src/lib/presentation/")) {
    const imports = [...source.matchAll(/from\s+["']([^"']+)["']/g)].map(
      (match) => match[1],
    );
    for (const imported of imports) {
      if (/desktopClient|\/service$/.test(imported)) {
        fail(
          path,
          `A presentation adapter cannot invoke services or native clients: ${imported}`,
        );
      }
    }
  }
}

if (violations.length) {
  console.error("Architecture boundary audit failed:");
  for (const violation of violations) console.error(`- ${violation}`);
  process.exit(1);
}

console.log(
  `Architecture boundary audit passed: ${files.length} source files; native, chart, storage, theme, and presentation seams are intact.`,
);

function walk(directory) {
  return readdirSync(directory).flatMap((name) => {
    const entry = resolve(directory, name);
    return statSync(entry).isDirectory() ? walk(entry) : [entry];
  });
}

function normalise(path) {
  return sep === "\\" ? path.replaceAll("\\", "/") : path;
}

function fail(path, message) {
  violations.push(`${path}: ${message}`);
}
