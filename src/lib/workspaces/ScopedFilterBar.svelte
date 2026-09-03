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

<div
  class="scoped-filter-bar"
  role="group"
  aria-label={label}
  data-scoped-filter-bar
>
  {@render children()}
</div>

<style>
  .scoped-filter-bar {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(13rem, 20rem));
    align-items: end;
    gap: 0.75rem;
    margin-block: 0.9rem;
    padding: 0.75rem;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
  }
  :global(.scoped-filter-bar label) {
    display: grid;
    gap: 0.35rem;
    min-width: 0;
  }
  :global(.scoped-filter-bar :is(input, select)) {
    width: 100%;
    min-width: 0;
  }
</style>
