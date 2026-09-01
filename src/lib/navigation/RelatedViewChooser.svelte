<script lang="ts">
  import { onMount } from "svelte";
  import { translation } from "../i18n/runtime";
  import type { RelatedDataDestination } from "./relatedData";
  import { relationshipLabelKey } from "./relatedData";

  let {
    destinations,
    onchoose,
    onclose,
  }: {
    destinations: RelatedDataDestination[];
    onchoose: (destination: RelatedDataDestination) => void;
    onclose: () => void;
  } = $props();

  let panel: HTMLElement;

  onMount(() => {
    panel.querySelector<HTMLButtonElement>("button")?.focus();
  });

  function keydown(event: KeyboardEvent): void {
    if (event.key !== "Escape") return;
    event.preventDefault();
    onclose();
  }
</script>

<div class="related-chooser-backdrop" role="presentation" onclick={onclose}>
  <div
    bind:this={panel}
    class="related-chooser"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="related-chooser-title"
    onkeydown={keydown}
    onclick={(event) => event.stopPropagation()}
  >
    <div>
      <span class="eyebrow">{$translation("related-nav-eyebrow")}</span>
      <h2 id="related-chooser-title">
        {$translation("related-nav-chooser-title")}
      </h2>
      <p>{$translation("related-nav-chooser-description")}</p>
    </div>
    <button
      class="dialog-close"
      type="button"
      aria-label={$translation("related-nav-close")}
      onclick={onclose}>×</button
    >
    <div class="related-chooser-actions">
      {#each destinations as destination (destination.id)}
        <button type="button" onclick={() => onchoose(destination)}>
          <strong>{$translation(destination.labelKey)}</strong>
          <span
            >{$translation(
              relationshipLabelKey(destination.relationship),
            )}</span
          >
        </button>
      {/each}
    </div>
  </div>
</div>
