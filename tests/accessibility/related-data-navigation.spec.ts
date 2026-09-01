import { expect, test, type Page } from "@playwright/test";

test("related values use a chooser, breadcrumb, and session Back trail", async ({
  page,
}) => {
  await page.setViewportSize({ width: 720, height: 820 });
  await openFixture(page, "workspace-briefing");

  const origin = page.locator(".metric-card .related-data-link").first();
  await origin.click();
  const chooser = page.getByRole("dialog", { name: "Choose what to open" });
  await expect(chooser).toBeVisible();
  await chooser.getByRole("button", { name: /Population/ }).click();

  await expect(page.locator(".related-breadcrumb")).toBeVisible();
  await expect(
    page.getByRole("navigation").getByRole("button", { name: "Population" }),
  ).toHaveAttribute("aria-current", "page");
  await expectShellContained(page);

  await page.keyboard.press("Alt+ArrowLeft");
  await expect(
    page.getByRole("navigation").getByRole("button", { name: "Briefing" }),
  ).toHaveAttribute("aria-current", "page");
  await expect(origin).toBeFocused();
});

test("closing a related-view chooser restores its dedicated value link", async ({
  page,
}) => {
  await openFixture(page, "workspace-briefing");
  const origin = page.locator(".metric-card .related-data-link").first();
  await origin.click();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("dialog", { name: "Choose what to open" }),
  ).toBeHidden();
  await expect(origin).toBeFocused();
});

test("clickable chart marks have the same action in the chart data table", async ({
  page,
}) => {
  await openFixture(page, "workspace-broadcast");
  const chart = page.locator("#receivers .chart-card").first();
  await chart.locator(".chart-data-ledger summary").click();
  const action = chart.locator(".chart-data-ledger .table-link").first();
  await expect(action).toBeVisible();
  await action.click();
  const chooser = page.getByRole("dialog", { name: "Choose what to open" });
  await expect(chooser).toBeVisible();
  await chooser.getByRole("button", { name: /Archive/ }).click();
  await expect(page.locator(".related-breadcrumb")).toBeVisible();
});

test("Broadcast electronics links preserve explicit market choices and Back", async ({
  page,
}) => {
  await openFixture(page, "workspace-broadcast");
  const origin = page.getByRole("button", { name: "Electronics", exact: true });
  await origin.click();
  const chooser = page.getByRole("dialog", { name: "Choose what to open" });
  await expect(chooser.getByRole("button")).toHaveCount(8);
  await chooser.getByRole("button", { name: /RUB standard trade/ }).click();

  await expect(
    page.getByRole("navigation").getByRole("button", { name: "Markets" }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page.getByLabel("Filter resource tokens")).toHaveValue(
    "eletronics",
  );

  await page.keyboard.press("Alt+ArrowLeft");
  await expect(origin).toBeFocused();
});

test("electronics market rows link back to receiver uptake", async ({
  page,
}) => {
  await openFixture(page, "workspace-markets");
  const rowLink = page.getByRole("button", { name: "eletronics" }).first();
  await rowLink.click();
  const chooser = page.getByRole("dialog", { name: "Choose what to open" });
  await chooser.getByRole("button", { name: /Broadcast/ }).click();

  await expect(
    page.getByRole("navigation").getByRole("button", { name: "Broadcast" }),
  ).toHaveAttribute("aria-current", "page");
  await expect(page.locator("#receivers")).toBeVisible();
});

async function openFixture(page: Page, scenario: string): Promise<void> {
  await page.goto("/?ui-review=fixture");
  await page.waitForFunction(() =>
    Boolean(window.__REPUBLIC_OBSERVATORY_UI_REVIEW__),
  );
  await page.evaluate(async (requested) => {
    await window.__REPUBLIC_OBSERVATORY_UI_REVIEW__?.selectScenario(
      requested as never,
    );
  }, scenario);
}

async function expectShellContained(page: Page): Promise<void> {
  const geometry = await page.evaluate(() => ({
    rootScroll: Math.max(
      window.scrollY,
      document.documentElement.scrollTop,
      document.body.scrollTop,
    ),
    commandTop: document.querySelector(".command-bar")?.getBoundingClientRect()
      .top,
    statusBottom: document.querySelector(".status-bar")?.getBoundingClientRect()
      .bottom,
    viewportHeight: window.innerHeight,
  }));
  expect(geometry.rootScroll).toBe(0);
  expect(geometry.commandTop).toBe(0);
  expect(geometry.statusBottom).toBeCloseTo(geometry.viewportHeight, 0);
}
