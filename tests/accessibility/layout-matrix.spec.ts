import { expect, test, type Page, type TestInfo } from "@playwright/test";
import { auditInterfaceDom } from "./dom-audit";

const workspaces = [
  "Briefing",
  "Monitor",
  "Broadcast",
  "Extensions",
  "Materials",
  "Population",
  "Archive",
];

const layoutCases = [
  { label: "narrow", width: 720, height: 820, textScale: 1 },
  { label: "laptop", width: 1280, height: 720, textScale: 1 },
  { label: "fhd-125", width: 1536, height: 864, textScale: 1.25 },
  { label: "qhd-150", width: 1707, height: 960, textScale: 1.5 },
  { label: "ultrawide-150", width: 2293, height: 959, textScale: 1.5 },
  { label: "uhd-200", width: 1920, height: 1080, textScale: 2 },
  { label: "native-ultrawide", width: 3440, height: 1439, textScale: 1 },
];

for (const layout of layoutCases) {
  test(`enabled workspaces retain their geometry at ${layout.label}`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(60_000);
    await page.setViewportSize({ width: layout.width, height: layout.height });
    await page.goto("/");
    await page.evaluate((scale) => {
      document.documentElement.style.fontSize = `${16 * scale}px`;
    }, layout.textScale);

    for (const workspace of workspaces) {
      await page
        .getByRole("navigation")
        .getByRole("button", { name: workspace })
        .click();
      await page.waitForTimeout(40);
      await assertLayout(page, testInfo, `${layout.label} / ${workspace}`);
    }
  });
}

async function assertLayout(
  page: Page,
  testInfo: TestInfo,
  label: string,
): Promise<void> {
  const failures = await page.evaluate(auditInterfaceDom);

  await testInfo.attach("layout-report", {
    body: JSON.stringify({ label, failures }, null, 2),
    contentType: "application/json",
  });
  expect(failures, `${label}: interface geometry failures`).toEqual([]);
}
