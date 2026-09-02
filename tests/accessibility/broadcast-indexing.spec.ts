import { expect, test, type Page } from "@playwright/test";

test("cached Broadcast saves are shown as checked and already current", async ({
  page,
}) => {
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
  await selectScenario(page, "broadcast-current");

  const progress = page.locator(".index-progress");
  await expect(progress.getByText(/Checked.*25.*of.*25.*saves/)).toBeVisible();
  await expect(
    progress.getByText(
      /Added.*0.*already current.*25.*missing.*0.*changed.*0.*failed.*0/,
    ),
  ).toBeVisible();
  await expect(
    progress.getByText("All available saves were checked."),
  ).toBeVisible();
});

async function selectScenario(page: Page, scenario: string): Promise<void> {
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
