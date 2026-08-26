<script lang="ts">
  import BriefingWorkspace from "./lib/workspaces/BriefingWorkspace.svelte";
  import BroadcastWorkspace from "./lib/workspaces/BroadcastWorkspace.svelte";
  import ExtensionsWorkspace from "./lib/workspaces/ExtensionsWorkspace.svelte";

  type WorkspaceName = "Briefing" | "Broadcast" | "Extensions";

  const workspaces: Array<{ name: string; enabled: boolean }> = [
    { name: "Briefing", enabled: true },
    { name: "Broadcast", enabled: true },
    { name: "Extensions", enabled: true },
    { name: "Plan", enabled: false },
    { name: "Materials", enabled: false },
    { name: "Population", enabled: false },
    { name: "Markets", enabled: false },
    { name: "Archive", enabled: false },
  ];

  let activeWorkspace = $state<WorkspaceName>("Briefing");
</script>

<svelte:head>
  <title>Republic Observatory · Synthetic foundation</title>
</svelte:head>

<main class="shell">
  <header class="command-bar">
    <div class="brand-lockup">
      <div class="brand-mark" aria-hidden="true"><span>R</span><i>O</i></div>
      <div>
        <span class="eyebrow">Ministry of Planning</span>
        <h1>Republic Observatory</h1>
      </div>
    </div>

    <nav aria-label="Primary workspaces">
      {#each workspaces as workspace}
        <button
          type="button"
          class:active={workspace.name === activeWorkspace}
          disabled={!workspace.enabled}
          aria-current={workspace.name === activeWorkspace ? "page" : undefined}
          onclick={() => {
            if (workspace.enabled)
              activeWorkspace = workspace.name as WorkspaceName;
          }}
        >
          {workspace.name}
        </button>
      {/each}
    </nav>

    <div class="scanner-state" aria-label="Save observer status">
      <span class="state-dot" aria-hidden="true"></span>
      <div><strong>Preview mode</strong><small>No save connected</small></div>
    </div>
  </header>

  <section class="observation-bar" aria-label="Observation context">
    <div class="observation-copy">
      <span class="history-glyph" aria-hidden="true"></span>
      <strong>Synthetic observation</strong>
      <span>Branch: planning-preview</span>
      <span>Game date: 2004 · day 230</span>
    </div>
    <div class="observation-actions">
      <span>0 saves observed</span>
      <button type="button" disabled>Return to latest</button>
    </div>
  </section>

  {#if activeWorkspace === "Briefing"}
    <BriefingWorkspace />
  {:else if activeWorkspace === "Broadcast"}
    <BroadcastWorkspace />
  {:else}
    <ExtensionsWorkspace />
  {/if}

  <footer class="status-bar">
    <span>Republic Observatory · interface foundation</span>
    <span>Read-only design · branch-aware · local-first</span>
    <span>Independent community project</span>
  </footer>
</main>
