import { readFileSync, readdirSync, statSync } from "node:fs";
import { relative, resolve } from "node:path";

const rustFiles = walk(resolve("src-tauri/src")).filter((path) =>
  path.endsWith(".rs"),
);
const keyword =
  /^(?:and|cross|delete|from|group|having|inner|insert|join|left|limit|on|or|order|outer|returning|right|select|set|union|update|values|where)\b/i;
const violations = [];

for (const file of rustFiles) {
  const lines = readFileSync(file, "utf8").split(/\r?\n/);
  for (let index = 0; index < lines.length - 1; index += 1) {
    if (!/[a-zA-Z0-9_]\\\s*$/.test(lines[index])) continue;
    const next = lines[index + 1].trimStart();
    if (!keyword.test(next)) continue;
    violations.push(
      `${relative(process.cwd(), file).replaceAll("\\", "/")}:${index + 1}`,
    );
  }
}

if (violations.length) {
  console.error("SQL literal spacing audit failed:");
  for (const violation of violations) {
    console.error(
      `- ${violation}: a continued SQL identifier meets the next keyword without whitespace.`,
    );
  }
  process.exit(1);
}

console.log(
  `SQL literal spacing audit passed: ${rustFiles.length} Rust source files checked.`,
);

function walk(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = resolve(directory, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}
