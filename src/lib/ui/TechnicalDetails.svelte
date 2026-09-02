<script lang="ts">
  import type { TranslationKey } from "../i18n/catalog";
  import { translation } from "../i18n/runtime";

  let {
    code,
    operation,
    detail,
    open = false,
  }: {
    code?: string;
    operation?: string;
    detail?: string;
    open?: boolean;
  } = $props();

  const explanations: Record<
    string,
    { happened: TranslationKey; next: TranslationKey }
  > = {
    research_source_archive_invalid: {
      happened: "error-explain-research-archive-happened",
      next: "error-explain-research-archive-next",
    },
    research_source_download_failed: {
      happened: "error-explain-research-download-happened",
      next: "error-explain-research-download-next",
    },
    research_source_install_failed: {
      happened: "error-explain-research-install-happened",
      next: "error-explain-research-install-next",
    },
    research_session_not_ready: {
      happened: "error-explain-research-session-not-ready-happened",
      next: "error-explain-research-session-not-ready-next",
    },
    research_session_conflict: {
      happened: "error-explain-research-session-conflict-happened",
      next: "error-explain-research-session-conflict-next",
    },
    research_session_preparation_failed: {
      happened: "error-explain-research-session-prepare-happened",
      next: "error-explain-research-session-prepare-next",
    },
    research_session_launch_failed: {
      happened: "error-explain-research-session-launch-happened",
      next: "error-explain-research-session-launch-next",
    },
    storage_busy: {
      happened: "error-explain-storage-busy-happened",
      next: "error-explain-storage-busy-next",
    },
    storage_unavailable: {
      happened: "error-explain-storage-unavailable-happened",
      next: "error-explain-storage-unavailable-next",
    },
  };
  const explanation = $derived(code ? explanations[code] : undefined);
</script>

<details class="technical-details" data-technical-details {open}>
  <summary>{$translation("technical-details-summary")}</summary>
  {#if explanation}
    <div class="explanation">
      <p>
        <strong>{$translation("technical-details-what-happened")}</strong>
        {$translation(explanation.happened)}
      </p>
      <p>
        <strong>{$translation("technical-details-next-step")}</strong>
        {$translation(explanation.next)}
      </p>
    </div>
  {/if}
  <dl>
    {#if code}
      <div>
        <dt>{$translation("technical-details-code")}</dt>
        <dd><code>{code}</code></dd>
      </div>
    {/if}
    {#if operation}
      <div>
        <dt>{$translation("technical-details-operation")}</dt>
        <dd><code>{operation}</code></dd>
      </div>
    {/if}
  </dl>
  {#if detail}
    <pre>{detail}</pre>
  {/if}
</details>

<style>
  .technical-details {
    margin-top: 14px;
    border: 1px solid var(--colour-line-faint);
    padding: 10px 12px;
    color: var(--colour-muted);
    background: var(--colour-surface-soft);
  }

  summary {
    color: var(--colour-text);
    cursor: pointer;
    font-weight: 700;
  }

  dl {
    display: grid;
    gap: 7px;
    margin: 10px 0 0;
  }

  .explanation {
    display: grid;
    gap: 8px;
    margin-top: 10px;
  }

  .explanation p {
    margin: 0;
    line-height: 1.5;
  }

  .explanation strong {
    display: block;
    color: var(--colour-text);
  }

  dl > div {
    display: grid;
    grid-template-columns: minmax(90px, 0.3fr) minmax(0, 1fr);
    gap: 10px;
  }

  dt,
  dd {
    margin: 0;
  }

  pre {
    overflow: auto;
    margin: 10px 0 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
