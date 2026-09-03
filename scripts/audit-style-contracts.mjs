import { readFileSync, readdirSync, statSync } from "node:fs";
import { relative, resolve } from "node:path";

const sourceFiles = walk(resolve("src")).filter((path) =>
  /\.(?:css|svelte)$/.test(path),
);
const definitions = new Set();
const references = new Map();

for (const file of sourceFiles) {
  const source = readFileSync(file, "utf8");
  const displayPath = relative(process.cwd(), file).replaceAll("\\", "/");
  for (const match of source.matchAll(/(--[a-zA-Z0-9-]+)\s*:/g)) {
    definitions.add(match[1]);
  }
  for (const match of source.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)/g)) {
    const files = references.get(match[1]) ?? new Set();
    files.add(displayPath);
    references.set(match[1], files);
  }
}

const missing = [...references.entries()]
  .filter(([name]) => !definitions.has(name))
  .sort(([left], [right]) => left.localeCompare(right));

if (missing.length) {
  console.error("Interface style audit failed:");
  for (const [name, files] of missing) {
    console.error(
      `- ${name} has no fallback definition (${[...files].join(", ")})`,
    );
  }
  process.exit(1);
}

console.log(
  `Interface style audit passed: ${references.size} custom properties have source fallbacks.`,
);

function walk(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = resolve(directory, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}
