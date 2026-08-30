<script lang="ts">
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatDate } from "../i18n/format";
  import type { DiagnosticLogView } from "../observations/types";
  import { modalFocus } from "../ui/modalFocus";

  let {
    open,
    busy,
    log,
    errorMessage,
    onclose,
    onrefresh,
    onclear,
  }: {
    open: boolean;
    busy: boolean;
    log: DiagnosticLogView | null;
    errorMessage: string;
    onclose: () => void;
    onrefresh: () => void;
    onclear: () => void;
  } = $props();

  const newestEntries = $derived([...(log?.entries ?? [])].reverse());
</script>

{#if open}
  <div class="diagnostics-backdrop">
    <dialog
      use:modalFocus={{ onclose, closeDisabled: busy }}
      open
      class="diagnostics-dialog"
      aria-modal="true"
      aria-labelledby="diagnostics-title"
      aria-describedby="diagnostics-introduction"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("diagnostics-eyebrow")}</span>
          <h2 id="diagnostics-title">{$translation("diagnostics-title")}</h2>
        </div>
        <button
          data-modal-autofocus
          class="dialog-close"
          type="button"
          aria-label={$translation("action-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <p id="diagnostics-introduction">
        {$translation("diagnostics-introduction")}
      </p>

      <div class="diagnostics-meta">
        <span
          >{log?.entries.length ?? 0}
          {$translation("diagnostics-entries")}</span
        >
        <span>{log?.language ?? "English"}</span>
        <span>{$translation("diagnostics-storage-local")}</span>
      </div>

      {#if errorMessage}
        <p class="diagnostics-error" role="alert">{errorMessage}</p>
      {/if}

      <section class="diagnostic-list" aria-live="polite">
        {#if newestEntries.length === 0}
          <p class="empty-log">{$translation("diagnostics-empty")}</p>
        {:else}
          {#each newestEntries as entry}
            <article data-level={entry.level}>
              <div>
                <strong>{entry.code}</strong>
                <time datetime={new Date(entry.occurred_at_ms).toISOString()}>
                  {formatDate(entry.occurred_at_ms, $activeLocale, {
                    dateStyle: "short",
                    timeStyle: "medium",
                  })}
                </time>
              </div>
              <p>{entry.message}</p>
              <small>{entry.level} · {entry.operation}</small>
            </article>
          {/each}
        {/if}
      </section>

      <p class="diagnostics-boundary">
        {$translation("diagnostics-boundary")}
      </p>

      <footer>
        <button type="button" disabled={busy} onclick={onrefresh}>
          {$translation("diagnostics-refresh")}
        </button>
        <button
          type="button"
          class="clear-button"
          disabled={busy || newestEntries.length === 0}
          onclick={onclear}
        >
          {$translation("diagnostics-clear")}
        </button>
        <button type="button" disabled={busy} onclick={onclose}>
          {$translation("action-close")}
        </button>
      </footer>
    </dialog>
  </div>
{/if}

<style>
  .diagnostics-backdrop {
    position: fixed;
    z-index: 30;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(3, 7, 11, 0.82);
  }
  .diagnostics-dialog {
    width: min(880px, 100%);
    max-height: min(760px, calc(100vh - 48px));
    display: grid;
    grid-template-rows: auto auto auto auto minmax(120px, 1fr) auto auto;
    gap: 12px;
    margin: 0;
    padding: 20px;
    border: 1px solid var(--colour-line);
    color: var(--colour-text);
    background: var(--colour-surface);
  }
  header,
  footer,
  .diagnostics-meta,
  article > div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  h2 {
    margin-top: 4px;
    font-size: 25px;
  }
  #diagnostics-introduction,
  .diagnostics-boundary,
  .empty-log {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.55;
  }
  button {
    border: 1px solid var(--colour-line-faint);
    padding: 8px 11px;
    color: var(--colour-text);
    background: var(--colour-surface-raised);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .dialog-close {
    width: 34px;
    font-size: 20px;
  }
  .diagnostics-meta {
    justify-content: flex-start;
    flex-wrap: wrap;
    color: var(--colour-observed);
    font-size: var(--type-caption);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .diagnostics-meta span {
    padding: 5px 7px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-observed-soft);
  }
  .diagnostic-list {
    min-height: 0;
    overflow-y: auto;
    border-block: 1px solid var(--colour-line-faint);
  }
  article {
    padding: 11px;
    border-inline-start: 3px solid var(--colour-observed);
    border-bottom: 1px solid var(--colour-line-faint);
    background: rgba(18, 33, 45, 0.58);
  }
  article[data-level="error"] {
    border-inline-start-color: var(--colour-risk);
  }
  article[data-level="warning"] {
    border-inline-start-color: var(--colour-gold);
  }
  article strong,
  article time,
  article small {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: var(--type-caption);
  }
  article strong {
    color: var(--colour-observed);
  }
  article time,
  article small {
    color: var(--colour-muted);
  }
  article p {
    margin: 7px 0;
    font-size: var(--type-caption);
  }
  .diagnostics-error {
    padding: 9px;
    border: 1px solid var(--colour-risk);
    color: var(--colour-risk);
  }
  .diagnostics-boundary {
    padding-inline-start: 10px;
    border-inline-start: 2px solid var(--colour-gold);
  }
  footer {
    justify-content: flex-end;
  }
  .clear-button {
    color: var(--colour-risk);
  }
  @media (max-width: 640px) {
    .diagnostics-backdrop {
      padding: 10px;
    }
    .diagnostics-dialog {
      max-height: calc(100vh - 20px);
      padding: 14px;
    }
    article > div {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
