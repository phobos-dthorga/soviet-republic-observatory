import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { auditInterfaceDom } from "./dom-audit";

const reviewScenarios = [
  "workspace-briefing",
  "workspace-monitor",
  "workspace-broadcast",
  "workspace-extensions",
  "workspace-plan",
  "workspace-materials",
  "materials-resource-catalogue",
  "workspace-population",
  "workspace-environment",
  "environment-indexing",
  "environment-details",
  "workspace-markets",
  "archive-latest",
  "dialog-language",
  "dialog-theme",
  "dialog-settings",
  "dialog-observation",
  "dialog-diagnostics",
  "dialog-legal",
  "dialog-research",
  "dialog-recovery",
] as const;

const discouragedPhrases = [
  "analytical head",
  "immutable revision",
  "deterministic calculation",
  "evidence assay",
  "source-backed",
  "host-owned",
  "analytical warehouse",
  "projection",
  "interpretation",
  "denominator",
  "guardrail",
  "trajectory",
  "coefficient",
  "provenance",
  "ledger",
  "bounded",
  "parsed save fact",
  "game-definition fact",
  "player definition",
  "player override",
];
const environmentSpecialistPhrases = [
  "save-backed",
  "checked-session",
  "facility contract",
  "accounting boundary",
  "source rows",
  "factor set",
  "factor-set",
  "game-file catalogue",
  "analysis database",
];

test("player-friendly review scenarios keep specialist wording out of ordinary surfaces", async ({
  page,
}) => {
  test.setTimeout(90_000);
  await openReview(page);
  await setWordingMode(page, "player_friendly");

  for (const scenario of reviewScenarios) {
    await selectScenario(page, scenario);
    const text = (await page.locator("body").innerText()).toLocaleLowerCase(
      "en-AU",
    );
    const phrases =
      scenario === "workspace-environment" ||
      scenario.startsWith("environment-")
        ? [...discouragedPhrases, ...environmentSpecialistPhrases]
        : discouragedPhrases;
    const failures = phrases.filter((phrase) => text.includes(phrase));
    expect(failures, `${scenario}: specialist wording`).toEqual([]);
    expect(
      text.match(
        /\b(?:storage|invalid|unknown|research|market)_[a-z0-9_]+\b/g,
      ) ?? [],
      `${scenario}: raw diagnostic code`,
    ).toEqual([]);
  }
});

test("native review can exercise both English wording modes without changing the interface state", async ({
  page,
}) => {
  await openReview(page);
  await selectScenario(page, "dialog-settings");

  await setWordingMode(page, "player_friendly");
  await expect(
    page.getByText(
      "Choose folders, appearance, and how patient background tasks should be.",
    ),
  ).toBeVisible();

  await setWordingMode(page, "technical");
  await expect(
    page.getByText(
      "Configure Observatory's private local sources, accessible presentation, and bounded background-work behaviour.",
    ),
  ).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute(
    "data-wording-mode",
    "technical",
  );
});

test("Environment keeps technical research language behind Technical wording", async ({
  page,
}) => {
  await openReview(page);
  await selectScenario(page, "workspace-environment");

  await setWordingMode(page, "player_friendly");
  await expect(page.getByText("From your recorded saves")).toBeVisible();
  await expect(page.getByText("From your save", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Save-backed environmental evidence"),
  ).toHaveCount(0);

  await setWordingMode(page, "technical");
  await expect(
    page.getByText("Save-backed environmental evidence"),
  ).toBeVisible();
  await expect(
    page.getByText("No validated facility snapshot is available"),
  ).toHaveCount(2);
  await expect(
    page.getByText("Parsed save fact", { exact: true }),
  ).toBeVisible();
});

test("resource catalogue review uses dynamic origins and retained live evidence", async ({
  page,
}) => {
  await openReview(page);
  await selectScenario(page, "materials-resource-catalogue");

  await expect(
    page.getByRole("heading", { name: "Resource Catalogue" }),
  ).toBeVisible();
  await expect(page.getByText("Last verified in a game session")).toBeVisible();
  await expect(page.getByText("ecomponents")).toBeVisible();
  await expect(page.getByText("player_polymer")).toBeVisible();

  const axe = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
    .analyze();
  expect(axe.violations).toEqual([]);
  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
});

test("contextual help remains fully inside a narrow viewport", async ({
  page,
}) => {
  await page.setViewportSize({ width: 720, height: 820 });
  await openReview(page);
  await selectScenario(page, "tooltip-contextual");

  const trigger = page.locator(
    "[data-help-topic='metric-context-source-stats-citizens-adults'] button",
  );
  await trigger.scrollIntoViewIfNeeded();
  await trigger.click();
  await expect(page.getByRole("tooltip")).toBeVisible();
  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
});

async function openReview(page: Page): Promise<void> {
  await page.goto("/?ui-review=fixture");
  await page.waitForFunction(() =>
    Boolean(
      (
        window as typeof window & {
          __REPUBLIC_OBSERVATORY_UI_REVIEW__?: unknown;
        }
      ).__REPUBLIC_OBSERVATORY_UI_REVIEW__,
    ),
  );
}

async function selectScenario(
  page: Page,
  scenario: (typeof reviewScenarios)[number],
): Promise<void> {
  await page.evaluate(async (selected) => {
    const controller = (
      window as typeof window & {
        __REPUBLIC_OBSERVATORY_UI_REVIEW__?: {
          selectScenario(value: string): Promise<void>;
        };
      }
    ).__REPUBLIC_OBSERVATORY_UI_REVIEW__;
    if (!controller) throw new Error("review controller unavailable");
    await controller.selectScenario(selected);
  }, scenario);
}

async function setWordingMode(
  page: Page,
  mode: "player_friendly" | "technical",
): Promise<void> {
  await page.evaluate(async (selected) => {
    const controller = (
      window as typeof window & {
        __REPUBLIC_OBSERVATORY_UI_REVIEW__?: {
          setWordingMode(value: string): Promise<void>;
        };
      }
    ).__REPUBLIC_OBSERVATORY_UI_REVIEW__;
    if (!controller) throw new Error("review controller unavailable");
    await controller.setWordingMode(selected);
  }, mode);
}
