<script lang="ts">
  let {
    topic,
    title,
    text,
    placement = "left",
  }: {
    topic: string;
    title: string;
    text: string;
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
    <span class="context-tooltip" id={tooltipId} role="tooltip">
      <strong>{title}</strong>
      <span>{text}</span>
    </span>
  {/if}
</span>
