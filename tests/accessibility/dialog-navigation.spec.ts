import { expect, test } from "@playwright/test";

test("child dialogs return to the immediately previous dialog", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await expect(settings).toBeVisible();

  await settings.getByRole("button", { name: /Interface language/i }).click();
  await expect(
    page.getByRole("dialog", { name: "Observatory languages" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(settings).toBeVisible();

  await settings.getByRole("button", { name: /Validated theme/i }).click();
  await expect(page.getByRole("dialog", { name: "Themes" })).toBeVisible();
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(settings).toBeVisible();
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(settings).toBeHidden();
});

test("the Settings shortcut never creates duplicate dialog layers", async ({
  page,
}) => {
  await page.goto("/");
  await page.keyboard.press("Control+Comma");
  await page.keyboard.press("Control+Comma");
  await expect(page.getByRole("dialog", { name: "Settings" })).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeHidden();
});

test("Save Observer and Research return to the dialog that opened them", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await settings.getByRole("button", { name: "Open Save Observer" }).click();
  await expect(
    page.getByRole("dialog", { name: "Save observer" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(settings).toBeVisible();
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Legal & notices" }).click();
  const legal = page.getByRole("dialog", {
    name: "Legal and third-party notices",
  });
  await legal.getByRole("tab", { name: "Read-only research" }).click();
  await legal.getByRole("button", { name: "Open research setup" }).click();
  await expect(
    page.getByRole("dialog", { name: "Experimental Research Setup" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(legal).toBeVisible();
});

test("unsaved settings require an explicit discard decision", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  const textScale = settings.getByLabel("Interface text scale");
  await textScale.selectOption("125");
  await textScale.focus();

  // The first Escape belongs to the native select control.
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  const recovery = page.getByRole("dialog", {
    name: "Discard unsaved settings?",
  });
  await expect(recovery).toBeVisible();
  await recovery.getByRole("button", { name: "Not now" }).click();
  await expect(settings).toBeVisible();

  await page.keyboard.press("Escape");
  await recovery.getByRole("button", { name: "Discard changes" }).click();
  await expect(settings).toBeHidden();
});
