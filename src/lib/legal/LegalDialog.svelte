<script lang="ts">
  import { translation } from "../i18n/runtime";
  import { modalFocus } from "../ui/modalFocus";

  let {
    open,
    active = true,
    layer = 0,
    onclose,
    onopenresearch,
  }: {
    open: boolean;
    active?: boolean;
    layer?: number;
    onclose: () => void;
    onopenresearch: () => void;
  } = $props();

  type LegalTab = "summary" | "research" | "licences";
  let activeTab = $state<LegalTab>("summary");
</script>

{#if open}
  <div
    class="legal-backdrop"
    inert={!active}
    aria-hidden={!active}
    data-dialog-active={active}
    style:z-index={300 + layer}
  >
    <dialog
      use:modalFocus={{ onclose, active }}
      open
      class="legal-dialog"
      aria-modal={active}
      aria-labelledby="legal-title"
      aria-describedby="legal-introduction"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("legal-dialog-eyebrow")}</span>
          <h2 id="legal-title">{$translation("legal-dialog-title")}</h2>
        </div>
        <button
          data-modal-autofocus
          class="dialog-close"
          type="button"
          aria-label={$translation("action-close")}
          onclick={onclose}>×</button
        >
      </header>

      <p id="legal-introduction">
        {$translation("legal-dialog-introduction")}
      </p>

      <div
        class="legal-tabs"
        role="tablist"
        aria-label={$translation("legal-tab-label")}
      >
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "summary"}
          onclick={() => (activeTab = "summary")}
          >{$translation("legal-tab-summary")}</button
        >
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "research"}
          onclick={() => (activeTab = "research")}
          >{$translation("legal-tab-research")}</button
        >
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "licences"}
          onclick={() => (activeTab = "licences")}
          >{$translation("legal-tab-licences")}</button
        >
      </div>

      <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-scrollable region) -->
      <section
        class="legal-content"
        aria-label={$translation("legal-dialog-title")}
        aria-live="polite"
        tabindex="0"
      >
        {#if activeTab === "summary"}
          <div id="legal-summary-panel" role="tabpanel" class="legal-grid">
            <article>
              <span>01</span>
              <h3>{$translation("legal-independent-community-project")}</h3>
              <p>{$translation("legal-independent-detail")}</p>
            </article>
            <article>
              <span>02</span>
              <h3>{$translation("legal-local-data-title")}</h3>
              <p>{$translation("legal-local-data-detail")}</p>
            </article>
            <article>
              <span>03</span>
              <h3>{$translation("legal-game-ownership-title")}</h3>
              <p>{$translation("legal-game-ownership-detail")}</p>
            </article>
            <article>
              <span>04</span>
              <h3>{$translation("legal-no-warranty-title")}</h3>
              <p>{$translation("legal-no-warranty-detail")}</p>
            </article>
          </div>
        {:else if activeTab === "research"}
          <div id="legal-research-panel" role="tabpanel" class="legal-stack">
            <article data-kind="observed">
              <h3>{$translation("legal-readonly-contract-title")}</h3>
              <p>{$translation("legal-readonly-contract-detail")}</p>
            </article>
            <article data-kind="risk">
              <h3>{$translation("legal-native-risk-title")}</h3>
              <p>{$translation("legal-native-risk-detail")}</p>
            </article>
            <article data-kind="risk">
              <h3>{$translation("legal-loader-configuration-title")}</h3>
              <p>{$translation("legal-loader-configuration-detail")}</p>
            </article>
            <article>
              <h3>{$translation("legal-build-gate-title")}</h3>
              <p>{$translation("legal-build-gate-detail")}</p>
            </article>
            <article data-kind="observed">
              <h3>{$translation("legal-source-download-title")}</h3>
              <p>{$translation("legal-source-download-detail")}</p>
            </article>
            <article>
              <h3>{$translation("legal-research-evidence-title")}</h3>
              <p>{$translation("legal-research-evidence-detail")}</p>
            </article>
            <article data-kind="observed">
              <h3>{$translation("legal-resource-readings-title")}</h3>
              <p>{$translation("legal-resource-readings-detail")}</p>
            </article>
            <article data-kind="risk">
              <h3>{$translation("legal-assurance-modes-title")}</h3>
              <p>{$translation("legal-assurance-modes-detail")}</p>
            </article>
            <button
              type="button"
              class="research-setup-link"
              onclick={onopenresearch}
            >
              {$translation("research-setup-open")}
            </button>
          </div>
        {:else}
          <div id="legal-licences-panel" role="tabpanel" class="legal-stack">
            <article>
              <h3>{$translation("legal-observatory-license-title")}</h3>
              <p>{$translation("legal-observatory-license-detail")}</p>
            </article>
            <article>
              <h3>{$translation("legal-probe-license-title")}</h3>
              <p>{$translation("legal-probe-license-detail")}</p>
            </article>
            <article>
              <h3>{$translation("legal-upstream-license-title")}</h3>
              <p>{$translation("legal-upstream-license-detail")}</p>
            </article>
            <p class="legal-advice-note">{$translation("legal-not-advice")}</p>
          </div>
        {/if}
      </section>

      <footer>
        <span>{$translation("legal-footer-boundary")}</span>
        <button type="button" onclick={onclose}
          >{$translation("action-close")}</button
        >
      </footer>
    </dialog>
  </div>
{/if}

<style>
  .legal-backdrop {
    position: fixed;
    z-index: 30;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(3, 7, 11, 0.84);
  }

  .legal-dialog {
    position: relative;
    inset: auto;
    width: min(920px, 100%);
    max-height: min(780px, calc(100vh - 48px));
    display: grid;
    grid-template-rows: auto auto auto minmax(180px, 1fr) auto;
    gap: 12px;
    margin: 0;
    padding: 20px;
    border: 1px solid var(--colour-line);
    color: var(--colour-text);
    background: var(--colour-surface);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h2 {
    margin-top: 4px;
    font-size: 25px;
  }

  #legal-introduction,
  footer span,
  article p,
  .legal-advice-note {
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

  .dialog-close {
    width: 36px;
    font-size: 20px;
  }

  .legal-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    border-block: 1px solid var(--colour-line-faint);
    padding-block: 8px;
  }

  .legal-tabs button[aria-selected="true"] {
    border-color: var(--colour-gold);
    color: var(--colour-gold);
    background: var(--colour-gold-soft);
  }

  .legal-content {
    min-height: 0;
    overflow-y: auto;
  }

  .legal-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 9px;
  }

  article {
    border: 1px solid var(--colour-line-faint);
    border-inline-start: 3px solid var(--colour-observed);
    padding: 13px;
    background: var(--colour-surface-raised);
  }

  article[data-kind="risk"] {
    border-inline-start-color: var(--colour-risk);
  }

  article > span {
    color: var(--colour-gold);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: var(--type-caption);
  }

  article h3 {
    margin-top: 5px;
    font-size: 1.0625rem;
  }

  article p {
    margin-top: 7px;
  }

  .legal-stack {
    display: grid;
    gap: 9px;
  }

  .research-setup-link {
    justify-self: start;
    border-color: var(--colour-observed);
    background: var(--colour-observed-soft);
  }

  .legal-advice-note {
    border: 1px dashed var(--colour-line);
    padding: 10px;
  }

  footer {
    border-top: 1px solid var(--colour-line-faint);
    padding-top: 10px;
  }

  @media (max-width: 680px) {
    .legal-backdrop {
      padding: 8px;
    }

    .legal-dialog {
      max-height: calc(100vh - 16px);
      padding: 14px;
    }

    .legal-grid {
      grid-template-columns: 1fr;
    }

    footer {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
