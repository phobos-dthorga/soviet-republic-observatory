<script lang="ts">
  import { translation } from "../i18n/runtime";
  import type { TechnicalDetailsView } from "../notifications/service";
  import TechnicalDetails from "./TechnicalDetails.svelte";

  let {
    message,
    technicalDetails,
  }: {
    message: string;
    technicalDetails?: TechnicalDetailsView;
  } = $props();

  let expanded = $state(false);
</script>

<div class="error-summary" data-error-summary>
  <button
    type="button"
    class="error-summary-trigger"
    aria-expanded={expanded}
    onclick={() => (expanded = !expanded)}
  >
    <span>{message}</span>
    <strong>
      {expanded
        ? $translation("error-details-hide")
        : $translation("error-details-open")}
    </strong>
  </button>
  {#if expanded && technicalDetails}
    <TechnicalDetails {...technicalDetails} open />
  {/if}
</div>

<style>
  .error-summary-trigger {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content;
    align-items: start;
    gap: 12px;
    border: 0;
    padding: 0;
    color: inherit;
    background: transparent;
    text-align: start;
    cursor: pointer;
  }

  .error-summary-trigger strong {
    color: var(--colour-observed);
    font-size: var(--type-caption);
    white-space: nowrap;
  }

  @media (max-width: 560px) {
    .error-summary-trigger {
      grid-template-columns: 1fr;
    }
  }
</style>
