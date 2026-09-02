import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import AxeBuilder from "@axe-core/webdriverio";
import { auditInterfaceContrast } from "../accessibility/contrast-audit";
import { auditInterfaceDom } from "../accessibility/dom-audit";
import {
  UI_REVIEW_SCENARIOS,
  type UiReviewScenarioId,
  type UiReviewThemeId,
} from "../../src/lib/ui-review/scenarios";

type ReviewFinding = {
  source: "geometry" | "contrast" | "axe";
  rule: string;
  selector: string;
  detail: string;
};

type ReviewResult = {
  scenario: UiReviewScenarioId;
  theme: UiReviewThemeId;
  layout: string;
  viewport: { width: number; height: number; text_scale: number };
  findings: ReviewFinding[];
};

const layouts = [
  { label: "narrow", width: 720, height: 820, textScale: 100 },
  { label: "laptop", width: 1280, height: 720, textScale: 100 },
  { label: "fhd-125", width: 1536, height: 864, textScale: 125 },
  { label: "qhd-150", width: 1707, height: 960, textScale: 150 },
  { label: "ultrawide-150", width: 2293, height: 959, textScale: 150 },
  { label: "uhd-200", width: 1920, height: 1080, textScale: 200 },
  { label: "native-ultrawide", width: 3440, height: 1439, textScale: 100 },
] as const;
const smokeScenarios: UiReviewScenarioId[] = [
  "workspace-briefing",
  "workspace-plan",
  "materials-warehouse-attention",
  "production-pathway",
  "population-probe-missing",
  "workspace-environment",
  "workspace-markets",
  "critical-task-failed",
  "dialog-theme",
  "dialog-settings",
  "dialog-research",
  "dialog-recovery",
  "notification-error",
  "tooltip-contextual",
  "attention-cue",
  "keyboard-focus",
  "native-dropdown",
];

export async function runNativeReview(
  client: WebdriverIO.Browser,
): Promise<void> {
  const artifactRoot = required("UI_REVIEW_ARTIFACT_ROOT");
  const suite = required("UI_REVIEW_SUITE");
  const results: ReviewResult[] = [];

  await waitForReviewController(client, artifactRoot);
  const nativeScenarios = await client.execute(() =>
    window.__REPUBLIC_OBSERVATORY_UI_REVIEW__?.listScenarios(),
  );
  if (JSON.stringify(nativeScenarios) !== JSON.stringify(UI_REVIEW_SCENARIOS)) {
    throw new Error(
      "The native host and review runner scenario catalogues differ.",
    );
  }
  await proveAuditorRejectsCollapsedGuidance(client);

  const selectedScenarios =
    suite === "full" ? UI_REVIEW_SCENARIOS : smokeScenarios;
  const selectedThemes: UiReviewThemeId[] =
    suite === "full" ? ["classic", "high-contrast", "boundary"] : ["classic"];
  const selectedLayouts = suite === "full" ? layouts : [layouts[1]];

  for (const layout of selectedLayouts) {
    await client.setWindowSize(layout.width, layout.height);
    await runController(client, "setTextScale", layout.textScale);
    for (const theme of selectedThemes) {
      await runController(client, "selectTheme", theme);
      for (const scenario of selectedScenarios) {
        await runController(client, "selectScenario", scenario);
        await prepareInteractiveScenario(client, scenario);
        const findings = await inspectState(client);
        const result: ReviewResult = {
          scenario,
          theme,
          layout: layout.label,
          viewport: {
            width: layout.width,
            height: layout.height,
            text_scale: layout.textScale,
          },
          findings,
        };
        results.push(result);
        const name = safeName(`${layout.label}-${theme}-${scenario}`);
        await client.saveScreenshot(join(artifactRoot, `${name}.png`));
        await persistResults(artifactRoot, suite, results);
      }
    }
  }

  const failures = results.reduce(
    (count, result) => count + result.findings.length,
    0,
  );
  if (failures > 0) {
    throw new Error(
      `Native UI review found ${failures} blocking issue(s). See findings.json.`,
    );
  }

  await runController(client, "selectScenario", "native-dropdown");
  const nativeSelect = await client.$("select:not(:disabled)");
  await nativeSelect.waitForDisplayed();
  await scrollWithinInterface(client, "select:not(:disabled)");
  await nativeSelect.click();
  await client.keys(["ArrowDown", "Enter", "Tab"]);
  const focusVisible = await client.execute(() => {
    const active = document.activeElement;
    return Boolean(active && active !== document.body);
  });
  if (!focusVisible)
    throw new Error("Keyboard focus did not enter the interface.");
}

async function prepareInteractiveScenario(
  client: WebdriverIO.Browser,
  scenario: UiReviewScenarioId,
): Promise<void> {
  if (scenario === "workspace-briefing") {
    const sectionLink = await client.$(".workspace .section-list a:last-child");
    await sectionLink.waitForDisplayed();
    await sectionLink.click();
    const shellPosition = await client.execute(() => ({
      rootScroll: Math.max(
        window.scrollY,
        document.documentElement.scrollTop,
        document.body.scrollTop,
      ),
      commandTop:
        document
          .querySelector<HTMLElement>(".command-bar")
          ?.getBoundingClientRect().top ?? -1,
      saveTop:
        document
          .querySelector<HTMLElement>(".observation-bar")
          ?.getBoundingClientRect().top ?? -1,
    }));
    if (
      shellPosition.rootScroll !== 0 ||
      shellPosition.commandTop !== 0 ||
      shellPosition.saveTop < 69
    ) {
      throw new Error(
        "A workspace section link moved the global navigation bars off-screen.",
      );
    }
  } else if (scenario === "tooltip-contextual") {
    const trigger = await client.$(
      "[data-help-topic='metric-context-source-stats-citizens-adults'] button",
    );
    await trigger.waitForDisplayed();
    await scrollWithinInterface(
      client,
      "[data-help-topic='metric-context-source-stats-citizens-adults'] button",
    );
    await trigger.click();
    const tooltip = await client.$("[role='tooltip']");
    await tooltip.waitForDisplayed();
    const text = await tooltip.getText();
    if (
      !text.includes("W&R's source-defined adult class") ||
      !text.includes("Not employed workers or active workers")
    ) {
      throw new Error(
        "The Metric Context tooltip omitted its source boundary.",
      );
    }
  } else if (scenario === "keyboard-focus") {
    await client.keys(["Tab", "Tab"]);
  } else if (scenario === "native-dropdown") {
    const nativeSelect = await client.$("select:not(:disabled)");
    await nativeSelect.waitForDisplayed();
    await scrollWithinInterface(client, "select:not(:disabled)");
    await nativeSelect.click();
    await client.keys(["ArrowDown", "Enter"]);
  } else if (scenario === "attention-cue") {
    await (await client.$(".attention-cue.active")).waitForDisplayed();
  } else if (scenario === "dialog-recovery") {
    await (await client.$(".recovery-dialog")).waitForDisplayed();
  } else if (scenario === "dialog-settings") {
    await (await client.$(".settings-dialog")).waitForDisplayed();
  } else if (scenario === "notification-error") {
    await (await client.$(".notification-toast")).waitForDisplayed();
  } else if (scenario === "production-pathway") {
    const pathway = await client.$(".pathway-laboratory");
    await pathway.waitForDisplayed();
    await scrollWithinInterface(client, ".pathway-laboratory");
  }
}

async function scrollWithinInterface(
  client: WebdriverIO.Browser,
  selector: string,
): Promise<void> {
  await client.execute((requestedSelector) => {
    const element = document.querySelector<HTMLElement>(requestedSelector);
    if (!element) return;

    let container = element.parentElement;
    while (container && container !== document.body) {
      const style = getComputedStyle(container);
      const scrollable =
        /(auto|scroll)/.test(style.overflowY) &&
        container.scrollHeight > container.clientHeight;
      if (scrollable) break;
      container = container.parentElement;
    }
    if (!container || container === document.body) return;

    const elementBox = element.getBoundingClientRect();
    const containerBox = container.getBoundingClientRect();
    const centredTop =
      container.scrollTop +
      elementBox.top -
      containerBox.top -
      (container.clientHeight - elementBox.height) / 2;
    container.scrollTo({
      top: Math.max(0, centredTop),
      left: 0,
      behavior: "instant",
    });
  }, selector);
}

async function proveAuditorRejectsCollapsedGuidance(
  client: WebdriverIO.Browser,
): Promise<void> {
  await client.execute(() => {
    const fixture = document.createElement("div");
    fixture.id = "native-collapsed-guidance-fixture";
    fixture.className = "guidance-surface";
    fixture.dataset.guidanceSurface = "instruction";
    fixture.dataset.guidanceLayout = "compact";
    fixture.textContent = "Collapsed guidance regression fixture.";
    fixture.style.padding = "0";
    fixture.style.minBlockSize = "0";
    document.body.append(fixture);
  });
  const findings = await client.execute(auditInterfaceDom);
  await client.execute(() =>
    document.querySelector("#native-collapsed-guidance-fixture")?.remove(),
  );
  if (!findings.some((finding) => finding.kind === "guidance-padding")) {
    throw new Error("The native geometry auditor accepted collapsed guidance.");
  }
}

async function inspectState(
  client: WebdriverIO.Browser,
): Promise<ReviewFinding[]> {
  const geometry = await client.execute(auditInterfaceDom);
  const contrast = await client.execute(auditInterfaceContrast);
  const axe = await new AxeBuilder({ client })
    .setLegacyMode()
    .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
    .analyze();
  return [
    ...geometry.map((finding) => ({
      source: "geometry" as const,
      rule: finding.kind,
      selector: finding.selector,
      detail: finding.detail,
    })),
    ...contrast.map((finding) => ({
      source: "contrast" as const,
      rule: "computed-style-contrast",
      selector: finding.selector,
      detail: `${finding.measured}:1; requires ${finding.required}:1 for ${finding.role}`,
    })),
    ...axe.violations.flatMap((violation) =>
      violation.nodes.map((node) => ({
        source: "axe" as const,
        rule: violation.id,
        selector: node.target.join(" "),
        detail: node.failureSummary ?? violation.help,
      })),
    ),
  ];
}

async function runController(
  client: WebdriverIO.Browser,
  operation: "selectScenario" | "selectTheme" | "setTextScale",
  value: UiReviewScenarioId | UiReviewThemeId | number,
): Promise<void> {
  const result = await client.executeAsync(
    (
      requestedOperation: "selectScenario" | "selectTheme" | "setTextScale",
      requestedValue: UiReviewScenarioId | UiReviewThemeId | number,
      done: (result: { ok: boolean; error?: string }) => void,
    ) => {
      const controller = window.__REPUBLIC_OBSERVATORY_UI_REVIEW__;
      if (!controller) {
        done({ ok: false, error: "ui_review_controller_unavailable" });
        return;
      }
      const promise =
        requestedOperation === "selectScenario"
          ? controller.selectScenario(requestedValue as UiReviewScenarioId)
          : requestedOperation === "selectTheme"
            ? controller.selectTheme(requestedValue as UiReviewThemeId)
            : controller.setTextScale(requestedValue as number);
      promise
        .then(() => done({ ok: true }))
        .catch((error) => done({ ok: false, error: String(error) }));
    },
    operation,
    value,
  );
  if (!result.ok) throw new Error(result.error ?? "ui_review_operation_failed");
}

async function waitForReviewController(
  client: WebdriverIO.Browser,
  artifactRoot: string,
): Promise<void> {
  try {
    await client.waitUntil(
      () =>
        client.execute(
          () =>
            Boolean(window.__REPUBLIC_OBSERVATORY_UI_REVIEW__) &&
            document.documentElement.dataset.uiReviewReady === "true",
        ),
      {
        timeout: 30_000,
        timeoutMsg:
          "The bounded native UI review controller did not become ready.",
      },
    );
  } catch (error) {
    const state = await client.execute(() => ({
      ready_state: document.readyState,
      title: document.title,
      tauri_host: Boolean(window.__TAURI_INTERNALS__),
      controller: Boolean(window.__REPUBLIC_OBSERVATORY_UI_REVIEW__),
      review_state: document.documentElement.dataset.uiReview ?? null,
      review_ready: document.documentElement.dataset.uiReviewReady ?? null,
    }));
    await writeFile(
      join(artifactRoot, "startup-state.json"),
      `${JSON.stringify(state, null, 2)}\n`,
      "utf8",
    );
    await client.saveScreenshot(join(artifactRoot, "startup-failure.png"));
    throw error;
  }
}

async function persistResults(
  artifactRoot: string,
  suite: string,
  results: ReviewResult[],
): Promise<void> {
  const failures = results.reduce(
    (count, result) => count + result.findings.length,
    0,
  );
  await writeFile(
    join(artifactRoot, "findings.json"),
    `${JSON.stringify({ schema_version: 1, suite, failures, results }, null, 2)}\n`,
    "utf8",
  );
  await writeFile(
    join(artifactRoot, "summary.md"),
    [
      "# Native UI review",
      "",
      `- Suite: ${suite}`,
      `- Reviewed states: ${results.length}`,
      `- Blocking findings: ${failures}`,
      "- Driver: external Tauri WebDriver; no global mouse injection",
      "",
    ].join("\n"),
    "utf8",
  );
}

function safeName(value: string): string {
  return value.replace(/[^a-z0-9_-]+/gi, "-").slice(0, 120);
}

function required(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`Missing native UI review setting ${name}.`);
  return value;
}
