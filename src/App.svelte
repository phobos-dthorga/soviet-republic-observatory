<script lang="ts">
  import { onMount } from "svelte";
  import BriefingWorkspace from "./lib/workspaces/BriefingWorkspace.svelte";
  import BroadcastWorkspace from "./lib/workspaces/BroadcastWorkspace.svelte";
  import ExtensionsWorkspace from "./lib/workspaces/ExtensionsWorkspace.svelte";
  import ArchiveWorkspace from "./lib/workspaces/ArchiveWorkspace.svelte";
  import MonitorWorkspace from "./lib/workspaces/MonitorWorkspace.svelte";
  import MaterialsWorkspace from "./lib/workspaces/MaterialsWorkspace.svelte";
  import LanguageDialog from "./lib/i18n/LanguageDialog.svelte";
  import { activeLocale, translation } from "./lib/i18n/runtime";
  import type { TranslationKey } from "./lib/i18n/catalog";
  import ObservationDialog from "./lib/observations/ObservationDialog.svelte";
  import {
    compareArchiveObservations,
    desktopHostAvailable,
    getArchiveOverview,
    getLatestReceiverDataset,
    getRecorderHealth,
    getSetupState,
    listenForRecorderUpdates,
    selectTimelineBranch,
  } from "./lib/observations/desktopClient";
  import type {
    ArchiveOverview,
    ReceiverDataset,
    RecorderHealth,
    RecorderUpdate,
    SetupState,
  } from "./lib/observations/types";

  type WorkspaceName =
    | "briefing"
    | "monitor"
    | "broadcast"
    | "extensions"
    | "materials"
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
    { id: "population", label: "nav-population", enabled: false },
    { id: "markets", label: "nav-markets", enabled: false },
    { id: "archive", label: "nav-archive", enabled: true },
  ];

  let activeWorkspace = $state<WorkspaceName>("briefing");
  let languageDialogOpen = $state(false);
  let observationDialogOpen = $state(false);
  const desktopAvailable = desktopHostAvailable();
  let setupState = $state<SetupState | null>(null);
  let receiverDataset = $state<ReceiverDataset | null>(null);
  let archiveOverview = $state<ArchiveOverview | null>(null);
  let recorderHealth = $state<RecorderHealth | null>(null);
  const latestReceiverPoint = $derived(receiverDataset?.points.at(-1));

  function activeBranchLabel(): string {
    if (!receiverDataset) return "planning-preview";
    if (receiverDataset.branch_id === "main")
      return $translation("archive-branch-main");
    if (receiverDataset.branch_id === "unassigned")
      return $translation("archive-branch-unassigned");
    return $translation("archive-branch-fork", {
      identity: receiverDataset.branch_id.replace("fork-", "").slice(0, 6),
    });
  }

  async function selectBranch(branchId: string): Promise<void> {
    const result = await selectTimelineBranch(branchId);
    archiveOverview = result.archive;
    receiverDataset = result.dataset;
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

  onMount(() => {
    if (!desktopAvailable) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
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
    void listenForRecorderUpdates(acceptRecorderUpdate).then((unlisten) => {
      if (disposed) unlisten();
      else stopListening = unlisten;
    });
    return () => {
      disposed = true;
      stopListening?.();
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
      <button
        type="button"
        class="language-button"
        onclick={() => (languageDialogOpen = true)}
      >
        {$translation("language-open", { locale: $activeLocale })}
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
      <button type="button" disabled>{$translation("return-latest")}</button>
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
    <ExtensionsWorkspace />
  {:else if activeWorkspace === "materials"}
    <MaterialsWorkspace
      {desktopAvailable}
      gameConfigured={Boolean(setupState?.game_directory)}
    />
  {:else}
    <ArchiveWorkspace
      archive={archiveOverview}
      {desktopAvailable}
      onselect={selectBranch}
      oncompare={compareArchiveObservations}
    />
  {/if}

  <footer class="status-bar">
    <span>{$translation("footer-foundation")}</span>
    <span>{$translation("save-safety-footer-principles")}</span>
    <span>{$translation("legal-independent-community-project")}</span>
  </footer>
</main>

<LanguageDialog
  open={languageDialogOpen}
  onclose={() => (languageDialogOpen = false)}
/>

<ObservationDialog
  open={observationDialogOpen}
  {desktopAvailable}
  setup={setupState}
  dataset={receiverDataset}
  onclose={() => (observationDialogOpen = false)}
  onsetupchange={acceptSetupChange}
  onobservation={acceptObservation}
/>
