<script lang="ts">
  import { untrack } from "svelte";
  import AttentionCue from "../attention/AttentionCue.svelte";
  import { replayAttentionCue } from "../attention/service";
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import { notify } from "../notifications/service";
  import TaskProgressPanel from "../tasks/TaskProgressPanel.svelte";
  import { observeLatestTaskProgress } from "../tasks/progress";
  import { modalFocus } from "../ui/modalFocus";
  import {
    buildResearchProbe,
    chooseResearchCheckout,
    configureResearchCheckout,
    getResearchBuildProgress,
    getResearchSetup,
    listenForResearchBuildProgress,
    researchDesktopAvailable,
    setResearchNoticeAccepted,
  } from "./desktopClient";
  import { researchBuildProgressView } from "./progress";
  import type { ResearchBuildProgress, ResearchSetupStatus } from "./types";

  let {
    open,
    onclose,
    onopenlegal,
  }: {
    open: boolean;
    onclose: () => void;
    onopenlegal: () => void;
  } = $props();

  let status = $state<ResearchSetupStatus | null>(null);
  let progress = $state<ResearchBuildProgress | null>(null);
  let busy = $state(false);
  let errorMessage = $state("");
  let stopProgress: (() => void) | null = null;
  const progressView = $derived(
    progress ? researchBuildProgressView(progress, $translation) : null,
  );

  $effect(() => {
    if (!open) {
      stopProgress?.();
      stopProgress = null;
      return;
    }
    untrack(() => {
      void initialise();
    });
    return () => {
      stopProgress?.();
      stopProgress = null;
    };
  });

  async function initialise(): Promise<void> {
    errorMessage = "";
    stopProgress?.();
    stopProgress = await observeLatestTaskProgress(
      {
        read: getResearchBuildProgress,
        listen: listenForResearchBuildProgress,
      },
      (latest) => (progress = latest),
      (error) => (errorMessage = describeError(error)),
    );
    try {
      status = await getResearchSetup();
      progress = status.progress;
    } catch (error) {
      errorMessage = describeError(error);
    }
  }

  function describeError(error: unknown): string {
    if (typeof error === "object" && error && "code" in error) {
      return $translation("research-setup-error", {
        code: String((error as { code: unknown }).code),
      });
    }
    if (error instanceof Error && error.message) return error.message;
    return $translation("research-setup-error", { code: "unknown" });
  }

  async function acceptNotice(accepted: boolean): Promise<void> {
    busy = true;
    errorMessage = "";
    try {
      status = await setResearchNoticeAccepted(accepted);
    } catch (error) {
      errorMessage = describeError(error);
    } finally {
      busy = false;
    }
  }

  async function selectCheckout(): Promise<void> {
    const selected = await chooseResearchCheckout(
      $translation("research-setup-checkout-picker"),
    );
    if (!selected) return;
    busy = true;
    errorMessage = "";
    try {
      status = await configureResearchCheckout(selected);
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-setup-checkout-reviewed"),
        tone: "success",
      });
    } catch (error) {
      errorMessage = describeError(error);
      notify({
        title: $translation("research-setup-title"),
        message: errorMessage,
        tone: "error",
      });
    } finally {
      busy = false;
    }
  }

  async function build(): Promise<void> {
    busy = true;
    errorMessage = "";
    try {
      status = await buildResearchProbe();
      progress = status.progress;
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-setup-build-success"),
        tone: "success",
      });
    } catch (error) {
      errorMessage = describeError(error);
      status = await getResearchSetup().catch(() => status);
      notify({
        title: $translation("research-setup-title"),
        message: errorMessage,
        tone: "error",
      });
    } finally {
      busy = false;
    }
  }

  async function replayGuidance(): Promise<void> {
    await Promise.all([
      replayAttentionCue("research.setup.entry", 1),
      replayAttentionCue("research.setup.build", 1),
    ]);
    notify({
      title: $translation("research-setup-guidance-title"),
      message: $translation("research-setup-guidance-replayed"),
      tone: "info",
    });
  }

  function openLegal(): void {
    onclose();
    onopenlegal();
  }
</script>

{#if open}
  <div class="research-backdrop">
    <dialog
      use:modalFocus={{ onclose }}
      open
      class="research-dialog"
      aria-modal="true"
      aria-labelledby="research-setup-title"
      aria-describedby="research-setup-introduction"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("research-setup-eyebrow")}</span>
          <h2 id="research-setup-title">
            {$translation("research-setup-title")}
          </h2>
        </div>
        <button
          data-modal-autofocus
          class="dialog-close"
          type="button"
          aria-label={$translation("action-close")}
          onclick={onclose}>×</button
        >
      </header>

      <p id="research-setup-introduction">
        {$translation("research-setup-introduction")}
      </p>
      <p class="research-boundary">
        <strong>{$translation("research-setup-boundary-title")}</strong>
        {$translation("research-setup-boundary-detail")}
      </p>

      {#if errorMessage}
        <p class="research-error" role="alert">{errorMessage}</p>
      {/if}

      <div class="research-content">
        <ol class="research-steps">
          <li data-ready={status?.notice_accepted ?? false}>
            <span>01</span>
            <div>
              <h3>{$translation("research-setup-notice-title")}</h3>
              <p>{$translation("research-setup-notice-detail")}</p>
              <div class="button-row">
                <button type="button" onclick={openLegal}>
                  {$translation("research-setup-read-notice")}
                </button>
                {#if status?.notice_accepted}
                  <button
                    type="button"
                    disabled={busy}
                    onclick={() => void acceptNotice(false)}
                    >{$translation("research-setup-revoke-notice")}</button
                  >
                {:else}
                  <button
                    type="button"
                    class="primary"
                    disabled={busy || !researchDesktopAvailable()}
                    onclick={() => void acceptNotice(true)}
                    >{$translation("research-setup-accept-notice")}</button
                  >
                {/if}
              </div>
            </div>
            <strong
              >{status?.notice_accepted
                ? $translation("research-setup-ready")
                : $translation("research-setup-required")}</strong
            >
          </li>

          <li data-ready={status?.checkout_state === "reviewed"}>
            <span>02</span>
            <div>
              <h3>{$translation("research-setup-checkout-title")}</h3>
              <p>{$translation("research-setup-checkout-detail")}</p>
              {#if status?.checkout_path}
                <code>{status.checkout_path}</code>
              {/if}
              <div class="button-row">
                <button
                  type="button"
                  disabled={busy || !researchDesktopAvailable()}
                  onclick={() => void selectCheckout()}
                  >{$translation("research-setup-choose-checkout")}</button
                >
              </div>
            </div>
            <strong
              >{status?.checkout_state === "reviewed"
                ? $translation("research-setup-reviewed")
                : $translation("research-setup-required")}</strong
            >
          </li>

          <li
            data-ready={Boolean(
              status?.source_available && status?.compiler_available,
            )}
          >
            <span>03</span>
            <div>
              <h3>{$translation("research-setup-prerequisites-title")}</h3>
              <p>{$translation("research-setup-prerequisites-detail")}</p>
              <dl>
                <div>
                  <dt>{$translation("research-setup-source")}</dt>
                  <dd>
                    {status?.source_available
                      ? $translation("research-setup-available")
                      : $translation("research-setup-unavailable")}
                  </dd>
                </div>
                <div>
                  <dt>{$translation("research-setup-compiler")}</dt>
                  <dd>
                    {status?.compiler_available
                      ? $translation("research-setup-available")
                      : $translation("research-setup-unavailable")}
                  </dd>
                </div>
              </dl>
            </div>
            <strong
              >{status?.source_available && status?.compiler_available
                ? $translation("research-setup-ready")
                : $translation("research-setup-blocked")}</strong
            >
          </li>

          <li data-ready={status?.probe_built ?? false}>
            <span>04</span>
            <div>
              <h3>{$translation("research-setup-build-title")}</h3>
              <p>{$translation("research-setup-build-detail")}</p>
              <AttentionCue
                cueId="research.setup.build"
                contentRevision={1}
                heading={$translation("research-setup-build-cue-title")}
                detail={$translation("research-setup-build-cue-detail")}
                dismissLabel={$translation("attention-dismiss")}
                tone="important"
                enabled={status?.can_build ?? false}
              >
                <button
                  type="button"
                  class="primary"
                  disabled={busy || !status?.can_build}
                  onclick={() => void build()}
                  >{busy
                    ? $translation("research-setup-building")
                    : $translation("research-setup-build-action")}</button
                >
              </AttentionCue>
            </div>
            <strong
              >{status?.probe_built
                ? $translation("research-setup-built")
                : $translation("research-setup-not-built")}</strong
            >
          </li>
        </ol>

        {#if progressView && progress?.state !== "idle"}
          <TaskProgressPanel
            view={progressView}
            headingId="research-build-progress-title"
          />
        {/if}

        {#if status?.probe_built}
          <section class="artifact" aria-labelledby="research-artifact-title">
            <span class="eyebrow"
              >{$translation("research-setup-artifact-eyebrow")}</span
            >
            <h3 id="research-artifact-title">
              {$translation("research-setup-artifact-title")}
            </h3>
            <dl>
              <div>
                <dt>{$translation("research-setup-artifact-size")}</dt>
                <dd>
                  {formatNumber(status.probe_size_bytes ?? 0, $activeLocale)} B
                </dd>
              </div>
              <div>
                <dt>{$translation("research-setup-artifact-hash")}</dt>
                <dd><code>{status.probe_content_hash}</code></dd>
              </div>
              <div>
                <dt>{$translation("research-setup-artifact-location")}</dt>
                <dd><code>{status.output_path}</code></dd>
              </div>
            </dl>
          </section>
        {/if}

        {#if progress?.log_lines.length}
          <details class="build-log">
            <summary>{$translation("research-setup-build-log")}</summary>
            <pre>{progress.log_lines.join("\n")}</pre>
          </details>
        {/if}
      </div>

      <footer>
        <button type="button" onclick={() => void replayGuidance()}>
          {$translation("research-setup-replay-guidance")}
        </button>
        <button type="button" onclick={onclose}>
          {$translation("action-close")}
        </button>
      </footer>
    </dialog>
  </div>
{/if}

<style>
  .research-backdrop {
    position: fixed;
    z-index: 32;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(3, 7, 11, 0.86);
  }
  .research-dialog {
    position: relative;
    inset: auto;
    width: min(1040px, 100%);
    max-height: calc(100vh - 40px);
    display: grid;
    grid-template-rows: auto auto auto auto minmax(180px, 1fr) auto;
    gap: 10px;
    margin: 0;
    padding: 20px;
    border: 1px solid var(--colour-line);
    color: var(--colour-text);
    background: var(--colour-surface);
  }
  header,
  footer,
  .button-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  h2 {
    margin-top: 4px;
    font-size: 25px;
  }
  h3 {
    font-size: 1.0625rem;
  }
  p {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.55;
  }
  button {
    border: 1px solid var(--colour-line);
    padding: 8px 11px;
    color: var(--colour-text);
    background: var(--colour-surface-raised);
    cursor: pointer;
  }
  button.primary {
    border-color: var(--colour-gold);
    color: var(--colour-text);
    background: var(--colour-gold-soft);
  }
  button:disabled {
    cursor: not-allowed;
  }
  .dialog-close {
    width: 36px;
    font-size: 20px;
  }
  .research-boundary,
  .research-error {
    border-inline-start: 3px solid var(--colour-gold);
    padding: 9px 11px;
    color: var(--colour-text);
    background: var(--colour-gold-soft);
  }
  .research-boundary strong {
    margin-inline-end: 5px;
    color: var(--colour-gold);
  }
  .research-error {
    border-color: var(--colour-risk);
    background: var(--colour-risk-soft);
  }
  .research-content {
    min-height: 0;
    overflow-y: auto;
  }
  .research-steps {
    display: grid;
    gap: 8px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
  }
  .research-steps > li {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) max-content;
    gap: 11px;
    border: 1px solid var(--colour-line-faint);
    border-inline-start: 3px solid var(--colour-risk);
    padding: 12px;
    background: var(--colour-surface-raised);
  }
  .research-steps > li[data-ready="true"] {
    border-inline-start-color: var(--colour-success);
  }
  .research-steps > li > span,
  .research-steps > li > strong {
    color: var(--colour-gold);
    font-size: var(--type-caption);
    font-weight: 700;
  }
  .research-steps > li > strong {
    color: var(--colour-text);
    text-transform: uppercase;
  }
  .research-steps p {
    margin: 5px 0 8px;
  }
  .button-row {
    justify-content: flex-start;
    flex-wrap: wrap;
  }
  code {
    overflow-wrap: anywhere;
    color: var(--colour-observed);
    font-size: var(--type-caption);
  }
  dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    margin: 8px 0 0;
  }
  dl > div {
    border: 1px solid var(--colour-line-faint);
    padding: 8px;
    background: var(--colour-surface);
  }
  dt,
  dd {
    font-size: var(--type-caption);
  }
  dt {
    color: var(--colour-muted);
  }
  dd {
    margin: 4px 0 0;
    color: var(--colour-text);
  }
  .artifact {
    margin-top: 10px;
    border: 1px solid var(--colour-line-faint);
    padding: 12px;
    background: var(--colour-surface-raised);
  }
  .artifact h3 {
    margin-top: 4px;
  }
  .artifact dl {
    grid-template-columns: minmax(120px, 0.35fr) minmax(0, 1fr);
  }
  .artifact dl > div:last-child {
    grid-column: 1 / -1;
  }
  .build-log {
    margin-top: 10px;
    border: 1px solid var(--colour-line-faint);
    padding: 10px;
  }
  .build-log summary {
    cursor: pointer;
  }
  pre {
    max-height: 180px;
    overflow: auto;
    margin-top: 9px;
    padding: 9px;
    color: var(--colour-muted);
    background: var(--colour-canvas);
    font-size: var(--type-caption);
    white-space: pre-wrap;
  }
  footer {
    border-top: 1px solid var(--colour-line-faint);
    padding-top: 10px;
  }
  @media (max-width: 720px) {
    .research-backdrop {
      padding: 7px;
    }
    .research-dialog {
      max-height: calc(100vh - 14px);
      padding: 13px;
    }
    .research-steps > li {
      grid-template-columns: 30px minmax(0, 1fr);
    }
    .research-steps > li > strong {
      grid-column: 2;
    }
    dl,
    .artifact dl {
      grid-template-columns: 1fr;
    }
    .artifact dl > div:last-child {
      grid-column: auto;
    }
    footer {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
