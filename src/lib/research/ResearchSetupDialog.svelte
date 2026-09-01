<script lang="ts">
  import { tick, untrack } from "svelte";
  import AttentionCue from "../attention/AttentionCue.svelte";
  import { replayAttentionCue } from "../attention/service";
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import { notify, openRecoveryProposal } from "../notifications/service";
  import TaskProgressPanel from "../tasks/TaskProgressPanel.svelte";
  import { observeLatestTaskProgress } from "../tasks/progress";
  import TechnicalDetails from "../ui/TechnicalDetails.svelte";
  import { modalFocus } from "../ui/modalFocus";
  import {
    buildResearchProbe,
    chooseResearchCheckout,
    configureResearchCheckout,
    downloadReviewedTesmioSource,
    getResearchBuildProgress,
    getResearchSourceDownloadProgress,
    getResearchSetup,
    listenForResearchBuildProgress,
    listenForResearchSourceDownloadProgress,
    researchDesktopAvailable,
    setResearchNoticeAccepted,
  } from "./desktopClient";
  import { researchBuildProgressView } from "./progress";
  import type {
    ResearchBuildProgress,
    ResearchSetupStatus,
    ResearchSourceDownloadProgress,
  } from "./types";

  let {
    open,
    active = true,
    layer = 0,
    onclose,
    onopenlegal,
    onopendiagnostics,
  }: {
    open: boolean;
    active?: boolean;
    layer?: number;
    onclose: () => void;
    onopenlegal: () => void;
    onopendiagnostics: () => void;
  } = $props();

  let status = $state<ResearchSetupStatus | null>(null);
  let progress = $state<ResearchBuildProgress | null>(null);
  let downloadProgress = $state<ResearchSourceDownloadProgress | null>(null);
  let busy = $state(false);
  let errorMessage = $state("");
  let errorCode = $state("");
  let stopProgress: (() => void) | null = null;
  let stopDownloadProgress: (() => void) | null = null;
  let researchContent = $state<HTMLDivElement>();
  let researchResults = $state<HTMLDivElement>();
  const progressView = $derived(
    progress ? researchBuildProgressView(progress, $translation) : null,
  );
  const buildFailure = $derived(
    progress?.state === "failed" ? describeBuildFailure(progress) : null,
  );

  $effect(() => {
    if (!open) {
      stopProgress?.();
      stopProgress = null;
      stopDownloadProgress?.();
      stopDownloadProgress = null;
      return;
    }
    untrack(() => {
      void initialise();
    });
    return () => {
      stopProgress?.();
      stopProgress = null;
      stopDownloadProgress?.();
      stopDownloadProgress = null;
    };
  });

  async function initialise(): Promise<void> {
    errorMessage = "";
    errorCode = "";
    stopProgress?.();
    stopProgress = await observeLatestTaskProgress(
      {
        read: getResearchBuildProgress,
        listen: listenForResearchBuildProgress,
      },
      (latest) => (progress = latest),
      (error) => (errorMessage = describeError(error)),
    );
    stopDownloadProgress = await listenForResearchSourceDownloadProgress(
      (latest) => (downloadProgress = latest),
    );
    try {
      status = await getResearchSetup();
      progress = status.progress;
      downloadProgress = await getResearchSourceDownloadProgress();
      await tick();
      researchContent?.scrollTo({ top: 0 });
    } catch (error) {
      errorMessage = describeError(error);
    }
  }

  function describeError(error: unknown): string {
    if (typeof error === "object" && error && "code" in error) {
      errorCode = String((error as { code: unknown }).code);
    } else {
      errorCode = "unknown";
    }
    return $translation("research-setup-error-summary");
  }

  function describeBuildFailure(value: ResearchBuildProgress): {
    detail: string;
    remediation: string;
  } {
    const code = value.error_code ?? "unknown";
    const detailKey =
      code === "research_notice_required"
        ? "research-setup-failure-notice"
        : code === "research_source_unavailable"
          ? "research-setup-failure-source"
          : code === "research_checkout_required" ||
              code === "research_checkout_missing"
            ? "research-setup-failure-checkout-missing"
            : code === "research_checkout_unsupported"
              ? "research-setup-failure-checkout-unsupported"
              : code === "research_toolchain_unavailable"
                ? "research-setup-failure-toolchain"
                : code === "research_artifact_invalid"
                  ? "research-setup-failure-artifact"
                  : code === "research_build_failed"
                    ? "research-setup-failure-compile"
                    : "research-setup-failure-unknown";
    const remediationKey =
      value.remediation_code === "review_research_notice"
        ? "research-setup-remediation-notice"
        : value.remediation_code === "choose_reviewed_checkout"
          ? "research-setup-remediation-checkout"
          : value.remediation_code === "install_visual_cpp_build_tools"
            ? "research-setup-remediation-toolchain"
            : value.remediation_code === "repair_application_installation"
              ? "research-setup-remediation-source"
              : "research-setup-remediation-diagnostics";
    return {
      detail: $translation(detailKey),
      remediation: $translation(remediationKey),
    };
  }

  function failureStageLabel(stage: string | null | undefined): string {
    return stage === "toolchain"
      ? $translation("research-setup-stage-toolchain")
      : stage === "compiling"
        ? $translation("research-setup-stage-compile")
        : stage === "verifying"
          ? $translation("research-setup-stage-verify")
          : $translation("research-setup-stage-preflight");
  }

  async function acceptNotice(accepted: boolean): Promise<void> {
    busy = true;
    errorMessage = "";
    errorCode = "";
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
    errorCode = "";
    try {
      status = await configureResearchCheckout(selected);
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-setup-checkout-reviewed"),
        tone: "success",
        dedupeKey: "research.checkout.result",
      });
    } catch (error) {
      errorMessage = describeError(error);
      notify({
        title: $translation("research-setup-title"),
        message: errorMessage,
        tone: "error",
        dedupeKey: "research.checkout.result",
        technicalDetails: {
          code: errorCode,
          operation: "research_checkout",
        },
      });
    } finally {
      busy = false;
    }
  }

  function confirmDownload(): void {
    openRecoveryProposal({
      title: $translation("research-setup-download-confirm-title"),
      message: $translation("research-setup-download-confirm-detail"),
      consequence: $translation("research-setup-download-confirm-safety"),
      actionLabel: $translation("research-setup-download-confirm-action"),
      run: downloadSource,
    });
  }

  async function downloadSource(): Promise<void> {
    busy = true;
    errorMessage = "";
    errorCode = "";
    try {
      status = await downloadReviewedTesmioSource();
      downloadProgress = status.download_progress;
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-setup-download-success"),
        tone: "success",
        dedupeKey: "research.source.download",
      });
    } catch (error) {
      downloadProgress = await getResearchSourceDownloadProgress().catch(
        () => downloadProgress,
      );
      errorMessage = describeError(error);
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-setup-download-failure"),
        tone: "error",
        dedupeKey: "research.source.download",
        technicalDetails: {
          code: errorCode,
          operation: "research_source_download",
        },
      });
      throw error;
    } finally {
      busy = false;
    }
  }

  async function build(): Promise<void> {
    busy = true;
    errorMessage = "";
    errorCode = "";
    try {
      status = await buildResearchProbe();
      progress = status.progress;
      await revealResearchResults();
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-setup-build-success"),
        tone: "success",
      });
    } catch (error) {
      status = await getResearchSetup().catch(() => status);
      if (status) progress = status.progress;
      errorMessage =
        progress?.state === "failed"
          ? describeBuildFailure(progress).detail
          : describeError(error);
      errorCode = (progress?.error_code ?? errorCode) || "unknown";
      notify({
        title: $translation("research-setup-title"),
        message: errorMessage,
        tone: "error",
        dedupeKey: "research.build.failure",
        technicalDetails: {
          code: errorCode,
          operation: "research_probe_build",
        },
      });
    } finally {
      busy = false;
    }
  }

  async function revealResearchResults(): Promise<void> {
    await tick();
    if (!researchContent || !researchResults) return;
    const contentBox = researchContent.getBoundingClientRect();
    const resultBox = researchResults.getBoundingClientRect();
    researchContent.scrollTo({
      top: Math.max(
        0,
        researchContent.scrollTop + resultBox.top - contentBox.top,
      ),
      behavior: "instant",
    });
    const settledContentBox = researchContent.getBoundingClientRect();
    const clippedStep = [
      ...researchContent.querySelectorAll<HTMLElement>(".research-steps > li"),
    ].find((step) => {
      const box = step.getBoundingClientRect();
      return (
        box.top < settledContentBox.top &&
        box.bottom > settledContentBox.top + 1
      );
    });
    if (clippedStep) {
      researchContent.scrollTop +=
        clippedStep.getBoundingClientRect().top - settledContentBox.top;
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
    onopenlegal();
  }

  function openDiagnostics(): void {
    onopendiagnostics();
  }
</script>

{#if open}
  <div
    class="research-backdrop"
    inert={!active}
    aria-hidden={!active}
    data-dialog-active={active}
    style:z-index={300 + layer}
  >
    <dialog
      use:modalFocus={{ onclose, closeDisabled: busy, active }}
      open
      class="research-dialog"
      aria-modal={active}
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
      <p
        class="research-boundary guidance-surface"
        data-guidance-surface="boundary"
        data-guidance-layout="compact"
        role="note"
      >
        <strong>{$translation("research-setup-boundary-title")}</strong>
        {$translation("research-setup-boundary-detail")}
      </p>

      {#if errorMessage && !buildFailure}
        <div class="research-error" role="alert">
          <p>{errorMessage}</p>
          <TechnicalDetails code={errorCode} operation="research_setup" />
        </div>
      {/if}

      <div class="research-content" bind:this={researchContent}>
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
              {#if status?.checkout_name}
                <p class="safe-location">
                  {status.source_origin === "observatory_downloaded"
                    ? $translation("research-setup-managed-source")
                    : $translation("research-setup-selected-folder")}
                  <code>{status.checkout_name}</code>
                </p>
              {/if}
              <div class="button-row">
                <button
                  type="button"
                  disabled={busy || !researchDesktopAvailable()}
                  onclick={() => void selectCheckout()}
                  >{$translation("research-setup-choose-checkout")}</button
                >
                <button
                  type="button"
                  class="primary"
                  disabled={busy ||
                    !status?.can_download ||
                    !researchDesktopAvailable()}
                  onclick={confirmDownload}
                  >{status?.source_origin === "observatory_downloaded" &&
                  status?.checkout_state !== "reviewed"
                    ? $translation("research-setup-repair-download")
                    : $translation("research-setup-download-action")}</button
                >
              </div>
              <p class="download-privacy">
                {$translation("research-setup-download-privacy-short")}
              </p>
              {#if downloadProgress?.state === "running"}
                <div class="source-download-progress" aria-live="polite">
                  <strong
                    >{$translation("research-setup-download-running")}</strong
                  >
                  <progress
                    max="100"
                    value={downloadProgress.progress_percent ?? undefined}
                  ></progress>
                  <span>
                    {formatNumber(
                      downloadProgress.transferred_bytes,
                      $activeLocale,
                    )} B
                  </span>
                </div>
              {/if}
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
              {#if status?.artifact_state === "missing"}
                <p class="artifact-warning">
                  {$translation("research-setup-artifact-missing")}
                </p>
              {/if}
            </div>
            <strong>
              {status?.artifact_state === "verified"
                ? $translation("research-setup-built")
                : status?.artifact_state === "unrecorded"
                  ? $translation("research-setup-detected")
                  : status?.artifact_state === "changed"
                    ? $translation("research-setup-changed")
                    : status?.artifact_state === "missing"
                      ? $translation("research-setup-missing")
                      : $translation("research-setup-not-built")}
            </strong>
          </li>
        </ol>

        <div class="research-results" bind:this={researchResults}>
          {#if buildFailure}
            <section class="failure-assay" role="alert">
              <div>
                <span class="eyebrow"
                  >{$translation("research-setup-failure-eyebrow")}</span
                >
                <h3>{$translation("research-setup-failure-title")}</h3>
              </div>
              <p>{buildFailure.detail}</p>
              <p>
                <strong>{$translation("research-setup-next-step")}</strong>
                {buildFailure.remediation}
              </p>
              <dl>
                <div>
                  <dt>{$translation("research-setup-failure-stage")}</dt>
                  <dd>{failureStageLabel(progress?.failed_stage)}</dd>
                </div>
              </dl>
              <TechnicalDetails
                code={progress?.error_code ?? "unknown"}
                operation="research_probe_build"
                detail={progress?.compiler_exit_code == null
                  ? undefined
                  : `${$translation("research-setup-failure-exit-code")}: ${progress.compiler_exit_code}`}
              />
              <button type="button" onclick={openDiagnostics}>
                {$translation("research-setup-open-diagnostics")}
              </button>
            </section>
          {/if}

          {#if progressView && progress?.state !== "idle"}
            <TaskProgressPanel
              view={progressView}
              headingId="research-build-progress-title"
            />
          {/if}

          {#if status && status.artifact_state !== "absent" && status.artifact_state !== "missing"}
            <section class="artifact" aria-labelledby="research-artifact-title">
              <span class="eyebrow"
                >{$translation("research-setup-artifact-eyebrow")}</span
              >
              <h3 id="research-artifact-title">
                {$translation("research-setup-artifact-title")}
              </h3>
              {#if status.artifact_state !== "verified"}
                <p class="artifact-warning">
                  {status.artifact_state === "unrecorded"
                    ? $translation("research-setup-artifact-unrecorded")
                    : $translation("research-setup-artifact-changed")}
                </p>
              {/if}
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
                  <dd><code>{status.output_display_path}</code></dd>
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
    display: flex;
    flex-direction: column;
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
  .research-boundary {
    --guidance-padding: 9px 11px;
    border-color: var(--colour-guidance);
    border-inline-start-width: 3px;
    padding: 9px 11px;
    color: var(--colour-text);
    background:
      linear-gradient(110deg, var(--colour-guidance-soft), transparent 76%),
      var(--colour-surface);
  }
  .research-boundary strong {
    margin-inline-end: 5px;
    color: var(--colour-guidance);
  }
  .research-error {
    border-inline-start: 3px solid var(--colour-risk);
    padding: 9px 11px;
    color: var(--colour-text);
    background: var(--colour-risk-soft);
  }
  .research-content {
    flex: 0 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-anchor: none;
    scroll-padding-block: 8px;
  }
  .research-results {
    display: grid;
    gap: 10px;
    scroll-margin-block-start: 8px;
  }
  .research-results :global(.task-progress),
  .research-results .failure-assay,
  .research-results .artifact,
  .research-results .build-log {
    margin-block: 0;
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
    align-self: start;
    padding: 3px 6px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
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
  .safe-location {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 6px;
    margin: 2px 0 8px;
  }
  .download-privacy {
    margin-bottom: 0;
  }
  .source-download-progress {
    display: grid;
    grid-template-columns: max-content minmax(120px, 1fr) max-content;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    border: 1px solid var(--colour-line-faint);
    padding: 8px;
    color: var(--colour-text);
    background: var(--colour-surface);
    font-size: var(--type-caption);
  }
  .source-download-progress progress {
    width: 100%;
    accent-color: var(--colour-observed);
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
  .artifact-warning,
  .failure-assay {
    border-inline-start: 3px solid var(--colour-gold);
    color: var(--colour-text);
    background: var(--colour-gold-soft);
  }
  .artifact-warning {
    margin: 8px 0 0;
    padding: 8px 10px;
  }
  .failure-assay {
    display: grid;
    gap: 8px;
    margin-bottom: 10px;
    padding: 12px;
    border-color: var(--colour-risk);
    background: var(--colour-risk-soft);
  }
  .failure-assay p,
  .failure-assay dl {
    margin: 0;
  }
  .failure-assay button {
    justify-self: start;
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
