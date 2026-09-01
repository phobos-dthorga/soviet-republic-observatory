import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type TestInfo } from "@playwright/test";
import { auditInterfaceContrast } from "./contrast-audit";

type ThemeDocument = {
  name: string;
  colours: Record<string, string>;
  chart_palette: string[];
};

const classic = readTheme("themes/republic-observatory-classic.rotheme.json");
const highContrast = readTheme(
  "themes/republic-observatory-high-contrast.rotheme.json",
);
const boundary: ThemeDocument = {
  ...classic,
  name: "Generated validator-boundary dark",
  colours: {
    ...classic.colours,
    text_muted: "#8D9DA7",
    line: "#5E7480",
  },
};
const themes = [classic, highContrast, boundary];
const workspaces = [
  "Briefing",
  "Monitor",
  "Broadcast",
  "Extensions",
  "Plan",
  "Materials",
  "Population",
  "Markets",
  "Archive",
];

function readTheme(file: string): ThemeDocument {
  return JSON.parse(readFileSync(resolve(file), "utf8")) as ThemeDocument;
}

async function applyTheme(page: Page, theme: ThemeDocument): Promise<void> {
  await page.evaluate((documentTheme) => {
    const alpha = (hex: string, opacity: number) => {
      const red = Number.parseInt(hex.slice(1, 3), 16);
      const green = Number.parseInt(hex.slice(3, 5), 16);
      const blue = Number.parseInt(hex.slice(5, 7), 16);
      return `rgba(${red}, ${green}, ${blue}, ${opacity})`;
    };
    const roles: Record<string, string> = {
      canvas: "--colour-canvas",
      surface: "--colour-surface",
      surface_raised: "--colour-surface-raised",
      surface_soft: "--colour-surface-soft",
      text: "--colour-text",
      text_muted: "--colour-muted",
      line: "--colour-line",
      accent: "--colour-gold",
      observed: "--colour-observed",
      risk: "--colour-risk",
      success: "--colour-success",
      comparison: "--colour-violet",
    };
    for (const [role, variable] of Object.entries(roles)) {
      document.documentElement.style.setProperty(
        variable,
        documentTheme.colours[role],
      );
    }
    document.documentElement.style.setProperty(
      "--colour-line-faint",
      alpha(documentTheme.colours.line, 0.45),
    );
    document.documentElement.style.setProperty(
      "--colour-observed-soft",
      alpha(documentTheme.colours.observed, 0.11),
    );
    document.documentElement.style.setProperty(
      "--colour-gold-soft",
      alpha(documentTheme.colours.accent, 0.11),
    );
    document.documentElement.style.setProperty(
      "--colour-risk-soft",
      alpha(documentTheme.colours.risk, 0.11),
    );
    document.documentElement.style.setProperty(
      "--colour-success-soft",
      alpha(documentTheme.colours.success, 0.11),
    );
    document.documentElement.style.setProperty(
      "--colour-overlay",
      alpha(documentTheme.colours.canvas, 0.94),
    );
    documentTheme.chart_palette.forEach((colour, index) =>
      document.documentElement.style.setProperty(
        `--chart-colour-${index + 1}`,
        colour,
      ),
    );
  }, theme);
}

for (const theme of themes) {
  test.describe(theme.name, () => {
    for (const workspace of workspaces) {
      test(`${workspace} has no contrast regression`, async ({
        page,
      }, testInfo) => {
        await page.goto("/");
        await applyTheme(page, theme);
        await page
          .getByRole("navigation")
          .getByRole("button", { name: workspace })
          .click();
        await page.waitForTimeout(80);
        await audit(page, testInfo, `${theme.name} / ${workspace}`);
      });
    }

    test("dialogs, tooltips, focus, controls and disabled states remain readable", async ({
      page,
    }, testInfo) => {
      test.setTimeout(60_000);
      await page.goto("/");
      await applyTheme(page, theme);
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      await page.getByRole("button", { name: /^Interface language/ }).click();
      await page
        .getByRole("button", { name: "Community language packs" })
        .focus();
      await audit(page, testInfo, `${theme.name} / Language dialog`);
      await page.getByRole("button", { name: "Close" }).last().click();
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      await page.getByRole("button", { name: /^Validated theme/ }).click();
      await page.getByRole("button", { name: "Data-only themes" }).click();
      await expect(page.getByRole("tooltip")).toBeVisible();
      await audit(page, testInfo, `${theme.name} / Theme dialog`);
      await page.getByRole("button", { name: "Close" }).click();
      await page.getByRole("button", { name: "Save observer status" }).click();
      await audit(page, testInfo, `${theme.name} / Observation dialog`);
      await page.keyboard.press("Escape");
      await page.getByRole("button", { name: "Legal & notices" }).click();
      await page.getByRole("tab", { name: "Read-only research" }).click();
      await page.getByRole("button", { name: "Open research setup" }).click();
      await audit(page, testInfo, `${theme.name} / Research setup dialog`);
      await page.getByRole("button", { name: "Close" }).last().click();
      await page.keyboard.press("Tab");
      await audit(page, testInfo, `${theme.name} / Keyboard focus`);
      const optionFailures = await page
        .locator("option")
        .evaluateAll((options) =>
          options.flatMap((option) => {
            const style = getComputedStyle(option);
            return style.color === style.backgroundColor
              ? [{ text: option.textContent, colour: style.color }]
              : [];
          }),
        );
      expect(
        optionFailures,
        "native option foreground must differ from its background",
      ).toEqual([]);
    });
  });
}

async function audit(
  page: Page,
  testInfo: TestInfo,
  label: string,
): Promise<void> {
  const axe = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
    .analyze();
  const styleFailures = await page.evaluate(auditInterfaceContrast);
  await testInfo.attach("contrast-report", {
    body: JSON.stringify(
      { label, axe: axe.violations, computedStyle: styleFailures },
      null,
      2,
    ),
    contentType: "application/json",
  });
  expect(axe.violations, `${label}: Axe accessibility violations`).toEqual([]);
  expect(styleFailures, `${label}: computed-style contrast failures`).toEqual(
    [],
  );
}
