<script lang="ts">
  import { translation } from "../i18n/runtime";

  let {
    code,
    operation,
    detail,
  }: {
    code?: string;
    operation?: string;
    detail?: string;
  } = $props();
</script>

<details class="technical-details" data-technical-details>
  <summary>{$translation("technical-details-summary")}</summary>
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
