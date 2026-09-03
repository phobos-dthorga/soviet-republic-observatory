import { expect, test, type Page } from "@playwright/test";
import { auditInterfaceDom } from "./dom-audit";

test("Environment keeps save facts, unavailable readings, and player assumptions distinct", async ({
  page,
}) => {
  await openEnvironmentReview(page);

  await expect(
    page
      .locator(".canvas > header")
      .getByRole("heading", { name: "Environment", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Buildings with environment information"),
  ).toBeVisible();
  await expect(page.getByText("18", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /chemicals.*over time/i }),
  ).toBeVisible();
  await expect(
    page.getByText("The two numbers from W&R stay separate"),
  ).toBeVisible();
  await expect(page.getByRole("cell", { name: "92.278" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "-2,485.883" })).toBeVisible();
  await expect(
    page.getByText("No live building reading is available"),
  ).toHaveCount(2);
  await expect(
    page.getByText("Your result covers only activity with carbon values"),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Where the covered estimate comes from",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("CO₂e per output unit is not available"),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Save carbon estimate setup" }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Turn on automatic live readings" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "See live-reading research status" }),
  ).toBeVisible();
  await expect(
    page.getByText("waiting_for_reviewed_facility_contract"),
  ).toHaveCount(0);

  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
});

test("Environment keeps carbon inputs grouped at an ultrawide viewport", async ({
  page,
}) => {
  await page.setViewportSize({ width: 3440, height: 1439 });
  await openEnvironmentReview(page);

  const groups = page.locator(".factor-group");
  await expect(groups).toHaveCount(2);
  const [study, entry] = await groups.evaluateAll((elements) =>
    elements.map((element) => {
      const box = element.getBoundingClientRect();
      return { top: box.top, left: box.left, right: box.right };
    }),
  );
  expect(Math.abs(study.top - entry.top)).toBeLessThanOrEqual(2);
  expect(study.right).toBeLessThan(entry.left);
  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
});

test("completed Environment indexing reports outcomes instead of zero progress", async ({
  page,
}) => {
  await openEnvironmentReview(page, "environment-indexing");
  await expect(page.locator(".progress-card strong")).toHaveText(
    /Finished checking.*12.*saves/,
  );
  await expect(page.locator(".progress-card > span")).toHaveText(
    /4.*updated.*6.*unchanged.*1.*missing.*1.*changed/,
  );
  await expect(page.getByText("Checked 0 of 12 saves")).toHaveCount(0);
});

test("Environment remains contained at a narrow, enlarged-text viewport", async ({
  page,
}) => {
  await page.setViewportSize({ width: 720, height: 820 });
  await openEnvironmentReview(page);
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "24px";
  });
  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
});

async function openEnvironmentReview(
  page: Page,
  scenario = "workspace-environment",
): Promise<void> {
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
