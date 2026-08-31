<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    attentionRevision,
    dismissAttentionCue,
    getAttentionCueStatus,
  } from "./service";
  import type { AttentionCueTone } from "./types";

  let {
    cueId,
    contentRevision,
    heading,
    detail,
    dismissLabel,
    actionLabel,
    onaction,
    tone = "information",
    enabled = true,
    layout = "compact",
    children,
  }: {
    cueId: string;
    contentRevision: number;
    heading: string;
    detail: string;
    dismissLabel: string;
    actionLabel?: string;
    onaction?: () => void;
    tone?: AttentionCueTone;
    enabled?: boolean;
    layout?: "compact" | "wide";
    children: Snippet;
  } = $props();

  let active = $state(false);
  let loading = $state(true);
  const detailId = $derived(`attention-cue-${cueId.replaceAll(".", "-")}`);

  $effect(() => {
    void $attentionRevision;
    if (!enabled) {
      active = false;
      loading = false;
      return;
    }
    let cancelled = false;
    loading = true;
    void getAttentionCueStatus(cueId, contentRevision)
      .then((status) => {
        if (!cancelled) active = !status.dismissed;
      })
      .catch(() => {
        if (!cancelled) active = true;
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  async function dismiss(): Promise<void> {
    active = false;
    await dismissAttentionCue(cueId, contentRevision).catch(() => {
      active = true;
    });
  }

  function activate(): void {
    onaction?.();
  }
</script>

<div
  class="attention-cue"
  class:active
  data-attention-cue={cueId}
  data-attention-tone={tone}
  data-attention-loading={loading}
  data-attention-layout={layout}
  aria-describedby={active ? detailId : undefined}
>
  <div class="attention-target">{@render children()}</div>
  {#if active}
    <aside id={detailId} class="attention-message" role="status">
      <strong>{heading}</strong>
      <span>{detail}</span>
      <div>
        {#if actionLabel && onaction}
          <button type="button" onclick={activate}>{actionLabel}</button>
        {/if}
        <button type="button" class="dismiss" onclick={() => void dismiss()}>
          {dismissLabel}
        </button>
      </div>
    </aside>
  {/if}
</div>

<style>
  .attention-cue {
    position: relative;
    display: grid;
    width: min(460px, 100%);
    min-width: 0;
    justify-items: start;
  }

  .attention-cue[data-attention-layout="wide"] {
    width: 100%;
  }

  .attention-target {
    position: relative;
    z-index: 1;
    display: inline-flex;
    width: fit-content;
    max-width: 100%;
  }

  .attention-cue.active .attention-target::after {
    content: "";
    position: absolute;
    z-index: -1;
    inset: -5px;
    border: 2px solid var(--colour-observed);
    pointer-events: none;
    animation: attention-pulse 1.25s ease-out 3;
  }

  .attention-cue[data-attention-tone="important"].active
    .attention-target::after {
    border-color: var(--colour-gold);
  }

  .attention-cue[data-attention-tone="success"].active
    .attention-target::after {
    border-color: var(--colour-success);
  }

  .attention-message {
    position: relative;
    z-index: 2;
    display: grid;
    width: 100%;
    gap: 6px;
    margin-top: 9px;
    border: 1px solid var(--colour-guidance);
    border-inline-start-width: 3px;
    padding: 10px;
    color: var(--colour-text);
    background:
      linear-gradient(110deg, var(--colour-guidance-soft), transparent 76%),
      var(--colour-surface);
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.28);
  }

  .attention-message > span {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.5;
  }

  .attention-message > div {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  button {
    border: 1px solid var(--colour-line);
    padding: 7px 10px;
    color: var(--colour-text);
    background: var(--colour-surface-raised);
    cursor: pointer;
  }

  button.dismiss {
    color: var(--colour-muted);
  }

  @keyframes attention-pulse {
    0% {
      opacity: 0.95;
      box-shadow: 0 0 0 0 var(--colour-observed-soft);
    }
    100% {
      opacity: 0;
      box-shadow: 0 0 0 12px transparent;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .attention-cue.active .attention-target::after {
      animation: none;
    }
  }

  @media (forced-colors: active) {
    .attention-cue.active .attention-target::after,
    .attention-message {
      border-color: Highlight;
    }
  }
</style>
