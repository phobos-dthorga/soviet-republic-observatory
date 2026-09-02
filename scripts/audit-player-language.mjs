import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = readJson("locales/en-AU.json");
const technical = readJson("locales/en-AU-technical.json");
const policy = readJson("locales/player-language-policy.json");
const failures = [];
const technicalKeys = new Set(policy.technical_detail_keys);
const exceptions = new Map();
const today = new Date().toISOString().slice(0, 10);

for (const exception of policy.exceptions) {
  const identity = `${exception.message_key}:${exception.rule}`;
  if (exceptions.has(identity)) {
    failures.push(`duplicate exception ${identity}`);
    continue;
  }
  if (!source.messages[exception.message_key]) {
    failures.push(
      `exception references unknown message ${exception.message_key}`,
    );
  }
  if (
    !exception.reason?.trim() ||
    !/^\d{4}-\d{2}-\d{2}$/.test(exception.expires)
  ) {
    failures.push(`exception ${identity} needs a reason and YYYY-MM-DD expiry`);
  } else if (exception.expires < today) {
    failures.push(`exception ${identity} expired on ${exception.expires}`);
  }
  exceptions.set(identity, exception);
}

if (
  technical.schema_version !== 1 ||
  technical.source_locale !== source.locale ||
  technical.source_catalog_version !== source.source_catalog_version ||
  technical.source_catalog_revision !== source.source_catalog_revision
) {
  failures.push("technical wording overlay identity is out of date");
}

for (const [key, pattern] of Object.entries(technical.messages)) {
  const sourcePattern = source.messages[key];
  if (!sourcePattern) {
    failures.push(`technical overlay contains unknown message ${key}`);
    continue;
  }
  if (!String(pattern).trim()) {
    failures.push(`technical overlay message ${key} is empty`);
  }
  if (variables(pattern).join(",") !== variables(sourcePattern).join(",")) {
    failures.push(`technical overlay variable mismatch in ${key}`);
  }
}

for (const [key, pattern] of Object.entries(source.messages)) {
  if (technicalKeys.has(key)) continue;
  const visible = visibleText(pattern);
  const lower = visible.toLocaleLowerCase("en-AU");

  for (const phrase of Object.keys(policy.discouraged_phrases)) {
    if (lower.includes(phrase.toLocaleLowerCase("en-AU"))) {
      failUnlessExcepted(key, `phrase:${phrase}`, `contains “${phrase}”`);
    }
  }
  for (const term of policy.implementation_terms) {
    if (new RegExp(`\\b${escapeRegex(term)}\\b`, "i").test(visible)) {
      failUnlessExcepted(
        key,
        `implementation:${term}`,
        `exposes implementation term “${term}”`,
      );
    }
  }
  for (const term of policy.formal_terms_reserved_for_details) {
    if (new RegExp(`\\b${escapeRegex(term)}\\b`, "i").test(visible)) {
      failUnlessExcepted(
        key,
        `formal:${term}`,
        `uses unexplained formal term “${term}”`,
      );
    }
  }
  if (/\b[a-z]+(?:_[a-z0-9]+)+\b/.test(visible)) {
    failUnlessExcepted(key, "snake_case", "shows a raw snake_case value");
  }

  for (const sentence of sentences(visible)) {
    const count = words(sentence).length;
    if (count > policy.maximum_primary_sentence_words) {
      failUnlessExcepted(
        key,
        "sentence_length",
        `has a ${count}-word sentence (maximum ${policy.maximum_primary_sentence_words})`,
      );
    }
  }
}

const workspacePrefixes = {
  Briefing: ["briefing"],
  Monitor: ["monitor", "scanner", "recorder"],
  Broadcast: [
    "broadcast",
    "programme",
    "station",
    "receiver",
    "notebook",
    "outcome",
    "causality",
  ],
  Extensions: ["extensions", "extension", "security"],
  Plan: ["plan"],
  Materials: ["catalogue", "production"],
  Population: ["population"],
  Environment: ["environment"],
  Markets: ["markets"],
  Archive: ["archive"],
};

const playerPatterns = Object.entries(source.messages).filter(
  ([key]) => !technicalKeys.has(key),
);
const overallGrade = fleschKincaid(playerPatterns.map(([, value]) => value));
if (overallGrade > policy.target_flesch_kincaid_grade) {
  failures.push(
    `player-facing English is grade ${overallGrade.toFixed(2)}; target is ${policy.target_flesch_kincaid_grade}`,
  );
}
for (const [workspace, prefixes] of Object.entries(workspacePrefixes)) {
  const corpus = playerPatterns
    .filter(([key]) => prefixes.some((prefix) => key.startsWith(`${prefix}-`)))
    .map(([, value]) => value);
  const grade = fleschKincaid(corpus);
  if (grade > policy.target_flesch_kincaid_grade) {
    failures.push(
      `${workspace} wording is grade ${grade.toFixed(2)}; target is ${policy.target_flesch_kincaid_grade}`,
    );
  }
}

if (failures.length > 0) {
  console.error(
    `Player-language audit failed:\n${failures.map((failure) => `- ${failure}`).join("\n")}`,
  );
  process.exit(1);
}

console.log(
  `Player-language audit passed: ${playerPatterns.length} ordinary messages at grade ${overallGrade.toFixed(2)}, ${Object.keys(technical.messages).length} technical overrides, and ${Object.keys(workspacePrefixes).length} workspace corpora checked.`,
);

function failUnlessExcepted(key, rule, detail) {
  if (
    exceptions.has(`${key}:${rule}`) ||
    exceptions.has(`${key}:${rule.split(":")[0]}`)
  ) {
    return;
  }
  failures.push(`${key}: ${detail}`);
}

function visibleText(pattern) {
  return String(pattern)
    .replace(/\{\s*\$[A-Za-z0-9_-]+\s*->/g, " ")
    .replace(/\{\s*\$[A-Za-z0-9_-]+\s*\}/g, " ")
    .replace(/^\s*\[[^\]]+\]\s*/gm, " ")
    .replace(/[{}*]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function variables(pattern) {
  return [...String(pattern).matchAll(/\{\s*\$([A-Za-z0-9_-]+)/g)]
    .map((match) => match[1])
    .sort();
}

function sentences(text) {
  return text
    .split(/(?<=[.!?])\s+/)
    .map((sentence) => sentence.trim())
    .filter(Boolean);
}

function words(text) {
  return text.match(/[A-Za-z]+(?:[-’'][A-Za-z]+)*/g) ?? [];
}

function fleschKincaid(patterns) {
  let wordCount = 0;
  let sentenceCount = 0;
  let syllableCount = 0;
  for (const pattern of patterns) {
    const text = visibleText(pattern);
    const messageWords = words(text);
    if (messageWords.length < 8) continue;
    wordCount += messageWords.length;
    sentenceCount += Math.max(1, (text.match(/[.!?]+/g) ?? []).length);
    syllableCount += messageWords.reduce(
      (sum, word) => sum + syllables(word),
      0,
    );
  }
  if (wordCount === 0) return 0;
  return (
    0.39 * (wordCount / sentenceCount) +
    11.8 * (syllableCount / wordCount) -
    15.59
  );
}

function syllables(word) {
  const normalised = word.toLocaleLowerCase("en-AU").replace(/[^a-z]/g, "");
  if (normalised.length <= 3) return 1;
  const groups = normalised
    .replace(/(?:[^laeiouy]es|ed|[^laeiouy]e)$/, "")
    .replace(/^y/, "")
    .match(/[aeiouy]+/g);
  return Math.max(1, groups?.length ?? 1);
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function readJson(path) {
  return JSON.parse(readFileSync(join(root, path), "utf8"));
}
