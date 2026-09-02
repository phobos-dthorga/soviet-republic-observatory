import { expect, test, type Page } from "@playwright/test";
import { auditInterfaceDom } from "./dom-audit";

test("Environment keeps save facts, unavailable readings, and player assumptions distinct", async ({
  page,
}) => {
  await openEnvironmentReview(page);

  await expect(
    page.getByRole("heading", { name: "Environment and resource use" }),
  ).toBeVisible();
  await expect(
    page.getByText("Buildings with environmental facts"),
  ).toBeVisible();
  await expect(page.getByText("18", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /chemicals.*recorded history/i }),
  ).toBeVisible();
  await expect(
    page.getByText("Two values are preserved, not combined"),
  ).toBeVisible();
  await expect(page.getByRole("cell", { name: "92.278" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "-2,485.883" })).toBeVisible();
  await expect(
    page.getByText("No checked facility reading is available yet"),
  ).toHaveCount(2);
  await expect(
    page.getByText("An estimate, not a complete footprint"),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Where the covered estimate comes from",
    }),
  ).toBeVisible();
  await expect(page.getByText("CO₂e intensity is unavailable")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Save new factor set" }),
  ).toBeDisabled();

  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
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

async function openEnvironmentReview(page: Page): Promise<void> {
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
  await page.evaluate(async () => {
    const controller = (
      window as typeof window & {
        __REPUBLIC_OBSERVATORY_UI_REVIEW__?: {
          selectScenario(value: string): Promise<void>;
        };
      }
    ).__REPUBLIC_OBSERVATORY_UI_REVIEW__;
    if (!controller) throw new Error("review controller unavailable");
    await controller.selectScenario("workspace-environment");
  });
}
