<script lang="ts">
  import { tick } from "svelte";
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
  let trigger = $state<HTMLButtonElement>();
  let tooltip = $state<HTMLSpanElement>();
  let tooltipPosition = $state<{ left: number; top: number } | null>(null);
  const tooltipId = $derived(`context-help-${topic}`);

  async function showTooltip(): Promise<void> {
    open = true;
    tooltipPosition = null;
    await tick();
    positionTooltip();
  }

  function hideTooltip(): void {
    open = false;
    tooltipPosition = null;
  }

  function positionTooltip(): void {
    if (!trigger || !tooltip) return;
    const margin = 16;
    const gap = 8;
    const triggerBox = trigger.getBoundingClientRect();
    const tooltipBox = tooltip.getBoundingClientRect();
    const preferredLeft =
      placement === "left"
        ? triggerBox.right - tooltipBox.width
        : triggerBox.left;
    const left = Math.min(
      Math.max(margin, preferredLeft),
      Math.max(margin, window.innerWidth - tooltipBox.width - margin),
    );
    const below = triggerBox.bottom + gap;
    const above = triggerBox.top - tooltipBox.height - gap;
    const preferredTop =
      below + tooltipBox.height <= window.innerHeight - margin ? below : above;
    const top = Math.min(
      Math.max(margin, preferredTop),
      Math.max(margin, window.innerHeight - tooltipBox.height - margin),
    );
    tooltipPosition = { left, top };
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      hideTooltip();
      (event.currentTarget as HTMLElement).blur();
    }
  }
</script>

<span class="context-help" data-help-topic={topic} data-placement={placement}>
  <button
    bind:this={trigger}
    type="button"
    aria-label={title}
    aria-expanded={open}
    aria-describedby={open ? tooltipId : undefined}
    onmouseenter={showTooltip}
    onmouseleave={hideTooltip}
    onfocus={showTooltip}
    onblur={hideTooltip}
    onkeydown={handleKeydown}
    onclick={showTooltip}>?</button
  >
  {#if open}
    <span
      bind:this={tooltip}
      class="context-tooltip guidance-surface"
      data-guidance-surface="help"
      data-guidance-layout="block"
      id={tooltipId}
      role="tooltip"
      style={tooltipPosition
        ? `position: fixed; inset-inline-start: ${tooltipPosition.left}px; inset-inline-end: auto; top: ${tooltipPosition.top}px;`
        : undefined}
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
