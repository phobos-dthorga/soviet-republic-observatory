import { expect, test, type Page, type TestInfo } from "@playwright/test";

type LayoutFailure = {
  kind: string;
  selector: string;
  detail: string;
};

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
  const failures = await page.evaluate((): LayoutFailure[] => {
    const tolerance = 1.5;
    const viewport = {
      width: document.documentElement.clientWidth,
      height: document.documentElement.clientHeight,
    };
    const result: LayoutFailure[] = [];
    const visible = (element: Element): element is HTMLElement => {
      const node = element as HTMLElement;
      const style = getComputedStyle(node);
      const box = node.getBoundingClientRect();
      return (
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        Number(style.opacity) > 0 &&
        box.width > 0 &&
        box.height > 0
      );
    };
    const identity = (element: Element): string => {
      const node = element as HTMLElement;
      if (node.id) return `#${node.id}`;
      const classes = [...node.classList].join(".");
      return `${node.tagName.toLowerCase()}${classes ? `.${classes}` : ""}`;
    };

    if (document.documentElement.scrollWidth > viewport.width + tolerance) {
      result.push({
        kind: "document-overflow",
        selector: "html",
        detail: `${document.documentElement.scrollWidth}px exceeds ${viewport.width}px`,
      });
    }

    for (const element of document.querySelectorAll(
      ".shell, .command-bar, .observation-bar, .workspace, .status-bar, [role='dialog']",
    )) {
      if (!visible(element)) continue;
      const box = element.getBoundingClientRect();
      if (box.left < -tolerance || box.right > viewport.width + tolerance) {
        result.push({
          kind: "landmark-horizontal-escape",
          selector: identity(element),
          detail: `left ${box.left.toFixed(1)}, right ${box.right.toFixed(1)}, viewport ${viewport.width}`,
        });
      }
      if (
        element.matches("[role='dialog']") &&
        (box.top < -tolerance || box.bottom > viewport.height + tolerance)
      ) {
        result.push({
          kind: "dialog-vertical-escape",
          selector: identity(element),
          detail: `top ${box.top.toFixed(1)}, bottom ${box.bottom.toFixed(1)}, viewport ${viewport.height}`,
        });
      }
    }

    for (const control of document.querySelectorAll(
      "button:not(:disabled), select:not(:disabled), input:not(:disabled), a[href]",
    )) {
      if (!visible(control)) continue;
      const box = control.getBoundingClientRect();
      const intersectsViewport =
        box.right > 0 &&
        box.left < viewport.width &&
        box.bottom > 0 &&
        box.top < viewport.height;
      if (!intersectsViewport) continue;
      if (box.width + tolerance < 24 || box.height + tolerance < 24) {
        result.push({
          kind: "undersized-control",
          selector: identity(control),
          detail: `${box.width.toFixed(1)} × ${box.height.toFixed(1)}px`,
        });
      }
    }

    for (const dialog of document.querySelectorAll("[role='dialog']")) {
      if (!visible(dialog)) continue;
      const dialogBox = dialog.getBoundingClientRect();
      const header = dialog.querySelector(":scope > header");
      const footer = dialog.querySelector(":scope > footer");
      for (const [name, region] of [
        ["header", header],
        ["footer", footer],
      ] as const) {
        if (!region || !visible(region)) continue;
        const box = region.getBoundingClientRect();
        if (
          box.left < dialogBox.left - tolerance ||
          box.right > dialogBox.right + tolerance ||
          box.top < dialogBox.top - tolerance ||
          box.bottom > dialogBox.bottom + tolerance
        ) {
          result.push({
            kind: "dialog-region-escape",
            selector: `${identity(dialog)} > ${name}`,
            detail: `${name} escapes its dialog bounds`,
          });
        }
      }
      if (header && footer && visible(header) && visible(footer)) {
        if (
          header.getBoundingClientRect().bottom >
          footer.getBoundingClientRect().top + tolerance
        ) {
          result.push({
            kind: "dialog-order",
            selector: identity(dialog),
            detail: "header and footer overlap or are out of order",
          });
        }
      }
    }

    return result;
  });

  await testInfo.attach("layout-report", {
    body: JSON.stringify({ label, failures }, null, 2),
    contentType: "application/json",
  });
  expect(failures, `${label}: interface geometry failures`).toEqual([]);
}
