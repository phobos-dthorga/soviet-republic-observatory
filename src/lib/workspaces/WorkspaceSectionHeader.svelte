<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    eyebrow,
    title,
    description = "",
    level = "section",
    actions,
  }: {
    eyebrow: string;
    title: string;
    description?: string;
    level?: "page" | "section";
    actions?: Snippet;
  } = $props();
</script>

<header class="workspace-section-header">
  <div class="workspace-section-copy">
    <span class="eyebrow">{eyebrow}</span>
    {#if level === "page"}<h2>{title}</h2>{:else}<h3>{title}</h3>{/if}
    {#if description}<p>{description}</p>{/if}
  </div>
  {#if actions}
    <div class="workspace-section-actions">{@render actions()}</div>
  {/if}
</header>

<style>
  .workspace-section-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    max-width: 100%;
  }
  .workspace-section-copy {
    min-width: 0;
    max-width: 72ch;
  }
  h2,
  h3 {
    margin-top: 0.3rem;
  }
  h2 {
    font-size: 1.75rem;
  }
  h3 {
    font-size: 1.35rem;
  }
  p {
    margin-top: 0.45rem;
    color: var(--colour-muted);
    line-height: 1.55;
  }
  .workspace-section-actions {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  @media (max-width: 720px) {
    .workspace-section-header {
      display: grid;
    }
    .workspace-section-actions {
      justify-content: flex-start;
    }
  }
</style>
