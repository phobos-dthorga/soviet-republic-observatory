<script lang="ts">
  import { translation } from "../i18n/runtime";
  import {
    workspaceLabelKey,
    type NavigationTrailEntry,
    type WorkspaceLocation,
  } from "./relatedData";

  let {
    trail,
    current,
    busy = false,
    onback,
    onjump,
  }: {
    trail: NavigationTrailEntry[];
    current: WorkspaceLocation;
    busy?: boolean;
    onback: () => void;
    onjump: (index: number) => void;
  } = $props();
</script>

{#if trail.length > 0}
  <nav
    class="related-breadcrumb"
    aria-label={$translation("related-nav-label")}
  >
    <button type="button" disabled={busy} onclick={onback}>
      <span aria-hidden="true">←</span>
      {$translation("related-nav-back")}
    </button>
    <ol>
      {#each trail as entry, index}
        <li>
          <button type="button" disabled={busy} onclick={() => onjump(index)}>
            {$translation(workspaceLabelKey(entry.location.workspace))}
          </button>
          <span aria-hidden="true">›</span>
        </li>
      {/each}
      <li aria-current="page">
        {$translation(workspaceLabelKey(current.workspace))}
      </li>
    </ol>
  </nav>
{/if}
