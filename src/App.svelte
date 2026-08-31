<script lang="ts">
  import { onMount } from "svelte";
  import BriefingWorkspace from "./lib/workspaces/BriefingWorkspace.svelte";
  import BroadcastWorkspace from "./lib/workspaces/BroadcastWorkspace.svelte";
  import ExtensionsWorkspace from "./lib/workspaces/ExtensionsWorkspace.svelte";
  import ArchiveWorkspace from "./lib/workspaces/ArchiveWorkspace.svelte";
  import MonitorWorkspace from "./lib/workspaces/MonitorWorkspace.svelte";
  import MaterialsWorkspace from "./lib/workspaces/MaterialsWorkspace.svelte";
  import PopulationWorkspace from "./lib/workspaces/PopulationWorkspace.svelte";
  import LanguageDialog from "./lib/i18n/LanguageDialog.svelte";
  import ThemeDialog from "./lib/theme/ThemeDialog.svelte";
  import { initialiseThemes } from "./lib/theme/service";
  import { activeLocale, translation } from "./lib/i18n/runtime";
  import type { TranslationKey } from "./lib/i18n/catalog";
  import { formatNumber } from "./lib/i18n/format";
  import ObservationDialog from "./lib/observations/ObservationDialog.svelte";
  import DiagnosticsDialog from "./lib/diagnostics/DiagnosticsDialog.svelte";
  import LegalDialog from "./lib/legal/LegalDialog.svelte";
  import ResearchSetupDialog from "./lib/research/ResearchSetupDialog.svelte";
  import TaskProgressIndicator from "./lib/tasks/TaskProgressIndicator.svelte";
  import NotificationCenter from "./lib/notifications/NotificationCenter.svelte";
  import { notify } from "./lib/notifications/service";
  import { observeLatestTaskProgress } from "./lib/tasks/progress";
  import { reinterpretationProgressView } from "./lib/tasks/reinterpretationProgress";
  import {
    getResearchBuildProgress,
    listenForResearchBuildProgress,
  } from "./lib/research/desktopClient";
  import { researchBuildProgressView } from "./lib/research/progress";
  import type { ResearchBuildProgress } from "./lib/research/types";
  import {
    clearDiagnosticLog,
    compareArchiveObservations,
    createTimelineContinuation,
    desktopHostAvailable,
    getArchiveOverview,
    getCatalogueStatus,
    getDiagnosticLog,
    getLatestReceiverDataset,
    getPopulationDataset,
    getReinterpretationProgress,
    getRecorderHealth,
    getSetupState,
    inspectArchiveObservation,
    listenForRecorderUpdates,
    listenForCatalogueProgress,
    listenForCompatibilityUpdates,
    listenForReinterpretationProgress,
    listenForWarehouseUpdates,
    selectTimelineBranch,
    returnToBranchTip,
    setTimelineBranchLabel,
  } from "./lib/observations/desktopClient";
  import type {
    ArchiveOverview,
    CatalogueRefreshProgress,
    CatalogueStatus,
    CompatibilityUpdate,
    DiagnosticLogView,
    ReceiverDataset,
    PopulationDataset,
    RecorderHealth,
    RecorderUpdate,
    ReinterpretationProgress,
    SetupState,
    WarehouseWriteActivity,
  } from "./lib/observations/types";

  type WorkspaceName =
    | "briefing"
    | "monitor"
    | "broadcast"
    | "extensions"
    | "materials"
    | "population"
    | "archive";
  const workspaces: Array<{
    id:
      | WorkspaceName
      | "plan"
      | "materials"
      | "population"
      | "markets"
      | "archive";
    label: TranslationKey;
    enabled: boolean;
  }> = [
    { id: "briefing", label: "nav-briefing", enabled: true },
    { id: "monitor", label: "nav-monitor", enabled: true },
    { id: "broadcast", label: "nav-broadcast", enabled: true },
    { id: "extensions", label: "nav-extensions", enabled: true },
    { id: "plan", label: "nav-plan", enabled: false },
    { id: "materials", label: "nav-materials", enabled: true },
    { id: "population", label: "nav-population", enabled: true },
    { id: "markets", label: "nav-markets", enabled: false },
    { id: "archive", label: "nav-archive", enabled: true },
  ];

  let activeWorkspace = $state<WorkspaceName>("briefing");
  let languageDialogOpen = $state(false);
  let themeDialogOpen = $state(false);
  let observationDialogOpen = $state(false);
  let diagnosticsDialogOpen = $state(false);
  let legalDialogOpen = $state(false);
  let researchSetupDialogOpen = $state(false);
  let diagnosticsBusy = $state(false);
  let diagnosticsError = $state("");
  let diagnosticLog = $state<DiagnosticLogView | null>(null);
  let catalogueProgress = $state<CatalogueRefreshProgress | null>(null);
  let warehouseStatus = $state<CatalogueStatus | null>(null);
  let reinterpretationProgress = $state<ReinterpretationProgress | null>(null);
  let researchBuildProgress = $state<ResearchBuildProgress | null>(null);
  const desktopAvailable = desktopHostAvailable();
  let setupState = $state<SetupState | null>(null);
  let receiverDataset = $state<ReceiverDataset | null>(null);
  let populationDataset = $state<PopulationDataset | null>(null);
  let archiveOverview = $state<ArchiveOverview | null>(null);
  let recorderHealth = $state<RecorderHealth | null>(null);
  const latestReceiverPoint = $derived(receiverDataset?.points.at(-1));

  function warehouseActivityLabel(activity: WarehouseWriteActivity): string {
    switch (activity.kind) {
      case "catalogue_publication":
        return $translation("catalogue-global-progress");
      case "observation_projection":
        return $translation("nav-monitor");
      case "overlay_projection":
        return $translation("catalogue-overlays");
      case "branch_membership_projection":
        return $translation("archive-branches");
      case "observation_rebuild":
        return $translation("catalogue-rebuild");
    }
  }

  function warehouseProgressPercent(
    activity: WarehouseWriteActivity | null,
  ): number | null {
    if (!activity || activity.rows_total === 0) return null;
    return Math.min(
      100,
      Math.round((activity.rows_processed / activity.rows_total) * 100),
    );
  }

  function warehouseProgressDetail(status: CatalogueStatus): string {
    const health = status.warehouse;
    if (health.failed_jobs > 0 || health.consecutive_write_failures > 0)
      return $translation("catalogue-global-attention");
    if (health.active_write?.rows_total) {
      return `${formatNumber(health.active_write.rows_processed, $activeLocale)} / ${formatNumber(health.active_write.rows_total, $activeLocale)} ${$translation("catalogue-progress-warehouse-rows")}`;
    }
    return `${health.pending_jobs} ${$translation("catalogue-pending-jobs")}`;
  }

  function activeBranchLabel(): string {
    if (!receiverDataset) return "planning-preview";
    const selected = archiveOverview?.branches.find(
      (branch) => branch.selected,
    );
    if (selected?.player_label) return selected.player_label;
    if (receiverDataset.branch_id === "main")
      return $translation("archive-branch-main");
    if (receiverDataset.branch_id === "unassigned")
      return $translation("archive-branch-unassigned");
    return $translation(
      selected?.origin === "manual_continuation"
        ? "archive-branch-continuation"
        : "archive-branch-fork",
      {
        identity:
          selected?.short_identity ?? receiverDataset.branch_id.slice(0, 12),
      },
    );
  }

  async function selectBranch(branchId: string): Promise<void> {
    const result = await selectTimelineBranch(branchId);
    applyAnalysisContext(result);
  }

  function applyAnalysisContext(result: {
    archive: ArchiveOverview;
    dataset: ReceiverDataset | null;
  }): void {
    archiveOverview = result.archive;
    receiverDataset = result.dataset;
    void refreshPopulationDataset();
  }

  async function refreshPopulationDataset(): Promise<void> {
    if (!desktopAvailable) return;
    try {
      populationDataset = await getPopulationDataset();
    } catch {
      populationDataset = null;
    }
  }

  async function inspectObservation(interpretationId: string): Promise<void> {
    applyAnalysisContext(await inspectArchiveObservation(interpretationId));
  }

  async function returnLatest(): Promise<void> {
    applyAnalysisContext(await returnToBranchTip());
  }

  async function continueFromObservation(
    interpretationId: string,
    label: string,
  ): Promise<void> {
    applyAnalysisContext(
      await createTimelineContinuation(interpretationId, label),
    );
  }

  async function renameBranch(
    branchId: string,
    label: string | null,
  ): Promise<void> {
    applyAnalysisContext(await setTimelineBranchLabel(branchId, label));
  }

  function acceptObservation(dataset: ReceiverDataset): void {
    receiverDataset = dataset;
    void Promise.all([
      getArchiveOverview(),
      getRecorderHealth(),
      getSetupState(),
    ]).then(([archive, health, setup]) => {
      archiveOverview = archive;
      recorderHealth = health;
      setupState = setup;
    });
    void refreshPopulationDataset();
  }

  function acceptRecorderUpdate(update: RecorderUpdate): void {
    recorderHealth = update.health;
    if (setupState) {
      setupState = {
        ...setupState,
        automatic_observer: update.health.observer,
      };
    }
    if (update.import_result) {
      receiverDataset = update.import_result.dataset;
      void Promise.all([getSetupState(), getArchiveOverview()]).then(
        ([setup, archive]) => {
          setupState = setup;
          archiveOverview = archive;
        },
      );
      void refreshPopulationDataset();
    }
  }

  function acceptSetupChange(setup: SetupState): void {
    setupState = setup;
    if (recorderHealth) {
      recorderHealth = {
        ...recorderHealth,
        observer: setup.automatic_observer,
      };
    }
  }

  function acceptCompatibilityUpdate(update: CompatibilityUpdate): void {
    if (setupState) {
      setupState = { ...setupState, compatibility: update.status };
    }
  }

  async function refreshDiagnostics(): Promise<void> {
    if (!desktopAvailable || diagnosticsBusy) return;
    diagnosticsBusy = true;
    diagnosticsError = "";
    try {
      diagnosticLog = await getDiagnosticLog();
    } catch {
      diagnosticsError = $translation("diagnostics-read-failed");
    } finally {
      diagnosticsBusy = false;
    }
  }

  function openDiagnostics(): void {
    diagnosticsDialogOpen = true;
    void refreshDiagnostics();
  }

  async function clearDiagnostics(): Promise<void> {
    if (
      !desktopAvailable ||
      diagnosticsBusy ||
      !window.confirm($translation("diagnostics-clear-confirm"))
    )
      return;
    diagnosticsBusy = true;
    diagnosticsError = "";
    try {
      diagnosticLog = await clearDiagnosticLog();
    } catch {
      diagnosticsError = $translation("diagnostics-clear-failed");
    } finally {
      diagnosticsBusy = false;
    }
  }

  onMount(() => {
    void initialiseThemes()
      .then((status) => {
        if (status?.fallback_applied) {
          notify({
            title: $translation("theme-fallback-title"),
            message: $translation("theme-fallback-message"),
            tone: "warning",
          });
        }
      })
      .catch(() => {
        notify({
          title: $translation("theme-fallback-title"),
          message: $translation("theme-storage-unavailable"),
          tone: "error",
        });
      });
    if (!desktopAvailable) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    let stopCatalogueListening: (() => void) | undefined;
    let stopCompatibilityListening: (() => void) | undefined;
    let stopReinterpretationListening: (() => void) | undefined;
    let stopResearchBuildListening: (() => void) | undefined;
    let stopWarehouseListening: (() => void) | undefined;
    void Promise.all([
      getSetupState(),
      getLatestReceiverDataset(),
      getArchiveOverview(),
      getRecorderHealth(),
    ]).then(([setup, dataset, archive, health]) => {
      if (disposed) return;
      setupState = setup;
      receiverDataset = dataset;
      archiveOverview = archive;
      recorderHealth = health;
    });
    void refreshPopulationDataset();
    void listenForRecorderUpdates(acceptRecorderUpdate).then((unlisten) => {
      if (disposed) unlisten();
      else stopListening = unlisten;
    });
    void listenForCompatibilityUpdates(acceptCompatibilityUpdate).then(
      (unlisten) => {
        if (disposed) unlisten();
        else stopCompatibilityListening = unlisten;
      },
    );
    void listenForWarehouseUpdates((status) => {
      if (!disposed) warehouseStatus = status;
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stopWarehouseListening = unlisten;
    });
    void observeLatestTaskProgress(
      {
        listen: listenForCatalogueProgress,
        read: async () => {
          const status = await getCatalogueStatus();
          warehouseStatus = status;
          return status.refresh;
        },
      },
      (progress) => {
        if (!disposed) catalogueProgress = progress;
      },
    ).then((unlisten) => {
      if (disposed) unlisten();
      else stopCatalogueListening = unlisten;
    });
    void observeLatestTaskProgress(
      {
        listen: listenForReinterpretationProgress,
        read: getReinterpretationProgress,
      },
      (progress) => {
        if (!disposed) reinterpretationProgress = progress;
      },
    ).then((unlisten) => {
      if (disposed) unlisten();
      else stopReinterpretationListening = unlisten;
    });
    void observeLatestTaskProgress(
      {
        listen: listenForResearchBuildProgress,
        read: getResearchBuildProgress,
      },
      (progress) => {
        if (!disposed) researchBuildProgress = progress;
      },
    ).then((unlisten) => {
      if (disposed) unlisten();
      else stopResearchBuildListening = unlisten;
    });
    return () => {
      disposed = true;
      stopListening?.();
      stopCatalogueListening?.();
      stopCompatibilityListening?.();
      stopReinterpretationListening?.();
      stopResearchBuildListening?.();
      stopWarehouseListening?.();
    };
  });

  function scannerHeading(): string {
    const phase = setupState?.automatic_observer.phase;
    if (phase === "waiting_for_stability")
      return $translation("scanner-waiting");
    if (phase === "retrying") return $translation("scanner-retrying");
    if (phase === "failed") return $translation("scanner-attention");
    if (setupState?.automatic_observer.enabled)
      return $translation("scanner-watching");
    return receiverDataset
      ? $translation("scanner-observed")
      : $translation("scanner-ready");
  }

  function scannerDetail(): string {
    const observer = setupState?.automatic_observer;
    if (
      observer?.phase === "waiting_for_stability" ||
      observer?.phase === "retrying" ||
      observer?.phase === "failed"
    ) {
      return (
        observer.candidate_file_name ?? $translation("scanner-no-candidate")
      );
    }
    if (receiverDataset)
      return $translation("scanner-observed-file", {
        file: receiverDataset.source_file_name,
      });
    return $translation("observer-save-candidates", {
      count: setupState?.save_candidates ?? 0,
    });
  }

  function catalogueProgressDetail(progress: CatalogueRefreshProgress): string {
    if (progress.phase === "failed")
      return $translation("catalogue-global-attention");
    if (progress.progress_percent != null)
      return `${progress.progress_percent}%`;
    if (progress.files_discovered > 0)
      return $translation("catalogue-global-files-found", {
        count: progress.files_discovered,
      });
    return $translation("task-progress-working");
  }
</script>

<svelte:head>
  <title>{$translation("app-document-title")}</title>
</svelte:head>

<main class="shell">
  <header class="command-bar">
    <div class="brand-lockup">
      <div class="brand-mark" aria-hidden="true"><span>R</span><i>O</i></div>
      <div>
        <span class="eyebrow">{$translation("brand-ministry")}</span>
        <h1>{$translation("brand-name")}</h1>
      </div>
    </div>

    <nav aria-label={$translation("nav-primary")}>
      {#each workspaces as workspace}
        <button
          type="button"
          class:active={workspace.id === activeWorkspace}
          disabled={!workspace.enabled}
          aria-current={workspace.id === activeWorkspace ? "page" : undefined}
          onclick={() => {
            if (workspace.enabled)
              activeWorkspace = workspace.id as WorkspaceName;
          }}
        >
          {$translation(workspace.label)}
        </button>
      {/each}
    </nav>

    <div class="command-actions">
      {#if warehouseStatus && (warehouseStatus.warehouse.pending_jobs > 0 || warehouseStatus.warehouse.failed_jobs > 0 || warehouseStatus.warehouse.active_write || warehouseStatus.warehouse.consecutive_write_failures > 0 || warehouseStatus.warehouse.phase === "rebuilding")}
        <TaskProgressIndicator
          label={$translation("catalogue-warehouse")}
          detail={warehouseProgressDetail(warehouseStatus)}
          percent={warehouseProgressPercent(
            warehouseStatus.warehouse.active_write,
          )}
          failed={warehouseStatus.warehouse.failed_jobs > 0 ||
            warehouseStatus.warehouse.consecutive_write_failures > 0}
          currentItem={warehouseStatus.warehouse.active_write
            ? warehouseActivityLabel(warehouseStatus.warehouse.active_write)
            : $translation("catalogue-recorder-independent")}
          onclick={() => (activeWorkspace = "materials")}
        />
      {/if}
      {#if catalogueProgress && ["discovering", "scanning", "publishing", "finalising", "failed"].includes(catalogueProgress.phase)}
        <TaskProgressIndicator
          label={$translation("catalogue-global-progress")}
          detail={catalogueProgressDetail(catalogueProgress)}
          percent={catalogueProgress.progress_percent}
          failed={catalogueProgress.phase === "failed"}
          currentItem={catalogueProgress.current_file ??
            catalogueProgress.current_source}
          onclick={() => (activeWorkspace = "materials")}
        />
      {/if}
      {#if reinterpretationProgress && ["reading", "parsing", "persisting", "queueing_warehouse", "failed"].includes(reinterpretationProgress.phase)}
        {@const view = reinterpretationProgressView(
          reinterpretationProgress,
          $translation,
        )}
        <TaskProgressIndicator
          label={$translation("reinterpretation-global-progress")}
          detail={view.heading}
          percent={view.progressPercent}
          failed={view.state === "failed"}
          currentItem={view.currentItem}
          onclick={() => (observationDialogOpen = true)}
        />
      {/if}
      {#if researchBuildProgress && ["running", "failed"].includes(researchBuildProgress.state)}
        {@const view = researchBuildProgressView(
          researchBuildProgress,
          $translation,
        )}
        <TaskProgressIndicator
          label={$translation("research-setup-progress-eyebrow")}
          detail={view.heading}
          percent={view.progressPercent}
          failed={view.state === "failed"}
          currentItem={view.currentItem}
          onclick={() => (researchSetupDialogOpen = true)}
        />
      {/if}
      <button
        type="button"
        class="legal-button"
        onclick={() => (legalDialogOpen = true)}
      >
        {$translation("legal-open")}
      </button>
      <button
        type="button"
        class="diagnostics-button"
        disabled={!desktopAvailable}
        onclick={openDiagnostics}
      >
        {$translation("diagnostics-open")}
      </button>
      <button
        type="button"
        class="language-button"
        onclick={() => (languageDialogOpen = true)}
      >
        {$translation("language-open", { locale: $activeLocale })}
      </button>
      <button
        type="button"
        class="theme-button"
        disabled={!desktopAvailable}
        onclick={() => (themeDialogOpen = true)}
      >
        {$translation("theme-open")}
      </button>
      <button
        type="button"
        class="scanner-state"
        aria-label={$translation("scanner-status-label")}
        title={$translation("observer-open")}
        onclick={() => (observationDialogOpen = true)}
      >
        <span class="state-dot" aria-hidden="true"></span>
        <div>
          {#if receiverDataset}
            <strong>{scannerHeading()}</strong>
            <small>{scannerDetail()}</small>
          {:else if desktopAvailable && setupState?.save_directory}
            <strong>{scannerHeading()}</strong>
            <small>{scannerDetail()}</small>
          {:else if desktopAvailable}
            <strong>{$translation("scanner-setup-required")}</strong>
            <small>{$translation("synthetic-no-save-connected")}</small>
          {:else}
            <strong>{$translation("synthetic-preview-mode")}</strong>
            <small>{$translation("synthetic-no-save-connected")}</small>
          {/if}
        </div>
      </button>
    </div>
  </header>

  <section
    class="observation-bar"
    aria-label={$translation("observation-context-label")}
  >
    <div class="observation-copy">
      <span class="history-glyph" aria-hidden="true"></span>
      <strong
        >{$translation(
          receiverDataset ? "observation-real" : "synthetic-observation",
        )}</strong
      >
      <span
        >{$translation("observation-branch", {
          branch: activeBranchLabel(),
        })}</span
      >
      <span
        >{$translation("observation-game-date", {
          year: latestReceiverPoint?.year ?? "2004",
          day: latestReceiverPoint?.day ?? 230,
        })}</span
      >
    </div>
    <div class="observation-actions">
      <span
        >{$translation("saves-observed", {
          count: setupState?.observed_saves ?? 0,
        })}</span
      >
      <button
        type="button"
        disabled={!desktopAvailable || archiveOverview?.analysis_context.is_tip}
        onclick={() => void returnLatest()}
        >{$translation("return-latest")}</button
      >
    </div>
  </section>

  {#if activeWorkspace === "briefing"}
    <BriefingWorkspace />
  {:else if activeWorkspace === "monitor"}
    <MonitorWorkspace
      health={recorderHealth}
      archive={archiveOverview}
      {receiverDataset}
      {desktopAvailable}
      oncompare={compareArchiveObservations}
    />
  {:else if activeWorkspace === "broadcast"}
    <BroadcastWorkspace {receiverDataset} />
  {:else if activeWorkspace === "extensions"}
    <ExtensionsWorkspace
      {desktopAvailable}
      observationContext={receiverDataset
        ? `${receiverDataset.analysis_context_id ?? "unbound"}:${receiverDataset.interpretation_id}:${receiverDataset.branch_id}`
        : ""}
    />
  {:else if activeWorkspace === "materials"}
    <MaterialsWorkspace
      {desktopAvailable}
      gameConfigured={Boolean(setupState?.game_directory)}
    />
  {:else if activeWorkspace === "population"}
    <PopulationWorkspace
      dataset={populationDataset}
      {desktopAvailable}
      onopenresearch={() => (researchSetupDialogOpen = true)}
    />
  {:else}
    <ArchiveWorkspace
      archive={archiveOverview}
      {desktopAvailable}
      onselect={selectBranch}
      oninspect={inspectObservation}
      oncontinue={continueFromObservation}
      onrename={renameBranch}
      onreturn={returnLatest}
      oncompare={compareArchiveObservations}
    />
  {/if}

  <footer class="status-bar">
    <span>{$translation("footer-foundation")}</span>
    <span>{$translation("save-safety-footer-principles")}</span>
    <span>{$translation("legal-independent-community-project")}</span>
  </footer>
</main>

<NotificationCenter />

<LanguageDialog
  open={languageDialogOpen}
  onclose={() => (languageDialogOpen = false)}
/>

<ThemeDialog open={themeDialogOpen} onclose={() => (themeDialogOpen = false)} />

<ObservationDialog
  open={observationDialogOpen}
  {desktopAvailable}
  setup={setupState}
  dataset={receiverDataset}
  {reinterpretationProgress}
  onclose={() => (observationDialogOpen = false)}
  onsetupchange={acceptSetupChange}
  onobservation={acceptObservation}
/>

<DiagnosticsDialog
  open={diagnosticsDialogOpen}
  busy={diagnosticsBusy}
  log={diagnosticLog}
  errorMessage={diagnosticsError}
  onclose={() => (diagnosticsDialogOpen = false)}
  onrefresh={() => void refreshDiagnostics()}
  onclear={() => void clearDiagnostics()}
/>

<LegalDialog
  open={legalDialogOpen}
  onclose={() => (legalDialogOpen = false)}
  onopenresearch={() => (researchSetupDialogOpen = true)}
/>

<ResearchSetupDialog
  open={researchSetupDialogOpen}
  onclose={() => (researchSetupDialogOpen = false)}
  onopenlegal={() => (legalDialogOpen = true)}
  onopendiagnostics={openDiagnostics}
/>
