<script lang="ts">
  import { translation } from "../i18n/runtime";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import { modalFocus } from "../ui/modalFocus";
  import { dismissRecoveryProposal, recoveryProposal } from "./service";

  let busy = $state(false);
  let failure = $state("");

  function close(): void {
    if (!busy) dismissRecoveryProposal();
  }

  async function recover(): Promise<void> {
    const proposal = $recoveryProposal;
    if (!proposal || busy) return;
    busy = true;
    failure = "";
    try {
      await proposal.run();
      dismissRecoveryProposal();
    } catch {
      failure = $translation("recovery-action-failed");
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    if ($recoveryProposal) failure = "";
  });
</script>

{#if $recoveryProposal}
  <div class="recovery-backdrop">
    <dialog
      use:modalFocus={{ onclose: close, closeDisabled: busy }}
      open
      class="recovery-dialog"
      aria-modal="true"
      aria-labelledby="recovery-title"
      aria-describedby="recovery-description"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("recovery-dialog-eyebrow")}</span>
          <h2 id="recovery-title">{$recoveryProposal.title}</h2>
        </div>
        <button
          class="dialog-close"
          type="button"
          disabled={busy}
          aria-label={$translation("action-close")}
          onclick={close}>×</button
        >
      </header>

      <p id="recovery-description">{$recoveryProposal.message}</p>

      {#if $recoveryProposal.consequence}
        <GuidanceSurface kind="instruction" layout="compact">
          <strong>{$translation("recovery-dialog-safety-title")}</strong>
          <p>{$recoveryProposal.consequence}</p>
        </GuidanceSurface>
      {/if}

      {#if failure}
        <p class="recovery-failure" role="alert">{failure}</p>
      {/if}

      <footer>
        <button type="button" disabled={busy} onclick={close}
          >{$translation("recovery-dialog-cancel")}</button
        >
        <button
          data-modal-autofocus
          class="primary-action"
          type="button"
          disabled={busy}
          onclick={recover}
          >{busy
            ? $translation("recovery-dialog-working")
            : $recoveryProposal.actionLabel}</button
        >
      </footer>
    </dialog>
  </div>
{/if}

<style>
  .recovery-backdrop {
    position: fixed;
    z-index: 260;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(2, 7, 12, 0.82);
  }

  .recovery-dialog {
    width: min(640px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--colour-line);
    padding: 22px;
    color: var(--colour-text);
    background: var(--colour-surface-raised);
    box-shadow: 0 22px 60px rgba(0, 0, 0, 0.52);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  header {
    margin-bottom: 16px;
  }

  h2,
  p {
    margin: 0;
  }

  #recovery-description {
    margin-bottom: 16px;
    color: var(--colour-muted);
    line-height: 1.55;
  }

  footer {
    justify-content: flex-end;
    margin-top: 20px;
  }

  footer button {
    min-height: 40px;
    border: 1px solid var(--colour-line);
    padding: 8px 14px;
    color: var(--colour-text);
    background: var(--colour-surface-soft);
  }

  .primary-action {
    border-color: var(--colour-gold);
    color: var(--colour-gold);
    background: var(--colour-surface-raised);
  }

  .recovery-failure {
    margin-top: 14px;
    border-inline-start: 3px solid var(--colour-risk);
    padding: 10px 12px;
    color: var(--colour-risk);
    background: var(--colour-surface-soft);
  }

  @media (max-width: 640px) {
    .recovery-backdrop {
      padding: 8px;
    }

    .recovery-dialog {
      width: calc(100vw - 16px);
      max-height: calc(100vh - 16px);
      padding: 16px;
    }

    footer {
      align-items: stretch;
      flex-direction: column-reverse;
    }
  }
</style>
