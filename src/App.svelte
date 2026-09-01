<script lang="ts">
  import { onMount, tick } from "svelte";
  import BriefingWorkspace from "./lib/workspaces/BriefingWorkspace.svelte";
  import BroadcastWorkspace from "./lib/workspaces/BroadcastWorkspace.svelte";
  import ExtensionsWorkspace from "./lib/workspaces/ExtensionsWorkspace.svelte";
  import ArchiveWorkspace from "./lib/workspaces/ArchiveWorkspace.svelte";
  import MonitorWorkspace from "./lib/workspaces/MonitorWorkspace.svelte";
  import MaterialsWorkspace from "./lib/workspaces/MaterialsWorkspace.svelte";
  import PopulationWorkspace from "./lib/workspaces/PopulationWorkspace.svelte";
  import PlanWorkspace from "./lib/workspaces/PlanWorkspace.svelte";
  import MarketsWorkspace from "./lib/workspaces/MarketsWorkspace.svelte";
  import LanguageDialog from "./lib/i18n/LanguageDialog.svelte";
  import ThemeDialog from "./lib/theme/ThemeDialog.svelte";
  import SettingsDialog from "./lib/settings/SettingsDialog.svelte";
  import { getApplicationSettings } from "./lib/settings/desktopClient";
  import { applyApplicationPreferences } from "./lib/settings/runtime";
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
  import {
    clearNotifications,
    dismissRecoveryProposal,
    notify,
    openRecoveryProposal,
    recoveryProposal,
  } from "./lib/notifications/service";
  import {
    dialogLayer,
    pushDialogRoute,
    removeDialogRoute,
    topDialogRoute,
    type DialogRoute,
  } from "./lib/navigation/dialogStack";
  import RelatedDataBreadcrumb from "./lib/navigation/RelatedDataBreadcrumb.svelte";
  import RelatedViewChooser from "./lib/navigation/RelatedViewChooser.svelte";
  import {
    defaultWorkspaceLocation,
    pushNavigationTrail,
    workspaceDestination,
    type AnalysisContextReference,
    type NavigationTrailEntry,
    type RelatedDataDestination,
    type WorkspaceLocation,
    type WorkspaceName,
  } from "./lib/navigation/relatedData";
  import { replayAttentionCue } from "./lib/attention/service";
  import { observeLatestTaskProgress } from "./lib/tasks/progress";
  import { reinterpretationProgressView } from "./lib/tasks/reinterpretationProgress";
  import { marketIndexingProgressView } from "./lib/presentation/markets";
  import {
    getResearchBuildProgress,
    listenForResearchBuildProgress,
  } from "./lib/research/desktopClient";
  import { researchBuildProgressView } from "./lib/research/progress";
  import type { ResearchBuildProgress } from "./lib/research/types";
  import {
    initialiseUiReview,
    type UiReviewScenarioRequest,
  } from "./lib/ui-review/runtime";
  import {
    reviewArchiveOverview,
    reviewCatalogueProgress,
    reviewMarketIndexingProgress,
    reviewMarketWorkspace,
    reviewPopulationDataset,
    reviewProductionPathway,
    reviewProductionRoute,
    reviewRepublicPlanWorkspace,
    reviewRepublicBrief,
    reviewWarehouseAttention,
  } from "./lib/ui-review/fixtures";
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
    getMarketIndexingProgress,
    getMarketWorkspace,
    getPublishedMetricContexts,
    getRepublicBrief,
    getRepublicPlanWorkspace,
    getReinterpretationProgress,
    getRecorderHealth,
    getSetupState,
    inspectArchiveObservation,
    listenForRecorderUpdates,
    listenForCatalogueProgress,
    listenForCompatibilityUpdates,
    listenForReinterpretationProgress,
    listenForMarketIndexingProgress,
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
    MarketIndexingProgress,
    MarketWorkspace,
    ReceiverDataset,
    PopulationDataset,
    PublishedMetricContext,
    ProductionPathwayModel,
    ProductionRouteModel,
    RecorderHealth,
    RecorderUpdate,
    RepublicBrief,
    RepublicPlanWorkspace,
    ReinterpretationProgress,
    SetupState,
    WarehouseWriteActivity,
  } from "./lib/observations/types";

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
    { id: "plan", label: "nav-plan", enabled: true },
    { id: "materials", label: "nav-materials", enabled: true },
    { id: "population", label: "nav-population", enabled: true },
    { id: "markets", label: "nav-markets", enabled: true },
    { id: "archive", label: "nav-archive", enabled: true },
  ];

  let activeLocation = $state<WorkspaceLocation>(
    defaultWorkspaceLocation("briefing"),
  );
  const activeWorkspace = $derived(activeLocation.workspace);
  let navigationTrail = $state<NavigationTrailEntry[]>([]);
  let navigationBusy = $state(false);
  let relatedChoices = $state<RelatedDataDestination[]>([]);
  let relatedChoiceOrigin = $state<HTMLElement | null>(null);
  let dialogStack = $state<DialogRoute[]>([]);
  const activeDialog = $derived(topDialogRoute(dialogStack));
  let diagnosticsBusy = $state(false);
  let diagnosticsError = $state("");
  let diagnosticLog = $state<DiagnosticLogView | null>(null);
  let catalogueProgress = $state<CatalogueRefreshProgress | null>(null);
  let warehouseStatus = $state<CatalogueStatus | null>(null);
  let reinterpretationProgress = $state<ReinterpretationProgress | null>(null);
  let researchBuildProgress = $state<ResearchBuildProgress | null>(null);
  let marketIndexingProgress = $state<MarketIndexingProgress | null>(null);
  const desktopAvailable = desktopHostAvailable();
  let setupState = $state<SetupState | null>(null);
  let receiverDataset = $state<ReceiverDataset | null>(null);
  let populationDataset = $state<PopulationDataset | null>(null);
  let archiveOverview = $state<ArchiveOverview | null>(null);
  let recorderHealth = $state<RecorderHealth | null>(null);
  let republicBrief = $state<RepublicBrief | null>(null);
  let republicPlan = $state<RepublicPlanWorkspace | null>(null);
  let marketWorkspace = $state<MarketWorkspace | null>(null);
  let publishedMetricContexts = $state<PublishedMetricContext[]>([]);
  let reviewRouteFixture = $state<ProductionRouteModel | null>(null);
  let reviewPathwayFixture = $state<ProductionPathwayModel | null>(null);
  const latestReceiverPoint = $derived(receiverDataset?.points.at(-1));

  function openDialog(route: DialogRoute): void {
    dialogStack = pushDialogRoute(dialogStack, route);
  }

  function currentAnalysisContext(): AnalysisContextReference | null {
    const context = archiveOverview?.analysis_context;
    if (!context) return null;
    return {
      branchId: context.selected_branch_id,
      headInterpretationId: context.head_interpretation_id,
      isTip: context.is_tip,
    };
  }

  function openWorkspace(workspace: WorkspaceName): void {
    navigationTrail = [];
    relatedChoices = [];
    activeLocation = defaultWorkspaceLocation(workspace);
  }

  function requestRelatedNavigation(
    destinations: RelatedDataDestination[],
    origin: HTMLElement | null = null,
  ): void {
    const unique = destinations.filter(
      (destination, index) =>
        destinations.findIndex((item) => item.id === destination.id) === index,
    );
    if (unique.length === 0) return;
    if (unique.length === 1) {
      void navigateRelated(unique[0]);
      return;
    }
    relatedChoiceOrigin =
      origin ?? (document.activeElement as HTMLElement | null);
    relatedChoices = unique;
  }

  function closeRelatedChoices(): void {
    relatedChoices = [];
    void tick().then(() => relatedChoiceOrigin?.focus());
  }

  async function restoreAnalysisContext(
    context: AnalysisContextReference | null,
  ): Promise<void> {
    if (!context || !desktopAvailable) return;
    const current = currentAnalysisContext();
    if (
      current?.branchId === context.branchId &&
      current.headInterpretationId === context.headInterpretationId &&
      current.isTip === context.isTip
    ) {
      return;
    }
    if (context.isTip) {
      await applyAnalysisContext(await selectTimelineBranch(context.branchId));
    } else if (context.headInterpretationId) {
      await applyAnalysisContext(
        await inspectArchiveObservation(context.headInterpretationId),
      );
    }
  }

  async function navigateRelated(
    destination: RelatedDataDestination,
  ): Promise<void> {
    if (navigationBusy) return;
    navigationBusy = true;
    const previous: NavigationTrailEntry = {
      location: structuredClone(activeLocation),
      context: currentAnalysisContext(),
    };
    try {
      if (destination.exactObservation && desktopAvailable) {
        await applyAnalysisContext(
          await inspectArchiveObservation(
            destination.exactObservation.interpretation_id,
          ),
        );
      }
      activeLocation = structuredClone(destination.location);
      navigationTrail = pushNavigationTrail(navigationTrail, previous);
      relatedChoices = [];
      await focusRelatedLocation(activeLocation);
    } catch {
      notify({
        title: $translation("related-nav-failed-title"),
        message: $translation("related-nav-failed-message"),
        tone: "error",
      });
    } finally {
      navigationBusy = false;
    }
  }

  async function returnThroughRelatedTrail(index?: number): Promise<void> {
    if (navigationBusy || navigationTrail.length === 0) return;
    const targetIndex = index ?? navigationTrail.length - 1;
    const target = navigationTrail[targetIndex];
    if (!target) return;
    navigationBusy = true;
    try {
      await restoreAnalysisContext(target.context);
      activeLocation = structuredClone(target.location);
      navigationTrail = navigationTrail.slice(0, targetIndex);
      await focusRelatedLocation(activeLocation);
    } catch {
      notify({
        title: $translation("related-nav-failed-title"),
        message: $translation("related-nav-failed-message"),
        tone: "error",
      });
    } finally {
      navigationBusy = false;
    }
  }

  async function focusRelatedLocation(
    location: WorkspaceLocation,
  ): Promise<void> {
    await tick();
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve()),
    );
    const target = document.getElementById(
      location.focusId ?? location.section,
    );
    const canvas = target?.closest<HTMLElement>(".workspace > .canvas");
    if (!target || !canvas) return;
    if (!target.hasAttribute("tabindex")) target.tabIndex = -1;
    target.focus({ preventScroll: true });
    const targetBox = target.getBoundingClientRect();
    const canvasBox = canvas.getBoundingClientRect();
    canvas.scrollTo({
      top: Math.max(0, canvas.scrollTop + targetBox.top - canvasBox.top - 8),
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "auto"
        : "smooth",
    });
    target.dataset.relatedArrival = "true";
    window.setTimeout(() => delete target.dataset.relatedArrival, 1800);
  }

  function closeDialog(route: DialogRoute): void {
    if (route === "recovery") {
      dismissRecoveryProposal();
      return;
    }
    dialogStack = removeDialogRoute(dialogStack, route);
  }

  function dialogOpen(route: DialogRoute): boolean {
    return dialogStack.includes(route);
  }

  function appShortcut(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing) return;
    if (
      event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      !event.shiftKey &&
      event.key === ","
    ) {
      event.preventDefault();
      openDialog("settings");
      return;
    }
    if (
      event.altKey &&
      !event.ctrlKey &&
      !event.metaKey &&
      event.key === "ArrowLeft" &&
      (activeDialog || navigationTrail.length > 0)
    ) {
      event.preventDefault();
      if (activeDialog) {
        document
          .querySelector<HTMLElement>('[data-dialog-active="true"] dialog')
          ?.dispatchEvent(
            new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
          );
      } else {
        void returnThroughRelatedTrail();
      }
    }
  }

  $effect(() => {
    const recoveryOpen = $recoveryProposal !== null;
    const recoveryInStack = dialogStack.includes("recovery");
    if (recoveryOpen && !recoveryInStack) openDialog("recovery");
    else if (!recoveryOpen && recoveryInStack)
      dialogStack = removeDialogRoute(dialogStack, "recovery");
  });

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
      case "market_projection":
        return $translation("nav-markets");
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
    const branchId =
      receiverDataset?.branch_id ??
      republicBrief?.analysis_context.selected_branch_id;
    if (!branchId) return $translation("observation-branch-unavailable");
    const selected = archiveOverview?.branches.find(
      (branch) => branch.selected,
    );
    if (selected?.player_label) return selected.player_label;
    if (branchId === "main") return $translation("archive-branch-main");
    if (branchId === "unassigned")
      return $translation("archive-branch-unassigned");
    return $translation(
      selected?.origin === "manual_continuation"
        ? "archive-branch-continuation"
        : "archive-branch-fork",
      {
        identity: selected?.short_identity ?? branchId.slice(0, 12),
      },
    );
  }

  async function selectBranch(branchId: string): Promise<void> {
    const result = await selectTimelineBranch(branchId);
    await applyAnalysisContext(result);
  }

  async function applyAnalysisContext(result: {
    archive: ArchiveOverview;
    dataset: ReceiverDataset | null;
  }): Promise<void> {
    archiveOverview = result.archive;
    receiverDataset = result.dataset;
    await Promise.all([
      refreshPopulationDataset(),
      refreshRepublicBrief(),
      refreshRepublicPlan(),
      refreshMarketWorkspace(),
    ]);
  }

  async function refreshPopulationDataset(): Promise<void> {
    if (!desktopAvailable) return;
    try {
      populationDataset = await getPopulationDataset();
    } catch {
      populationDataset = null;
    }
  }

  async function refreshRepublicBrief(): Promise<void> {
    if (!desktopAvailable) return;
    try {
      republicBrief = await getRepublicBrief();
    } catch {
      republicBrief = null;
    }
  }

  async function refreshRepublicPlan(): Promise<void> {
    if (!desktopAvailable) return;
    try {
      republicPlan = await getRepublicPlanWorkspace();
    } catch {
      republicPlan = null;
    }
  }

  async function refreshMarketWorkspace(): Promise<void> {
    if (!desktopAvailable) return;
    try {
      marketWorkspace = await getMarketWorkspace();
    } catch {
      marketWorkspace = null;
    }
  }

  async function inspectObservation(interpretationId: string): Promise<void> {
    await applyAnalysisContext(
      await inspectArchiveObservation(interpretationId),
    );
  }

  async function returnLatest(): Promise<void> {
    await applyAnalysisContext(await returnToBranchTip());
  }

  async function continueFromObservation(
    interpretationId: string,
    label: string,
  ): Promise<void> {
    await applyAnalysisContext(
      await createTimelineContinuation(interpretationId, label),
    );
  }

  async function renameBranch(
    branchId: string,
    label: string | null,
  ): Promise<void> {
    await applyAnalysisContext(await setTimelineBranchLabel(branchId, label));
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
    void refreshRepublicBrief();
    void refreshRepublicPlan();
    void refreshMarketWorkspace();
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
      void refreshRepublicBrief();
      void refreshRepublicPlan();
      void refreshMarketWorkspace();
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
    openDialog("diagnostics");
    void refreshDiagnostics();
  }

  async function applyUiReviewScenario(
    request: UiReviewScenarioRequest,
  ): Promise<void> {
    clearNotifications();
    dialogStack = [];
    catalogueProgress = null;
    warehouseStatus = null;
    reinterpretationProgress = null;
    researchBuildProgress = null;
    marketIndexingProgress = null;
    marketWorkspace = null;
    reviewRouteFixture = null;
    reviewPathwayFixture = null;

    switch (request.scenario) {
      case "workspace-briefing":
        openWorkspace("briefing");
        republicBrief = reviewRepublicBrief();
        break;
      case "workspace-monitor":
        openWorkspace("monitor");
        break;
      case "workspace-broadcast":
        openWorkspace("broadcast");
        break;
      case "workspace-extensions":
        openWorkspace("extensions");
        break;
      case "workspace-plan":
        openWorkspace("plan");
        republicPlan = reviewRepublicPlanWorkspace();
        break;
      case "workspace-materials":
        openWorkspace("materials");
        break;
      case "materials-warehouse-attention":
        openWorkspace("materials");
        warehouseStatus = reviewWarehouseAttention();
        break;
      case "production-pathway":
        openWorkspace("materials");
        reviewRouteFixture = reviewProductionRoute();
        reviewPathwayFixture = reviewProductionPathway();
        break;
      case "workspace-population":
      case "population-probe-missing":
        openWorkspace("population");
        populationDataset = reviewPopulationDataset();
        break;
      case "workspace-markets":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("ready");
        break;
      case "markets-indexing":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("ready");
        marketIndexingProgress = reviewMarketIndexingProgress(false);
        break;
      case "markets-paused":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("ready");
        marketIndexingProgress = {
          ...reviewMarketIndexingProgress(false),
          phase: "paused",
          error_code: "storage_occupied",
        };
        break;
      case "markets-partial":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("partial");
        break;
      case "markets-empty":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("empty");
        break;
      case "markets-lagging":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("lagging");
        break;
      case "markets-failed":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("partial");
        marketIndexingProgress = reviewMarketIndexingProgress(true);
        break;
      case "archive-latest":
        openWorkspace("archive");
        archiveOverview = reviewArchiveOverview(false);
        break;
      case "archive-historical":
        openWorkspace("archive");
        archiveOverview = reviewArchiveOverview(true);
        break;
      case "critical-task-loading":
        openWorkspace("materials");
        catalogueProgress = reviewCatalogueProgress(false);
        break;
      case "critical-task-failed":
        openWorkspace("materials");
        catalogueProgress = reviewCatalogueProgress(true);
        break;
      case "dialog-language":
        openDialog("language");
        break;
      case "dialog-theme":
        openDialog("theme");
        break;
      case "dialog-settings":
        openDialog("settings");
        break;
      case "dialog-observation":
        openDialog("observation");
        break;
      case "dialog-diagnostics":
        openDialog("diagnostics");
        break;
      case "dialog-legal":
        openDialog("legal");
        break;
      case "dialog-research":
        openDialog("research");
        break;
      case "dialog-recovery":
        openWorkspace("markets");
        marketWorkspace = reviewMarketWorkspace("partial");
        openRecoveryProposal({
          title: $translation("markets-recovery-title"),
          message: $translation("markets-recovery-contract-message"),
          consequence: $translation("recovery-retained-evidence-safety"),
          actionLabel: $translation("markets-recovery-repair-action"),
          run: () => undefined,
        });
        break;
      case "notification-error":
        openWorkspace("briefing");
        republicBrief = reviewRepublicBrief();
        notify({
          title: $translation("diagnostics-title"),
          message: $translation("theme-storage-unavailable"),
          tone: "error",
        });
        break;
      case "tooltip-contextual":
        openWorkspace("briefing");
        republicBrief = reviewRepublicBrief();
        break;
      case "attention-cue":
        openWorkspace("population");
        populationDataset = reviewPopulationDataset();
        await replayAttentionCue("research.setup.entry", 1);
        break;
      case "keyboard-focus":
        openWorkspace("briefing");
        republicBrief = reviewRepublicBrief();
        break;
      case "native-dropdown":
        openWorkspace("population");
        populationDataset = reviewPopulationDataset();
        break;
    }
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
    window.addEventListener("keydown", appShortcut);
    let stopUiReview: (() => void) | undefined;
    const themeReady = initialiseThemes()
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
    if (!desktopAvailable) {
      void themeReady.then(() =>
        initialiseUiReview(applyUiReviewScenario).then((dispose) => {
          stopUiReview = dispose;
        }),
      );
      return () => {
        window.removeEventListener("keydown", appShortcut);
        stopUiReview?.();
      };
    }
    let disposed = false;
    let stopListening: (() => void) | undefined;
    let stopCatalogueListening: (() => void) | undefined;
    let stopCompatibilityListening: (() => void) | undefined;
    let stopReinterpretationListening: (() => void) | undefined;
    let stopMarketIndexingListening: (() => void) | undefined;
    let stopResearchBuildListening: (() => void) | undefined;
    let stopWarehouseListening: (() => void) | undefined;
    const initialDataReady = Promise.all([
      getApplicationSettings(),
      getLatestReceiverDataset(),
      getArchiveOverview(),
      getRecorderHealth(),
      getPopulationDataset().catch(() => null),
      getRepublicBrief().catch(() => null),
      getRepublicPlanWorkspace().catch(() => null),
      getMarketWorkspace().catch(() => null),
      getPublishedMetricContexts().catch(() => []),
    ]).then(
      ([
        settings,
        dataset,
        archive,
        health,
        population,
        brief,
        plan,
        markets,
        metricContexts,
      ]) => {
        if (disposed) return;
        setupState = settings.setup;
        applyApplicationPreferences(settings.preferences);
        receiverDataset = dataset;
        archiveOverview = archive;
        recorderHealth = health;
        populationDataset = population;
        republicBrief = brief;
        republicPlan = plan;
        marketWorkspace = markets;
        publishedMetricContexts = metricContexts;
      },
    );
    void Promise.allSettled([themeReady, initialDataReady]).then(() => {
      if (disposed) return;
      void initialiseUiReview(applyUiReviewScenario).then((dispose) => {
        if (disposed) dispose();
        else stopUiReview = dispose;
      });
    });
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
      if (!disposed) {
        warehouseStatus = status;
        void refreshRepublicBrief();
        void refreshRepublicPlan();
        void refreshMarketWorkspace();
      }
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
    void observeLatestTaskProgress(
      {
        listen: listenForMarketIndexingProgress,
        read: getMarketIndexingProgress,
      },
      (progress) => {
        if (!disposed) {
          marketIndexingProgress = progress;
          if (progress.phase === "complete") void refreshMarketWorkspace();
        }
      },
    ).then((unlisten) => {
      if (disposed) unlisten();
      else stopMarketIndexingListening = unlisten;
    });
    return () => {
      disposed = true;
      stopListening?.();
      stopCatalogueListening?.();
      stopCompatibilityListening?.();
      stopReinterpretationListening?.();
      stopMarketIndexingListening?.();
      stopResearchBuildListening?.();
      stopWarehouseListening?.();
      stopUiReview?.();
      window.removeEventListener("keydown", appShortcut);
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
            if (workspace.enabled) openWorkspace(workspace.id as WorkspaceName);
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
          onclick={() => openWorkspace("materials")}
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
          onclick={() => openWorkspace("materials")}
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
          onclick={() => openDialog("observation")}
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
          onclick={() => openDialog("research")}
        />
      {/if}
      {#if marketIndexingProgress && ["discovering", "matching", "reading_archive", "parsing_records", "persisting", "queueing_warehouse", "failed"].includes(marketIndexingProgress.phase)}
        {@const view = marketIndexingProgressView(
          marketIndexingProgress,
          $translation,
        )}
        <TaskProgressIndicator
          label={$translation("markets-index-eyebrow")}
          detail={view.heading}
          percent={view.progressPercent}
          failed={view.state === "failed"}
          currentItem={view.currentItem}
          onclick={() => openWorkspace("markets")}
        />
      {/if}
      <button
        type="button"
        class="legal-button"
        onclick={() => openDialog("legal")}
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
        class="settings-button"
        onclick={() => openDialog("settings")}
      >
        {$translation("settings-open")}
      </button>
      <button
        type="button"
        class="scanner-state"
        aria-label={$translation("scanner-status-label")}
        title={$translation("observer-open")}
        onclick={() => openDialog("observation")}
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
            <strong>{$translation("browser-interface-mode")}</strong>
            <small>{$translation("browser-native-evidence-unavailable")}</small>
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
          receiverDataset || republicBrief?.observation
            ? "observation-real"
            : "observation-unavailable",
        )}</strong
      >
      <span
        >{$translation("observation-branch", {
          branch: activeBranchLabel(),
        })}</span
      >
      <span
        >{$translation("observation-game-date", {
          year:
            republicBrief?.observation?.year ??
            latestReceiverPoint?.year ??
            "—",
          day:
            republicBrief?.observation?.day ?? latestReceiverPoint?.day ?? "—",
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

  <RelatedDataBreadcrumb
    trail={navigationTrail}
    current={activeLocation}
    busy={navigationBusy}
    onback={() => void returnThroughRelatedTrail()}
    onjump={(index) => void returnThroughRelatedTrail(index)}
  />

  {#if activeWorkspace === "briefing"}
    <BriefingWorkspace
      brief={republicBrief}
      onopenworkspace={(workspace) =>
        requestRelatedNavigation([workspaceDestination(workspace)])}
    />
  {:else if activeWorkspace === "monitor"}
    <MonitorWorkspace
      health={recorderHealth}
      archive={archiveOverview}
      {receiverDataset}
      {desktopAvailable}
      metricContexts={publishedMetricContexts}
      oncompare={compareArchiveObservations}
    />
  {:else if activeWorkspace === "broadcast"}
    <BroadcastWorkspace
      {receiverDataset}
      metricContexts={publishedMetricContexts}
      onrelatednavigate={requestRelatedNavigation}
    />
  {:else if activeWorkspace === "extensions"}
    <ExtensionsWorkspace
      {desktopAvailable}
      observationContext={receiverDataset
        ? `${receiverDataset.analysis_context_id ?? "unbound"}:${receiverDataset.interpretation_id}:${receiverDataset.branch_id}`
        : ""}
    />
  {:else if activeWorkspace === "plan"}
    <PlanWorkspace
      workspace={republicPlan}
      {desktopAvailable}
      onupdate={(updated) => {
        republicPlan = updated;
        void refreshRepublicBrief();
      }}
      onrelatednavigate={requestRelatedNavigation}
    />
  {:else if activeWorkspace === "materials"}
    <MaterialsWorkspace
      {desktopAvailable}
      gameConfigured={Boolean(setupState?.game_directory)}
      reviewRoute={reviewRouteFixture}
      reviewPathway={reviewPathwayFixture}
    />
  {:else if activeWorkspace === "population"}
    <PopulationWorkspace
      dataset={populationDataset}
      metricContexts={publishedMetricContexts}
      {desktopAvailable}
      onopenresearch={() => openDialog("research")}
    />
  {:else if activeWorkspace === "markets"}
    <MarketsWorkspace
      workspace={marketWorkspace}
      indexingProgress={marketIndexingProgress}
      {desktopAvailable}
      onupdate={(updated) => (marketWorkspace = updated)}
      onprogress={(progress) => {
        marketIndexingProgress = progress;
        if (progress.phase === "complete") void refreshMarketWorkspace();
      }}
      onrelatednavigate={requestRelatedNavigation}
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

{#if relatedChoices.length > 0}
  <RelatedViewChooser
    destinations={relatedChoices}
    onchoose={(destination) => void navigateRelated(destination)}
    onclose={closeRelatedChoices}
  />
{/if}

<NotificationCenter
  recoveryActive={activeDialog === "recovery"}
  recoveryLayer={dialogLayer(dialogStack, "recovery")}
/>

<LanguageDialog
  open={dialogOpen("language")}
  active={activeDialog === "language"}
  layer={dialogLayer(dialogStack, "language")}
  onclose={() => closeDialog("language")}
/>

<ThemeDialog
  open={dialogOpen("theme")}
  active={activeDialog === "theme"}
  layer={dialogLayer(dialogStack, "theme")}
  onclose={() => closeDialog("theme")}
/>

<SettingsDialog
  open={dialogOpen("settings")}
  active={activeDialog === "settings"}
  layer={dialogLayer(dialogStack, "settings")}
  setup={setupState}
  onclose={() => closeDialog("settings")}
  onsetupchange={acceptSetupChange}
  onopenlanguage={() => openDialog("language")}
  onopentheme={() => openDialog("theme")}
  onopenobserver={() => openDialog("observation")}
  onopenlegal={() => openDialog("legal")}
  onopendiagnostics={openDiagnostics}
/>

<ObservationDialog
  open={dialogOpen("observation")}
  active={activeDialog === "observation"}
  layer={dialogLayer(dialogStack, "observation")}
  {desktopAvailable}
  setup={setupState}
  dataset={receiverDataset}
  {reinterpretationProgress}
  onclose={() => closeDialog("observation")}
  onsetupchange={acceptSetupChange}
  onobservation={acceptObservation}
  onopensettings={() => openDialog("settings")}
/>

<DiagnosticsDialog
  open={dialogOpen("diagnostics")}
  active={activeDialog === "diagnostics"}
  layer={dialogLayer(dialogStack, "diagnostics")}
  busy={diagnosticsBusy}
  log={diagnosticLog}
  errorMessage={diagnosticsError}
  onclose={() => closeDialog("diagnostics")}
  onrefresh={() => void refreshDiagnostics()}
  onclear={() => void clearDiagnostics()}
/>

<LegalDialog
  open={dialogOpen("legal")}
  active={activeDialog === "legal"}
  layer={dialogLayer(dialogStack, "legal")}
  onclose={() => closeDialog("legal")}
  onopenresearch={() => openDialog("research")}
/>

<ResearchSetupDialog
  open={dialogOpen("research")}
  active={activeDialog === "research"}
  layer={dialogLayer(dialogStack, "research")}
  onclose={() => closeDialog("research")}
  onopenlegal={() => openDialog("legal")}
  onopendiagnostics={openDiagnostics}
/>
