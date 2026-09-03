<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    label,
    children,
  }: {
    label: string;
    children: Snippet;
  } = $props();
</script>

<div class="workspace-toolbar" role="toolbar" aria-label={label}>
  {@render children()}
</div>

<style>
  .workspace-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.55rem;
  }
</style>
