import { expect, test, type Page } from "@playwright/test";

const reviewScenarios = [
  "workspace-briefing",
  "workspace-monitor",
  "workspace-broadcast",
  "workspace-extensions",
  "workspace-plan",
  "workspace-materials",
  "materials-resource-catalogue",
  "workspace-population",
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
    const failures = discouragedPhrases.filter((phrase) =>
      text.includes(phrase),
    );
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
