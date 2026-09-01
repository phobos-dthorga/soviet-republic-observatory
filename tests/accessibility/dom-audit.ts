export type InterfaceAuditFailure = {
  kind: string;
  selector: string;
  detail: string;
};

/**
 * Keep this function self-contained. Playwright and the native WebDriver suite
 * serialize it into the inspected webview instead of importing application
 * policy into presentation components.
 */
export function auditInterfaceDom(): InterfaceAuditFailure[] {
  const tolerance = 1.5;
  const viewport = {
    width: document.documentElement.clientWidth,
    height: document.documentElement.clientHeight,
  };
  const result: InterfaceAuditFailure[] = [];
  const visible = (element: Element): element is HTMLElement => {
    const node = element as HTMLElement;
    if (node.classList.contains("visually-hidden")) return false;
    if (
      node.getAttribute("aria-hidden") === "true" &&
      node.getAttribute("tabindex") === "-1"
    )
      return false;
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
  const pixels = (value: string): number => Number.parseFloat(value) || 0;

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

  for (const tooltip of document.querySelectorAll("[role='tooltip']")) {
    if (!visible(tooltip)) continue;
    const box = tooltip.getBoundingClientRect();
    if (
      box.left < -tolerance ||
      box.right > viewport.width + tolerance ||
      box.top < -tolerance ||
      box.bottom > viewport.height + tolerance
    ) {
      result.push({
        kind: "tooltip-viewport-escape",
        selector: identity(tooltip),
        detail: `left ${box.left.toFixed(1)}, top ${box.top.toFixed(1)}, right ${box.right.toFixed(1)}, bottom ${box.bottom.toFixed(1)}, viewport ${viewport.width} × ${viewport.height}`,
      });
    }
    for (const control of document.querySelectorAll(
      "button, select, input, a[href]",
    )) {
      if (tooltip.contains(control) || !visible(control)) continue;
      const controlBox = control.getBoundingClientRect();
      const intersection = {
        left: Math.max(box.left, controlBox.left),
        top: Math.max(box.top, controlBox.top),
        right: Math.min(box.right, controlBox.right),
        bottom: Math.min(box.bottom, controlBox.bottom),
      };
      if (
        intersection.right - intersection.left <= tolerance ||
        intersection.bottom - intersection.top <= tolerance
      )
        continue;
      const topElement = document.elementFromPoint(
        (intersection.left + intersection.right) / 2,
        (intersection.top + intersection.bottom) / 2,
      );
      if (
        topElement &&
        (topElement === control || control.contains(topElement))
      ) {
        result.push({
          kind: "tooltip-occlusion",
          selector: identity(tooltip),
          detail: `${identity(control)} paints above the tooltip`,
        });
        break;
      }
    }
  }

  for (const group of document.querySelectorAll(
    "[data-aligned-action-group]",
  )) {
    if (!visible(group)) continue;
    const selector = identity(group);
    const entries = [
      ...group.querySelectorAll(":scope > [data-aligned-action-item]"),
    ]
      .filter(visible)
      .map((item) => ({
        item,
        itemBox: item.getBoundingClientRect(),
        actions: [...item.querySelectorAll("[data-aligned-action]")].filter(
          visible,
        ),
      }));
    if (entries.length < 2) continue;
    for (const entry of entries) {
      if (entry.actions.length !== 1) {
        result.push({
          kind: "aligned-action-contract",
          selector: identity(entry.item),
          detail: `expected one visible aligned action; found ${entry.actions.length}`,
        });
      }
    }
    const completeEntries = entries.filter(
      (entry) => entry.actions.length === 1,
    );
    const rows: (typeof completeEntries)[] = [];
    for (const entry of completeEntries) {
      const row = rows.find(
        (candidate) =>
          Math.abs(candidate[0].itemBox.top - entry.itemBox.top) <= tolerance,
      );
      if (row) row.push(entry);
      else rows.push([entry]);
    }
    for (const row of rows.filter((candidate) => candidate.length > 1)) {
      const actionBottoms = row.map(
        (entry) => entry.actions[0].getBoundingClientRect().bottom,
      );
      const minimum = Math.min(...actionBottoms);
      const maximum = Math.max(...actionBottoms);
      if (maximum - minimum > tolerance) {
        result.push({
          kind: "aligned-action-edge",
          selector,
          detail: `peer action lower edges differ by ${(maximum - minimum).toFixed(1)}px`,
        });
      }
    }
  }

  const guidanceLayouts = new Set(["block", "compact", "inline"]);
  for (const surface of document.querySelectorAll("[data-guidance-surface]")) {
    if (!visible(surface)) continue;
    const node = surface as HTMLElement;
    const selector = identity(surface);
    const layout = node.dataset.guidanceLayout ?? "";
    const style = getComputedStyle(node);
    const box = node.getBoundingClientRect();
    if (!guidanceLayouts.has(layout)) {
      result.push({
        kind: "guidance-layout",
        selector,
        detail: `unsupported layout '${layout || "missing"}'`,
      });
      continue;
    }
    const minimumPadding = layout === "block" ? 10 : 8;
    const padding = {
      top: pixels(style.paddingTop),
      right: pixels(style.paddingRight),
      bottom: pixels(style.paddingBottom),
      left: pixels(style.paddingLeft),
    };
    if (
      Math.min(padding.top, padding.right, padding.bottom, padding.left) <
      minimumPadding
    ) {
      result.push({
        kind: "guidance-padding",
        selector,
        detail: `${layout} padding is ${padding.top}/${padding.right}/${padding.bottom}/${padding.left}px; minimum ${minimumPadding}px`,
      });
    }
    const lineHeight = pixels(style.lineHeight);
    if (
      lineHeight > 0 &&
      box.height + tolerance < lineHeight + padding.top + padding.bottom
    ) {
      result.push({
        kind: "guidance-collapse",
        selector,
        detail: `${box.height.toFixed(1)}px cannot contain ${lineHeight.toFixed(1)}px line height and vertical padding`,
      });
    }
    if (
      (style.overflowY === "hidden" || style.overflow === "hidden") &&
      node.scrollHeight > node.clientHeight + tolerance
    ) {
      result.push({
        kind: "guidance-clipping",
        selector,
        detail: `${node.scrollHeight}px content is clipped to ${node.clientHeight}px`,
      });
    }
  }

  return result;
}
