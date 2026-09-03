import { expect, test, type Page } from "@playwright/test";
import { auditInterfaceDom } from "./dom-audit";

const taskScenarios = [
  ["broadcast-outcome-task", "broadcast-outcome-laboratory"],
  ["plan-editor-task", "plan-editor"],
  ["production-pathway", "materials-pathway-study"],
  ["materials-overlay-task", "materials-overlay-editor"],
  ["markets-basket-task", "markets-basket-laboratory"],
  ["markets-scenario-task", "markets-scenario-laboratory"],
  ["archive-comparison-task", "archive-comparison"],
] as const;

for (const [scenario, route] of taskScenarios) {
  test(`${route} uses a contained task surface and returns one layer`, async ({
    page,
  }) => {
    await openReview(page, scenario);
    const layer = page.locator(`[data-workspace-task="${route}"]`);
    await expect(layer.getByRole("dialog")).toBeVisible();
    await expect(layer.locator(".task-drawer-body")).toHaveCSS(
      "overflow-y",
      "auto",
    );
    expect(await page.evaluate(auditInterfaceDom)).toEqual([]);

    await page.keyboard.press("Escape");
    await expect(layer).toHaveCount(0);
    await expect(page.locator(".workspace .canvas")).toBeVisible();
  });
}

test("section navigation identifies the current contained section", async ({
  page,
}) => {
  await openReview(page, "workspace-markets");
  const sections = page.locator(".workspace .section-list a");
  await expect(sections.first()).toHaveAttribute("aria-current", "location");
  await sections.last().click();
  await expect(sections.last()).toHaveAttribute("aria-current", "location");
  await expect(page.locator(".command-bar")).toBeVisible();
  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);
});

test("closing a task returns focus to the control that opened it", async ({
  page,
}) => {
  await openReview(page, "workspace-broadcast");
  const opener = page.getByRole("button", { name: "Compare these histories" });
  await opener.click();
  await expect(
    page
      .locator('[data-workspace-task="broadcast-outcome-laboratory"]')
      .getByRole("dialog"),
  ).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
});

test("all workspace tasks remain contained with enlarged text on a narrow screen", async ({
  page,
}) => {
  await page.setViewportSize({ width: 720, height: 820 });
  for (const [scenario, route] of taskScenarios) {
    await openReview(page, scenario);
    await page.evaluate(async () => {
      await (
        window as typeof window & {
          __REPUBLIC_OBSERVATORY_UI_REVIEW__?: {
            setTextScale(value: number): Promise<void>;
          };
        }
      ).__REPUBLIC_OBSERVATORY_UI_REVIEW__?.setTextScale(200);
    });
    await expect(
      page.locator(`[data-workspace-task="${route}"]`).getByRole("dialog"),
    ).toBeVisible();
    const failures = await page.evaluate(auditInterfaceDom);
    expect(
      failures.filter(
        (failure) =>
          failure.kind !== "landmark-horizontal-escape" ||
          failure.selector.includes("task-drawer"),
      ),
    ).toEqual([]);
  }
});

async function openReview(page: Page, scenario: string): Promise<void> {
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
