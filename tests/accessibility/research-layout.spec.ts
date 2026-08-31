import { expect, test, type Page } from "@playwright/test";

async function openResearchSetup(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: "Legal & notices" }).click();
  await page.getByRole("tab", { name: "Read-only research" }).click();
  await page.getByRole("button", { name: "Open research setup" }).click();
  await expect(
    page.getByRole("heading", { name: "Experimental Research Setup" }),
  ).toBeVisible();
}

for (const viewport of [
  { width: 1440, height: 1000 },
  { width: 720, height: 820 },
]) {
  test(`research setup remains aligned at ${viewport.width}px`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await openResearchSetup(page);
    const dialog = page.getByRole("dialog", {
      name: "Experimental Research Setup",
    });
    const dialogBox = await dialog.boundingBox();
    const footerBox = await dialog.locator("footer").boundingBox();
    expect(dialogBox).not.toBeNull();
    expect(footerBox).not.toBeNull();
    expect((dialogBox?.x ?? -1) >= 0).toBe(true);
    expect((dialogBox?.y ?? -1) >= 0).toBe(true);
    expect((dialogBox?.x ?? 0) + (dialogBox?.width ?? 0)).toBeLessThanOrEqual(
      viewport.width,
    );
    expect((dialogBox?.y ?? 0) + (dialogBox?.height ?? 0)).toBeLessThanOrEqual(
      viewport.height,
    );
    expect((footerBox?.y ?? 0) + (footerBox?.height ?? 0)).toBeLessThanOrEqual(
      (dialogBox?.y ?? 0) + (dialogBox?.height ?? 0),
    );
  });
}
