import { expect, test, type Page } from "@playwright/test";

const idleProgress = {
  task_id: "research_probe_build",
  run_id: "not_started",
  state: "idle",
  phase: "idle",
  progress_percent: null,
  started_at_ms: null,
  updated_at_ms: null,
  current_item: null,
  log_lines: [],
  error_code: null,
  failed_stage: null,
  compiler_exit_code: null,
  remediation_code: null,
};

const completeProgress = {
  ...idleProgress,
  run_id: "ui-audit-complete",
  state: "complete",
  phase: "complete",
  progress_percent: 100,
  started_at_ms: 1,
  updated_at_ms: 2,
  current_item: "build-complete",
  log_lines: [
    "Reviewed source contract verified.",
    "Bounded research probe artifact verified.",
  ],
};

const readyStatus = {
  notice_revision: 1,
  notice_accepted: true,
  source_available: true,
  compiler_available: true,
  checkout_state: "reviewed",
  checkout_name: "Reviewed TesmioLoader checkout",
  reviewed_tesmio_revision: "3baa141f9f08921aea9c95f0a400289cabd9960a",
  probe_built: false,
  artifact_state: "absent",
  probe_content_hash: null,
  probe_size_bytes: null,
  output_display_path: null,
  last_built_at_ms: null,
  can_build: true,
  blockers: [],
  warnings: [],
  progress: idleProgress,
};

const completeStatus = {
  ...readyStatus,
  probe_built: true,
  artifact_state: "verified",
  probe_content_hash:
    "b54932ee577edab993c6ec1fffbb1984de9b069ae6d1123e1b1a2852592ccaf9",
  probe_size_bytes: 140_288,
  output_display_path:
    "research/tesmioloader-probe/build/observatory_probe.dll",
  last_built_at_ms: 2,
  progress: completeProgress,
};

async function openResearchSetup(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: "Legal & notices" }).click();
  await page.getByRole("tab", { name: "Read-only research" }).click();
  await page.getByRole("button", { name: "Open research setup" }).click();
  await expect(
    page.getByRole("heading", { name: "Experimental Research Setup" }),
  ).toBeVisible();
}

async function installResearchHostMock(page: Page): Promise<void> {
  await page.evaluate(
    ({ ready, complete, idle, finished }) => {
      let built = false;
      let callbackId = 0;
      const callbacks = new Map<number, (...args: unknown[]) => void>();
      Object.assign(globalThis, { isTauri: true });
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          transformCallback(callback: (...args: unknown[]) => void) {
            callbackId += 1;
            callbacks.set(callbackId, callback);
            return callbackId;
          },
          unregisterCallback(id: number) {
            callbacks.delete(id);
          },
          async invoke(command: string) {
            if (command === "plugin:event|listen") return 1;
            if (command === "plugin:event|unlisten") return null;
            if (command === "get_research_setup")
              return structuredClone(built ? complete : ready);
            if (command === "get_research_build_progress")
              return structuredClone(built ? finished : idle);
            if (command === "build_research_probe") {
              built = true;
              return structuredClone(complete);
            }
            if (command === "attention_cue_status") {
              return {
                cue_id: "research.setup.build",
                content_revision: 1,
                dismissed: true,
              };
            }
            throw new Error(`Unexpected UI-audit command: ${command}`);
          },
        },
        __TAURI_EVENT_PLUGIN_INTERNALS__: {
          unregisterListener() {},
        },
      });
    },
    {
      ready: readyStatus,
      complete: completeStatus,
      idle: idleProgress,
      finished: completeProgress,
    },
  );
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

test("completed build reveals a whole result region without a clipped step", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/");
  await installResearchHostMock(page);
  await page.getByRole("button", { name: "Legal & notices" }).click();
  await page.getByRole("tab", { name: "Read-only research" }).click();
  await page.getByRole("button", { name: "Open research setup" }).click();
  const dialog = page.getByRole("dialog", {
    name: "Experimental Research Setup",
  });
  await dialog.getByRole("button", { name: "Build research probe" }).click();
  await expect(
    dialog.getByRole("heading", { name: "Research probe build complete" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Dismiss notification" }).click();

  const alignment = await dialog.evaluate((node) => {
    const content = node.querySelector(".research-content");
    const results = node.querySelector(".research-results");
    if (!content || !results) return null;
    const contentBox = content.getBoundingClientRect();
    const resultsBox = results.getBoundingClientRect();
    const clippedStepHeadings = [
      ...node.querySelectorAll(".research-steps > li h3"),
    ]
      .map((heading) => heading.getBoundingClientRect())
      .filter(
        (box) => box.top < contentBox.top && box.bottom > contentBox.top + 1,
      ).length;
    return {
      resultsVisible: resultsBox.top < contentBox.bottom,
      clippedStepHeadings,
    };
  });
  expect(alignment).not.toBeNull();
  expect(alignment?.resultsVisible).toBe(true);
  expect(alignment?.clippedStepHeadings).toBe(0);
  await expect(dialog).toHaveScreenshot("research-build-complete.png", {
    animations: "disabled",
    caret: "hide",
    maxDiffPixelRatio: 0.01,
  });
});

test("theme assay renders readable running and failed critical-task states", async ({
  page,
}) => {
  await page.goto("/");
  await page.locator(".theme-button").evaluate((button) => {
    (button as HTMLButtonElement).disabled = false;
  });
  await page.getByRole("button", { name: "Theme" }).click();
  const preview = page.locator(".semantic-state-preview");
  const failed = preview.locator(".task-indicator.failed");
  await expect(failed.getByText("Failed", { exact: true })).toBeVisible();
  const failedStyle = await failed.locator("strong").evaluate((node) => {
    const style = getComputedStyle(node);
    return { foreground: style.color, background: style.backgroundColor };
  });
  expect(failedStyle.background).toBe("rgba(0, 0, 0, 0)");
  expect(failedStyle.foreground).not.toBe(failedStyle.background);
  await expect(preview).toHaveScreenshot("critical-task-states.png", {
    animations: "disabled",
    caret: "hide",
    maxDiffPixelRatio: 0.01,
  });
});
