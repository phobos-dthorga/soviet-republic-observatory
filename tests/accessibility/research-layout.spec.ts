import { expect, test, type Page } from "@playwright/test";
import { auditInterfaceDom } from "./dom-audit";

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

const idleDownloadProgress = {
  task_id: "research_source_download",
  run_id: "not_started",
  state: "idle",
  phase: "idle",
  progress_percent: null,
  transferred_bytes: 0,
  expected_bytes: null,
  started_at_ms: null,
  updated_at_ms: null,
  current_item: null,
  error_code: null,
};

const runningDownloadProgress = {
  ...idleDownloadProgress,
  run_id: "ui-audit-download",
  state: "running",
  phase: "downloading",
  progress_percent: 42,
  transferred_bytes: 640_000,
  expected_bytes: 1_450_928,
  started_at_ms: 10,
  updated_at_ms: 20,
  current_item: "reviewed_source_archive",
};

const completeDownloadProgress = {
  ...runningDownloadProgress,
  state: "complete",
  phase: "complete",
  progress_percent: 100,
  transferred_bytes: 1_450_928,
  updated_at_ms: 30,
  current_item: "download_complete",
};

const idleSessionProgress = {
  task_id: "research_session_preparation",
  run_id: "not_started",
  state: "idle",
  phase: "idle",
  progress_percent: null,
  started_at_ms: null,
  updated_at_ms: null,
  current_item: null,
  log_lines: [],
  error_code: null,
};

const runningSessionProgress = {
  ...idleSessionProgress,
  run_id: "ui-audit-session",
  state: "running",
  phase: "installing",
  progress_percent: 70,
  started_at_ms: 10,
  updated_at_ms: 20,
  current_item: "isolated_game_folder",
};

const completeSessionProgress = {
  ...runningSessionProgress,
  state: "complete",
  phase: "complete",
  progress_percent: 100,
  updated_at_ms: 30,
  current_item: "ready_for_confirmed_launch",
};

const readyStatus = {
  notice_revision: 4,
  notice_accepted: true,
  source_available: true,
  compiler_available: true,
  checkout_state: "reviewed",
  source_origin: "manual_checkout",
  checkout_name: "Reviewed TesmioLoader checkout",
  reviewed_tesmio_revision: "3baa141f9f08921aea9c95f0a400289cabd9960a",
  probe_built: false,
  artifact_state: "absent",
  probe_content_hash: null,
  probe_size_bytes: null,
  output_display_path: null,
  last_built_at_ms: null,
  can_build: true,
  can_download: true,
  blockers: [],
  warnings: [],
  progress: idleProgress,
  download_progress: idleDownloadProgress,
  session: {
    state: "prerequisites_required",
    game_configured: true,
    reviewed_loader_source_available: true,
    probe_ready: false,
    report_snapshot_count: 0,
    report_collection_stage: null,
    managed_folder: "W&R/tesmioloader/observatory",
    can_prepare: false,
    can_launch: false,
    writes_game_directory: true,
    writes_save_data: false,
    changes_running_game_memory: true,
    progress: idleSessionProgress,
  },
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
  session: {
    ...readyStatus.session,
    state: "ready_to_prepare",
    probe_ready: true,
    can_prepare: true,
  },
};

const preparedStatus = {
  ...completeStatus,
  session: {
    ...completeStatus.session,
    state: "prepared",
    can_prepare: true,
    can_launch: true,
    progress: completeSessionProgress,
  },
};

const waitingForWorldStatus = {
  ...preparedStatus,
  session: {
    ...preparedStatus.session,
    state: "report_available",
    report_snapshot_count: 0,
    report_collection_stage: "waiting_for_loaded_republic",
  },
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
    ({
      ready,
      complete,
      idle,
      finished,
      idleDownload,
      runningDownload,
      finishedDownload,
      idleSession,
      runningSession,
      finishedSession,
      prepared,
      waitingForWorld,
    }) => {
      let built = false;
      let downloaded = false;
      let sessionPrepared = false;
      let sessionLaunched = false;
      let callbackId = 0;
      const callbacks = new Map<number, (...args: unknown[]) => void>();
      const listeners = new Map<string, number>();
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
          async invoke(command: string, args?: Record<string, unknown>) {
            if (command === "plugin:event|listen") {
              if (
                typeof args?.event === "string" &&
                typeof args?.handler === "number"
              ) {
                listeners.set(args.event, args.handler);
              }
              return 1;
            }
            if (command === "plugin:event|unlisten") return null;
            if (command === "get_research_setup")
              return structuredClone(
                sessionLaunched
                  ? waitingForWorld
                  : sessionPrepared
                    ? prepared
                    : built
                      ? complete
                      : downloaded
                        ? {
                            ...ready,
                            source_origin: "observatory_downloaded",
                            download_progress: finishedDownload,
                          }
                        : ready,
              );
            if (command === "get_research_report_status") {
              return {
                state: sessionLaunched ? "available" : "missing",
                read_only: true,
                optional: true,
                persisted: false,
                probe_id: sessionLaunched
                  ? "org.republic-observatory.tesmio-readonly"
                  : null,
                probe_version: sessionLaunched ? "0.2.3" : null,
                loader_api_version: sessionLaunched ? 4 : null,
                target_game_version: sessionLaunched ? "1.1.1.9" : null,
                executable_timestamp: sessionLaunched ? 1782494893 : null,
                content_hash: null,
                snapshot_count: 0,
                sample_count: 0,
                latest_year: null,
                latest_day: null,
                latest_population_count: null,
                collection_stage: sessionLaunched
                  ? "waiting_for_loaded_republic"
                  : null,
                warnings: [],
              };
            }
            if (command === "get_research_build_progress")
              return structuredClone(built ? finished : idle);
            if (command === "get_research_source_download_progress")
              return structuredClone(
                downloaded ? finishedDownload : idleDownload,
              );
            if (command === "get_research_session_progress")
              return structuredClone(
                sessionPrepared ? finishedSession : idleSession,
              );
            if (command === "download_reviewed_tesmio_source") {
              const handler = listeners.get(
                "research-source-download-progress",
              );
              if (handler != null) {
                callbacks.get(handler)?.({
                  event: "research-source-download-progress",
                  id: 1,
                  payload: structuredClone(runningDownload),
                });
              }
              await new Promise((resolve) => setTimeout(resolve, 350));
              downloaded = true;
              if (handler != null) {
                callbacks.get(handler)?.({
                  event: "research-source-download-progress",
                  id: 1,
                  payload: structuredClone(finishedDownload),
                });
              }
              return structuredClone({
                ...ready,
                source_origin: "observatory_downloaded",
                download_progress: finishedDownload,
              });
            }
            if (command === "build_research_probe") {
              built = true;
              return structuredClone(complete);
            }
            if (command === "prepare_observation_only_session") {
              if (args?.gameDirectoryWriteConfirmed !== true) {
                throw new Error("Missing explicit game-folder consent");
              }
              const handler = listeners.get("research-session-progress");
              if (handler != null) {
                callbacks.get(handler)?.({
                  event: "research-session-progress",
                  id: 2,
                  payload: structuredClone(runningSession),
                });
              }
              await new Promise((resolve) => setTimeout(resolve, 350));
              sessionPrepared = true;
              if (handler != null) {
                callbacks.get(handler)?.({
                  event: "research-session-progress",
                  id: 2,
                  payload: structuredClone(finishedSession),
                });
              }
              return structuredClone(prepared);
            }
            if (command === "launch_observation_only_session") {
              if (args?.runningGameMemoryConfirmed !== true) {
                throw new Error("Missing explicit live-memory consent");
              }
              sessionLaunched = true;
              return structuredClone(waitingForWorld);
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
      idleDownload: idleDownloadProgress,
      runningDownload: runningDownloadProgress,
      finishedDownload: completeDownloadProgress,
      idleSession: idleSessionProgress,
      runningSession: runningSessionProgress,
      finishedSession: completeSessionProgress,
      prepared: preparedStatus,
      waitingForWorld: waitingForWorldStatus,
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
  const completedDialogBox = await dialog.boundingBox();
  expect(completedDialogBox).not.toBeNull();
  expect((completedDialogBox?.y ?? -1) >= 0).toBe(true);
  expect(
    (completedDialogBox?.y ?? 0) + (completedDialogBox?.height ?? 0),
  ).toBeLessThanOrEqual(1000);
  await expect(dialog).toHaveScreenshot("research-build-complete.png", {
    animations: "disabled",
    caret: "hide",
    maxDiffPixelRatio: 0.01,
  });
});

test("source confirmation yields to detailed foreground download progress", async ({
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

  await dialog
    .getByRole("button", { name: "Download reviewed source" })
    .click();
  const confirmation = page.getByRole("dialog", {
    name: "Download reviewed source from GitHub?",
  });
  await confirmation.getByRole("button", { name: "Download source" }).click();

  await expect(confirmation).toBeHidden();
  await expect(
    dialog.getByRole("heading", {
      name: "Receiving the reviewed source archive",
    }),
  ).toBeVisible();
  const progress = dialog.locator('[data-task-id="research_source_download"]');
  await expect(progress.getByText("640,000 B")).toBeVisible();
  await expect(progress.getByText("1,450,928 B")).toBeVisible();
  await expect(
    dialog.getByRole("heading", { name: "Reviewed source is ready" }),
  ).toBeVisible();
});

test("checked-session automation separates game-folder and live-launch consent", async ({
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
  await dialog
    .getByRole("button", { name: "Review and prepare session" })
    .click();
  const prepareConfirmation = page.getByRole("dialog", {
    name: "Prepare files inside the W&R folder?",
  });
  await expect(
    prepareConfirmation.getByText("No game executable"),
  ).toBeVisible();
  await expect(prepareConfirmation.getByText("save file")).toBeVisible();
  await prepareConfirmation
    .getByRole("button", { name: "Prepare checked session" })
    .click();

  await expect(
    dialog.getByRole("heading", {
      name: "Preparing the dedicated game folder",
    }),
  ).toBeVisible();
  await expect(
    dialog.getByRole("heading", { name: "Checked session is ready" }),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "Review and launch W&R" }).click();
  const launchConfirmation = page.getByRole("dialog", {
    name: "Launch the checked observation session?",
  });
  await expect(
    launchConfirmation.getByText("temporarily run inside the game"),
  ).toBeVisible();
  await expect(
    launchConfirmation.getByText("Normal gameplay can still save"),
  ).toBeVisible();
  await launchConfirmation
    .getByRole("button", { name: "Launch checked session" })
    .click();
  await expect(launchConfirmation).toBeHidden();
  await expect(dialog.getByText("Waiting for a loaded republic")).toHaveCount(
    2,
  );
});

test("theme assay renders readable non-interactive critical-task states", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: /^Validated theme/ }).click();
  const preview = page.locator(".semantic-state-preview");
  const failed = preview.locator(".task-indicator.failed");
  await expect(preview.getByRole("button")).toHaveCount(0);
  await expect(preview.locator('[data-display-mode="preview"]')).toHaveCount(2);
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
  await page.getByRole("button", { name: "Data-only themes" }).click();
  const tooltip = page.getByRole("tooltip");
  await expect(tooltip).toBeVisible();
  const tooltipStyle = await tooltip.evaluate((node) => {
    const style = getComputedStyle(node);
    return {
      borderInlineStartWidth: style.borderInlineStartWidth,
      backgroundImage: style.backgroundImage,
      interactiveChildren: node.querySelectorAll("button, a, input, select")
        .length,
    };
  });
  expect(tooltipStyle.borderInlineStartWidth).toBe("3px");
  expect(tooltipStyle.backgroundImage).not.toBe("none");
  expect(tooltipStyle.interactiveChildren).toBe(0);
});

test("enabled workspaces use the shared guidance surface without disguising controls", async ({
  page,
}) => {
  await page.goto("/");
  const workspaces = [
    "Briefing",
    "Monitor",
    "Broadcast",
    "Extensions",
    "Plan",
    "Materials",
    "Population",
    "Archive",
  ];

  for (const workspace of workspaces) {
    await page
      .getByRole("navigation")
      .getByRole("button", { name: workspace })
      .click();
    const visibleSurfaces = page.locator("[data-guidance-surface]:visible");
    await expect(
      visibleSurfaces.first(),
      `${workspace} guidance`,
    ).toBeVisible();
    const defects = await visibleSurfaces.evaluateAll((surfaces) =>
      surfaces.flatMap((surface) => {
        const style = getComputedStyle(surface);
        const previewControls =
          surface.getAttribute("data-guidance-surface") === "preview"
            ? surface.querySelectorAll("button, a, input, select, textarea")
                .length
            : 0;
        return style.borderInlineStartWidth === "3px" &&
          style.backgroundImage !== "none" &&
          previewControls === 0
          ? []
          : [
              {
                kind: surface.getAttribute("data-guidance-surface"),
                borderInlineStartWidth: style.borderInlineStartWidth,
                backgroundImage: style.backgroundImage,
                previewControls,
              },
            ];
      }),
    );
    expect(defects, `${workspace} guidance-surface contract`).toEqual([]);
  }

  await page
    .getByRole("navigation")
    .getByRole("button", { name: "Broadcast" })
    .click();
  await expect(page.locator(".causation-warning.guidance-surface")).toHaveCount(
    0,
  );
  await page
    .getByRole("navigation")
    .getByRole("button", { name: "Materials" })
    .click();
  await expect(
    page.locator(".sidebar-note:not(.guidance-surface)"),
  ).toHaveCount(1);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: /^Interface language/ }).click();
  await expect(page.locator(".language-boundary")).toHaveAttribute(
    "data-guidance-surface",
    "boundary",
  );
  await page.getByRole("button", { name: "Close" }).last().click();

  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await page.getByRole("button", { name: /^Validated theme/ }).click();
  await expect(page.locator(".theme-boundary")).toHaveAttribute(
    "data-guidance-surface",
    "boundary",
  );
  await page.getByRole("button", { name: "Close" }).last().click();

  await page.getByRole("button", { name: "Open Save Observer" }).click();
  await expect(page.locator(".observer-browser-state")).toHaveAttribute(
    "data-guidance-surface",
    "instruction",
  );
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");

  await page.locator(".diagnostics-button").evaluate((button) => {
    (button as HTMLButtonElement).disabled = false;
  });
  await page.getByRole("button", { name: "Diagnostics" }).click();
  await expect(page.locator(".diagnostics-boundary")).toHaveAttribute(
    "data-guidance-surface",
    "boundary",
  );
  await page.getByRole("button", { name: "Close" }).last().click();

  await page.getByRole("button", { name: "Legal & notices" }).click();
  await page.getByRole("tab", { name: "Read-only research" }).click();
  await page.getByRole("button", { name: "Open research setup" }).click();
  await expect(page.locator(".research-boundary")).toHaveAttribute(
    "data-guidance-surface",
    "boundary",
  );
});

test("guidance density rejects the collapsed Population-note regression", async ({
  page,
}) => {
  await page.goto("/");
  await page.evaluate(() => {
    const fixture = document.createElement("div");
    fixture.className = "guidance-surface population-probe-note";
    fixture.dataset.guidanceSurface = "instruction";
    fixture.dataset.guidanceLayout = "compact";
    fixture.textContent =
      "The companion remains entirely optional. Ordinary save observation continues without it.";
    document.body.append(fixture);
  });

  const note = page.locator(".population-probe-note");
  await expect(note).toBeVisible();
  expect(await page.evaluate(auditInterfaceDom)).toEqual([]);

  await note.evaluate((element) => {
    const node = element as HTMLElement;
    node.style.padding = "0";
    node.style.minBlockSize = "0";
  });
  const defects = await page.evaluate(auditInterfaceDom);
  expect(defects.some((defect) => defect.kind === "guidance-padding")).toBe(
    true,
  );
});

test("geometry audit rejects a tooltip that escapes the viewport", async ({
  page,
}) => {
  await page.goto("/");
  await page.evaluate(() => {
    const fixture = document.createElement("div");
    fixture.id = "escaping-tooltip-fixture";
    fixture.setAttribute("role", "tooltip");
    fixture.textContent = "Deliberately unreachable tooltip fixture";
    Object.assign(fixture.style, {
      position: "fixed",
      left: "-80px",
      top: "20px",
      width: "120px",
      height: "40px",
      background: "white",
      color: "black",
    });
    document.body.append(fixture);
  });

  const defects = await page.evaluate(auditInterfaceDom);
  expect(
    defects.some((defect) => defect.kind === "tooltip-viewport-escape"),
  ).toBe(true);
});

test("geometry audit rejects a control painted over a tooltip", async ({
  page,
}) => {
  await page.goto("/");
  await page.evaluate(() => {
    const tooltip = document.createElement("div");
    tooltip.id = "occluded-tooltip-fixture";
    tooltip.setAttribute("role", "tooltip");
    tooltip.textContent = "Tooltip evidence must remain readable";
    Object.assign(tooltip.style, {
      position: "fixed",
      zIndex: "10",
      left: "20px",
      top: "20px",
      width: "240px",
      height: "100px",
      background: "white",
      color: "black",
    });
    const control = document.createElement("button");
    control.textContent = "Occluding control";
    Object.assign(control.style, {
      position: "fixed",
      zIndex: "20",
      left: "60px",
      top: "50px",
      width: "120px",
      height: "40px",
    });
    document.body.append(tooltip, control);
  });

  const defects = await page.evaluate(auditInterfaceDom);
  expect(defects.some((defect) => defect.kind === "tooltip-occlusion")).toBe(
    true,
  );
});
