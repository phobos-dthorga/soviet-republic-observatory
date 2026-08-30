import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type TestInfo } from "@playwright/test";

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
  "Materials",
  "Population",
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

async function computedContrastFailures(page: Page) {
  return page.evaluate(() => {
    type Rgba = [number, number, number, number];
    const parse = (value: string): Rgba | null => {
      const match = value.match(/rgba?\(([^)]+)\)/);
      if (!match) return null;
      const channels = match[1].split(/[, ]+/).filter(Boolean).map(Number);
      return [channels[0], channels[1], channels[2], channels[3] ?? 1];
    };
    const composite = (top: Rgba, bottom: Rgba): Rgba => {
      const alpha = top[3] + bottom[3] * (1 - top[3]);
      if (alpha === 0) return [0, 0, 0, 0];
      return [
        (top[0] * top[3] + bottom[0] * bottom[3] * (1 - top[3])) / alpha,
        (top[1] * top[3] + bottom[1] * bottom[3] * (1 - top[3])) / alpha,
        (top[2] * top[3] + bottom[2] * bottom[3] * (1 - top[3])) / alpha,
        alpha,
      ];
    };
    const background = (element: Element): Rgba => {
      let result: Rgba = [255, 255, 255, 1];
      const layers: Rgba[] = [];
      let current: Element | null = element;
      while (current) {
        const parsed = parse(getComputedStyle(current).backgroundColor);
        if (parsed && parsed[3] > 0) layers.push(parsed);
        current = current.parentElement;
      }
      for (const layer of layers.reverse()) result = composite(layer, result);
      return result;
    };
    const linear = (channel: number) => {
      const value = channel / 255;
      return value <= 0.04045
        ? value / 12.92
        : ((value + 0.055) / 1.055) ** 2.4;
    };
    const luminance = (colour: Rgba) =>
      0.2126 * linear(colour[0]) +
      0.7152 * linear(colour[1]) +
      0.0722 * linear(colour[2]);
    const ratio = (first: Rgba, second: Rgba) => {
      const a = luminance(first);
      const b = luminance(second);
      return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
    };
    const selector = [
      "body *:not(script):not(style):not(svg):not(path):not(canvas)",
      "option",
      "optgroup",
    ].join(",");
    return [...document.querySelectorAll(selector)].flatMap(
      (element, index) => {
        const node = element as HTMLElement;
        const style = getComputedStyle(node);
        const visible =
          style.display !== "none" &&
          style.visibility !== "hidden" &&
          Number(style.opacity) > 0 &&
          (node.innerText?.trim() || node instanceof HTMLOptionElement);
        if (
          !visible ||
          [...element.children].some((child) =>
            (child as HTMLElement).innerText?.trim(),
          )
        ) {
          return [];
        }
        const foreground = parse(style.color);
        if (!foreground) return [];
        const effectiveForeground = composite(foreground, background(element));
        const measured = ratio(effectiveForeground, background(element));
        const large =
          parseFloat(style.fontSize) >= 24 ||
          (parseFloat(style.fontSize) >= 18.66 &&
            Number(style.fontWeight) >= 700);
        const required = large ? 3 : 4.5;
        if (measured + 0.01 >= required) return [];
        const identity = node.id
          ? `#${node.id}`
          : `${node.tagName.toLowerCase()}.${[...node.classList].join(".") || "unclassified"}:nth-audit(${index})`;
        return [
          {
            selector: identity,
            role: node.getAttribute("role") ?? node.tagName.toLowerCase(),
            foreground: style.color,
            background: getComputedStyle(element).backgroundColor,
            measured: Number(measured.toFixed(2)),
            required,
            text: node.innerText?.trim().slice(0, 100),
          },
        ];
      },
    );
  });
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
      await page.goto("/");
      await applyTheme(page, theme);
      await page.getByRole("button", { name: /^Language/ }).click();
      await page
        .getByRole("button", { name: "Community language packs" })
        .focus();
      await audit(page, testInfo, `${theme.name} / Language dialog`);
      await page.getByRole("button", { name: "Close" }).click();
      await page.locator(".theme-button").evaluate((button) => {
        (button as HTMLButtonElement).disabled = false;
      });
      await page.getByRole("button", { name: "Theme" }).click();
      await audit(page, testInfo, `${theme.name} / Theme dialog`);
      await page.getByRole("button", { name: "Close" }).click();
      await page.getByRole("button", { name: "Save observer status" }).click();
      await audit(page, testInfo, `${theme.name} / Observation dialog`);
      await page.keyboard.press("Escape");
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
  const styleFailures = await computedContrastFailures(page);
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
