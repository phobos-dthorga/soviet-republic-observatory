import { expect, test } from "@playwright/test";

const workspacesWithSectionNavigation = [
  "Briefing",
  "Monitor",
  "Broadcast",
  "Extensions",
  "Plan",
  "Materials",
  "Population",
  "Environment",
  "Markets",
];

for (const workspace of workspacesWithSectionNavigation) {
  test(`${workspace} section links preserve the global navigation bars`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await page
      .getByRole("navigation")
      .getByRole("button", { name: workspace })
      .click();

    const sectionLinks = page.locator(".workspace .section-list a");
    await expect(sectionLinks.first()).toBeVisible();
    const linkCount = await sectionLinks.count();
    for (let index = 0; index < linkCount; index += 1) {
      const link = sectionLinks.nth(index);
      const targetHref = await link.getAttribute("href");
      const targetExists = await page.evaluate(
        (href) => Boolean(href && document.getElementById(href.slice(1))),
        targetHref,
      );
      await link.click();

      const geometry = await page.evaluate(() => {
        const commandBar = document.querySelector<HTMLElement>(".command-bar");
        const saveBar = document.querySelector<HTMLElement>(".observation-bar");
        const canvas = document.querySelector<HTMLElement>(
          ".workspace > .canvas",
        );
        return {
          rootScroll: document.documentElement.scrollTop,
          bodyScroll: document.body.scrollTop,
          commandTop: commandBar?.getBoundingClientRect().top,
          saveTop: saveBar?.getBoundingClientRect().top,
          canvasScroll: canvas?.scrollTop,
          activeTarget: document.activeElement?.id,
        };
      });

      expect(geometry.rootScroll).toBe(0);
      expect(geometry.bodyScroll).toBe(0);
      expect(geometry.commandTop).toBe(0);
      expect(geometry.saveTop).toBeGreaterThanOrEqual(69);
      expect(geometry.canvasScroll).toBeGreaterThanOrEqual(0);
      if (targetExists) {
        expect(geometry.activeTarget).toBe(targetHref?.slice(1));
      }
      await expect(page.locator(".command-bar")).toBeVisible();
      await expect(page.locator(".observation-bar")).toBeVisible();
    }
  });
}

test("the selected section remains stable while a final section smooth-scrolls", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.setViewportSize({ width: 1280, height: 720 });
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
    await (
      window as typeof window & {
        __REPUBLIC_OBSERVATORY_UI_REVIEW__?: {
          selectScenario(value: string): Promise<void>;
        };
      }
    ).__REPUBLIC_OBSERVATORY_UI_REVIEW__?.selectScenario(
      "materials-resource-catalogue",
    );
  });

  const link = page.locator(
    '.workspace .section-list a[href="#overlay-laboratory"]',
  );
  await expect(link).toBeVisible();
  await link.click();
  await expect(link).toHaveAttribute("aria-current", "location");
  await page.waitForTimeout(1_300);
  await expect(link).toHaveAttribute("aria-current", "location");

  const geometry = await page.evaluate(() => ({
    rootScroll: document.documentElement.scrollTop,
    bodyScroll: document.body.scrollTop,
    commandTop: document
      .querySelector<HTMLElement>(".command-bar")
      ?.getBoundingClientRect().top,
  }));
  expect(geometry.rootScroll).toBe(0);
  expect(geometry.bodyScroll).toBe(0);
  expect(geometry.commandTop).toBe(0);
});
