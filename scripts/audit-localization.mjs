import { readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = readJson("locales/en-AU.json");
const schema = readJson("schemas/language-pack-v1.schema.json");
const example = readJson(
  "examples/language-packs/community-fr-example.rolanguage.json",
);
const validationSource = readFileSync(
  join(root, "src/lib/i18n/validation.ts"),
  "utf8",
);
const keys = new Set(Object.keys(source.messages));
const failures = [];

for (const [label, value] of [
  ["schema version", schema.properties.schema_version.const],
  ["example schema version", example.schema_version],
]) {
  if (value !== source.schema_version)
    failures.push(`${label} does not match the source catalogue`);
}
for (const [label, value] of [
  ["schema source version", schema.properties.source_catalog_version.const],
  ["example source version", example.source_catalog_version],
]) {
  if (value !== source.source_catalog_version)
    failures.push(`${label} does not match the source catalogue`);
}
if (
  schema.properties.source_catalog_revision.maximum !==
  source.source_catalog_revision
) {
  failures.push("schema source revision does not match the source catalogue");
}
if (example.source_catalog_revision > source.source_catalog_revision) {
  failures.push("example targets a future source revision");
}
for (const [constant, expected] of [
  ["LANGUAGE_PACK_SCHEMA_VERSION", source.schema_version],
  ["SOURCE_CATALOG_VERSION", source.source_catalog_version],
  ["SOURCE_CATALOG_REVISION", source.source_catalog_revision],
]) {
  const actual = Number(
    validationSource.match(
      new RegExp(`export const ${constant} = (\\d+)`),
    )?.[1],
  );
  if (actual !== expected)
    failures.push(`${constant} does not match the source catalogue`);
}

for (const file of walk(join(root, "src"))) {
  if (
    ![".ts", ".svelte"].includes(extname(file)) ||
    /(?:\.test\.ts|catalog\.ts)$/.test(file)
  )
    continue;
  const contents = readFileSync(file, "utf8");
  const display = relative(root, file).replaceAll("\\", "/");
  const literalPattern =
    /(?:\$translation|\btranslate|\bt)\(\s*["']([a-z][a-z0-9-]*)["']/g;
  for (const match of contents.matchAll(literalPattern)) {
    if (!keys.has(match[1]))
      failures.push(`${display}: unknown translation key ${match[1]}`);
  }
  if (/(?:\$translation|\btranslate|\bt)\(\s*`[^`]*\$\{/s.test(contents)) {
    failures.push(
      `${display}: constructed translation keys are forbidden; use an explicit typed map`,
    );
  }
  if (file.replaceAll("\\", "/").endsWith("/src/lib/i18n/format.ts")) continue;
  if (
    /Intl\.(?:NumberFormat|DateTimeFormat)|\.toLocale(?:String|DateString|TimeString)\(/.test(
      contents,
    )
  ) {
    failures.push(
      `${display}: locale-sensitive formatting must use src/lib/i18n/format.ts`,
    );
  }
}

if (failures.length > 0) {
  console.error(
    `Localisation audit failed:\n${failures.map((failure) => `- ${failure}`).join("\n")}`,
  );
  process.exit(1);
}

console.log(
  `Localisation audit passed: ${keys.size} canonical messages, compatibility v${source.source_catalog_version}, revision ${source.source_catalog_revision}.`,
);

function readJson(path) {
  return JSON.parse(readFileSync(join(root, path), "utf8"));
}

function walk(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}
