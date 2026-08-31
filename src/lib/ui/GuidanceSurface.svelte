<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    kind,
    layout = "block",
    semanticRole,
    class: className = "",
    children,
  }: {
    kind: "boundary" | "help" | "instruction" | "preview";
    layout?: "block" | "compact" | "inline";
    semanticRole?: "note" | "status";
    class?: string;
    children: Snippet;
  } = $props();
</script>

<div
  class={`guidance-surface ${className}`.trim()}
  data-guidance-surface={kind}
  data-guidance-layout={layout}
  role={semanticRole}
>
  {@render children()}
</div>
