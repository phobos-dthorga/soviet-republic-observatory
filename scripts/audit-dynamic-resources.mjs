import { readFileSync, readdirSync, statSync } from "node:fs";
import { relative, resolve } from "node:path";

const violations = [];
const sourceFiles = walk(resolve("src/lib")).filter((path) =>
  /\.(?:ts|svelte)$/.test(path),
);

for (const file of sourceFiles) {
  const path = relative(process.cwd(), file).replaceAll("\\", "/");
  const source = readFileSync(file, "utf8");
  if (
    /switch\s*\(\s*(?:resourceId|resourceToken|resource_id)\s*\)/.test(source)
  ) {
    fail(
      path,
      "Resource labels and inventories must come from the Rust catalogue.",
    );
  }
  if (
    /(?:resourceIds|resourceTokens|resources)\s*=\s*\[[\s\S]{0,600}?resource::/.test(
      source,
    ) &&
    !path.includes("ui-review/fixtures") &&
    !path.endsWith(".test.ts") &&
    path !== "src/lib/navigation/relatedData.ts"
  ) {
    fail(path, "A player-facing resource inventory is hard-coded.");
  }
}

const nativeModels = readFileSync("src-tauri/src/model.rs", "utf8");
const nativeCatalogue = readFileSync(
  "src-tauri/src/resource_catalogue.rs",
  "utf8",
);
for (const required of [
  "ResourceCatalogueRevision",
  "ResourceCatalogueEntry",
  "ResourceOriginEvidence",
  "ResourceLivePrice",
  "ResourceRegistryAssurance",
]) {
  if (
    !nativeModels.includes(`struct ${required}`) &&
    !nativeModels.includes(`enum ${required}`)
  ) {
    fail(
      "src-tauri/src/model.rs",
      `Missing dynamic resource contract ${required}.`,
    );
  }
}
if (!nativeCatalogue.includes("unobtainium_crystal")) {
  fail(
    "src-tauri/src/resource_catalogue.rs",
    "The catalogue needs a synthetic unknown-resource regression test.",
  );
}

if (violations.length) {
  console.error("Dynamic resource audit failed:");
  for (const violation of violations) console.error(`- ${violation}`);
  process.exit(1);
}

console.log(
  `Dynamic resource audit passed: ${sourceFiles.length} interface files use the Rust-owned resource catalogue boundary.`,
);

function walk(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = resolve(directory, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

function fail(path, message) {
  violations.push(`${path}: ${message}`);
}
