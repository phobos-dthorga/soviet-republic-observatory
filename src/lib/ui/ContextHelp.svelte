<script lang="ts">
  import type { ContextHelpDetail } from "./types";

  let {
    topic,
    title,
    text,
    details = [],
    placement = "left",
  }: {
    topic: string;
    title: string;
    text: string;
    details?: ContextHelpDetail[];
    placement?: "left" | "right" | "below";
  } = $props();

  let open = $state(false);
  const tooltipId = $derived(`context-help-${topic}`);

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      open = false;
      (event.currentTarget as HTMLElement).blur();
    }
  }
</script>

<span class="context-help" data-help-topic={topic} data-placement={placement}>
  <button
    type="button"
    aria-label={title}
    aria-expanded={open}
    aria-describedby={open ? tooltipId : undefined}
    onmouseenter={() => (open = true)}
    onmouseleave={() => (open = false)}
    onfocus={() => (open = true)}
    onblur={() => (open = false)}
    onkeydown={handleKeydown}
    onclick={() => (open = true)}>?</button
  >
  {#if open}
    <span
      class="context-tooltip guidance-surface"
      data-guidance-surface="help"
      data-guidance-layout="block"
      id={tooltipId}
      role="tooltip"
    >
      <strong>{title}</strong>
      <span>{text}</span>
      {#if details.length}
        <dl>
          {#each details as detail}
            <div>
              <dt>{detail.label}</dt>
              <dd>{detail.value}</dd>
            </div>
          {/each}
        </dl>
      {/if}
    </span>
  {/if}
</span>
