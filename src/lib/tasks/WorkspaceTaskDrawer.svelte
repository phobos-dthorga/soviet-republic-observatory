<script lang="ts">
  import { tick, type Snippet } from "svelte";

  let {
    open,
    eyebrow,
    title,
    description,
    closeLabel,
    route,
    onclose,
    children,
  }: {
    open: boolean;
    eyebrow: string;
    title: string;
    description: string;
    closeLabel: string;
    route: string;
    onclose: () => void;
    children: Snippet;
  } = $props();
  let closeButton = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    if (open) void tick().then(() => closeButton?.focus());
  });

  function handleKeydown(event: KeyboardEvent): void {
    if (!open || event.defaultPrevented || event.isComposing) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="task-drawer-layer" data-workspace-task={route}>
    <button
      class="task-drawer-scrim"
      type="button"
      aria-label={closeLabel}
      onclick={onclose}
    ></button>
    <div
      class="task-drawer"
      role="dialog"
      aria-modal="true"
      aria-labelledby={`${route}-title`}
    >
      <header>
        <div>
          <span class="eyebrow">{eyebrow}</span>
          <h2 id={`${route}-title`}>{title}</h2>
          <p>{description}</p>
        </div>
        <button
          type="button"
          class="close"
          bind:this={closeButton}
          onclick={onclose}
        >
          {closeLabel}
        </button>
      </header>
      <div class="task-drawer-body">{@render children()}</div>
    </div>
  </div>
{/if}

<style>
  .task-drawer-layer {
    position: fixed;
    z-index: 190;
    inset: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(34rem, min(52rem, 68vw));
  }
  .task-drawer-scrim {
    width: 100%;
    height: 100%;
    border: 0;
    background: color-mix(in srgb, var(--colour-canvas) 82%, transparent);
    cursor: default;
  }
  .task-drawer {
    min-width: 0;
    height: 100%;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    border-inline-start: 1px solid var(--colour-line);
    color: var(--colour-text);
    background: var(--colour-surface);
    box-shadow: -1rem 0 2.5rem rgba(0, 0, 0, 0.42);
  }
  .task-drawer > header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.25rem 1.35rem;
    border-bottom: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
  }
  .task-drawer h2 {
    margin-top: 0.3rem;
    font-size: 1.55rem;
  }
  .task-drawer p {
    max-width: 62ch;
    margin-top: 0.45rem;
    color: var(--colour-muted);
    line-height: 1.5;
  }
  .close {
    flex: 0 0 auto;
  }
  .task-drawer-body {
    min-width: 0;
    overflow-y: auto;
    padding: 1.25rem 1.35rem 2rem;
  }
  @media (max-width: 760px) {
    .task-drawer-layer {
      grid-template-columns: 1fr;
    }
    .task-drawer-scrim {
      display: none;
    }
    .task-drawer {
      grid-column: 1;
      border-inline-start: 0;
    }
  }
</style>
