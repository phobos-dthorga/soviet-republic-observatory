<script lang="ts">
  import BriefingWorkspace from "./lib/workspaces/BriefingWorkspace.svelte";
  import BroadcastWorkspace from "./lib/workspaces/BroadcastWorkspace.svelte";
  import ExtensionsWorkspace from "./lib/workspaces/ExtensionsWorkspace.svelte";
  import LanguageDialog from "./lib/i18n/LanguageDialog.svelte";
  import { activeLocale, translation } from "./lib/i18n/runtime";
  import type { TranslationKey } from "./lib/i18n/catalog";

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
      <div
        class="scanner-state"
        aria-label={$translation("scanner-status-label")}
      >
        <span class="state-dot" aria-hidden="true"></span>
        <div>
          <strong>{$translation("synthetic-preview-mode")}</strong>
          <small>{$translation("synthetic-no-save-connected")}</small>
        </div>
      </div>
    </div>
  </header>

  <section
    class="observation-bar"
    aria-label={$translation("observation-context-label")}
  >
    <div class="observation-copy">
      <span class="history-glyph" aria-hidden="true"></span>
      <strong>{$translation("synthetic-observation")}</strong>
      <span
        >{$translation("observation-branch", {
          branch: "planning-preview",
        })}</span
      >
      <span
        >{$translation("observation-game-date", {
          year: "2004",
          day: 230,
        })}</span
      >
    </div>
    <div class="observation-actions">
      <span>{$translation("saves-observed", { count: 0 })}</span>
      <button type="button" disabled>{$translation("return-latest")}</button>
    </div>
  </section>

  {#if activeWorkspace === "briefing"}
    <BriefingWorkspace />
  {:else if activeWorkspace === "broadcast"}
    <BroadcastWorkspace />
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
