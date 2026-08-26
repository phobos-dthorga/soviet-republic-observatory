<script lang="ts">
  import { onMount } from "svelte";
  import BriefingWorkspace from "./lib/workspaces/BriefingWorkspace.svelte";
  import BroadcastWorkspace from "./lib/workspaces/BroadcastWorkspace.svelte";
  import ExtensionsWorkspace from "./lib/workspaces/ExtensionsWorkspace.svelte";
  import LanguageDialog from "./lib/i18n/LanguageDialog.svelte";
  import { activeLocale, translation } from "./lib/i18n/runtime";
  import type { TranslationKey } from "./lib/i18n/catalog";
  import ObservationDialog from "./lib/observations/ObservationDialog.svelte";
  import {
    desktopHostAvailable,
    getLatestReceiverDataset,
    getSetupState,
  } from "./lib/observations/desktopClient";
  import type { ReceiverDataset, SetupState } from "./lib/observations/types";

  type WorkspaceName = "briefing" | "broadcast" | "extensions";
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
    { id: "broadcast", label: "nav-broadcast", enabled: true },
    { id: "extensions", label: "nav-extensions", enabled: true },
    { id: "plan", label: "nav-plan", enabled: false },
    { id: "materials", label: "nav-materials", enabled: false },
    { id: "population", label: "nav-population", enabled: false },
    { id: "markets", label: "nav-markets", enabled: false },
    { id: "archive", label: "nav-archive", enabled: false },
  ];

  let activeWorkspace = $state<WorkspaceName>("briefing");
  let languageDialogOpen = $state(false);
  let observationDialogOpen = $state(false);
  const desktopAvailable = desktopHostAvailable();
  let setupState = $state<SetupState | null>(null);
  let receiverDataset = $state<ReceiverDataset | null>(null);
  const latestReceiverPoint = $derived(receiverDataset?.points.at(-1));

  onMount(() => {
    if (!desktopAvailable) return;
    void Promise.all([getSetupState(), getLatestReceiverDataset()]).then(
      ([setup, dataset]) => {
        setupState = setup;
        receiverDataset = dataset;
      },
    );
  });
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
            <strong>{$translation("scanner-observed")}</strong>
            <small
              >{$translation("scanner-observed-file", {
                file: receiverDataset.source_file_name,
              })}</small
            >
          {:else if desktopAvailable && setupState?.save_directory}
            <strong>{$translation("scanner-ready")}</strong>
            <small
              >{$translation("observer-save-candidates", {
                count: setupState.save_candidates,
              })}</small
            >
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
          branch: receiverDataset
            ? $translation("observation-branch-unassigned")
            : "planning-preview",
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
  {:else if activeWorkspace === "broadcast"}
    <BroadcastWorkspace {receiverDataset} />
  {:else}
    <ExtensionsWorkspace />
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
  onsetupchange={(setup) => (setupState = setup)}
  onobservation={(dataset) => (receiverDataset = dataset)}
/>
