<script lang="ts">
  import { tick, untrack } from "svelte";
  import AttentionCue from "../attention/AttentionCue.svelte";
  import { replayAttentionCue } from "../attention/service";
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import { notify, openRecoveryProposal } from "../notifications/service";
  import { detailsFromError } from "../notifications/errors";
  import TaskProgressPanel from "../tasks/TaskProgressPanel.svelte";
  import { observeLatestTaskProgress } from "../tasks/progress";
  import ErrorSummary from "../ui/ErrorSummary.svelte";
  import TechnicalDetails from "../ui/TechnicalDetails.svelte";
  import { modalFocus } from "../ui/modalFocus";
  import {
    buildResearchProbe,
    chooseResearchCheckout,
    configureResearchCheckout,
    downloadReviewedTesmioSource,
    getResearchBuildProgress,
    getResearchSessionProgress,
    getResearchReportStatus,
    getResearchSourceDownloadProgress,
    getResearchSetup,
    listenForResearchBuildProgress,
    listenForResearchSessionProgress,
    listenForResearchSourceDownloadProgress,
    researchDesktopAvailable,
    launchObservationOnlySession,
    prepareObservationOnlySession,
    setResearchNoticeAccepted,
  } from "./desktopClient";
  import {
    researchBuildProgressView,
    researchDownloadProgressView,
    researchSessionProgressView,
  } from "./progress";
  import type {
    ResearchBuildProgress,
    ResearchSessionProgress,
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
  let sessionProgress = $state<ResearchSessionProgress | null>(null);
  let busy = $state(false);
  let errorMessage = $state("");
  let errorCode = $state("");
  let errorDetail = $state("");
  let stopProgress: (() => void) | null = null;
  let stopDownloadProgress: (() => void) | null = null;
  let stopSessionProgress: (() => void) | null = null;
  let researchContent = $state<HTMLDivElement>();
  let researchResults = $state<HTMLDivElement>();
  const progressView = $derived(
    progress ? researchBuildProgressView(progress, $translation) : null,
  );
  const downloadProgressView = $derived(
    downloadProgress
      ? researchDownloadProgressView(
          downloadProgress,
          $translation,
          (value) => `${formatNumber(value, $activeLocale)} B`,
        )
      : null,
  );
  const buildFailure = $derived(
    progress?.state === "failed" ? describeBuildFailure(progress) : null,
  );
  const sessionProgressView = $derived(
    sessionProgress
      ? researchSessionProgressView(sessionProgress, $translation)
      : null,
  );

  $effect(() => {
    if (!open) {
      stopProgress?.();
      stopProgress = null;
      stopDownloadProgress?.();
      stopDownloadProgress = null;
      stopSessionProgress?.();
      stopSessionProgress = null;
      return;
    }
    untrack(() => {
      void initialise();
    });
    const reportClock = researchDesktopAvailable()
      ? window.setInterval(() => void refreshCheckedReportStatus(), 3_000)
      : undefined;
    return () => {
      if (reportClock !== undefined) window.clearInterval(reportClock);
      stopProgress?.();
      stopProgress = null;
      stopDownloadProgress?.();
      stopDownloadProgress = null;
      stopSessionProgress?.();
      stopSessionProgress = null;
    };
  });

  async function refreshCheckedReportStatus(): Promise<void> {
    if (busy || !status?.session.can_launch) return;
    try {
      const report = await getResearchReportStatus();
      const reportAvailable =
        report.state === "available" || report.state === "warning";
      status = {
        ...status,
        session: {
          ...status.session,
          state: reportAvailable
            ? "report_available"
            : status.session.state === "report_available"
              ? "prepared"
              : status.session.state,
          report_snapshot_count: reportAvailable ? report.snapshot_count : 0,
          report_collection_stage: reportAvailable
            ? report.collection_stage
            : null,
        },
      };
    } catch {
      // The normal command surfaces actionable errors. Background refreshes
      // remain quiet so a temporary read does not replace the current dialog.
    }
  }

  async function initialise(): Promise<void> {
    errorMessage = "";
    errorCode = "";
    errorDetail = "";
    stopProgress?.();
    stopProgress = await observeLatestTaskProgress(
      {
        read: getResearchBuildProgress,
        listen: listenForResearchBuildProgress,
      },
      (latest) => (progress = latest),
      (error) => (errorMessage = describeError(error)),
    );
    stopDownloadProgress = await observeLatestTaskProgress(
      {
        read: getResearchSourceDownloadProgress,
        listen: listenForResearchSourceDownloadProgress,
      },
      (latest) => (downloadProgress = latest),
      (error) => (errorMessage = describeError(error)),
    );
    stopSessionProgress = await observeLatestTaskProgress(
      {
        read: getResearchSessionProgress,
        listen: listenForResearchSessionProgress,
      },
      (latest) => (sessionProgress = latest),
      (error) => (errorMessage = describeError(error)),
    );
    try {
      status = await getResearchSetup();
      progress = status.progress;
      sessionProgress = status.session.progress;
      await tick();
      researchContent?.scrollTo({ top: 0 });
    } catch (error) {
      errorMessage = describeError(error);
    }
  }

  function describeError(error: unknown): string {
    const details = detailsFromError(error);
    errorCode = details.code ?? "unknown";
    errorDetail = details.detail ?? "";
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
    errorDetail = "";
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
    errorDetail = "";
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
      closeBeforeRun: true,
      run: downloadSource,
    });
  }

  async function downloadSource(): Promise<void> {
    busy = true;
    errorMessage = "";
    errorCode = "";
    errorDetail = "";
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
          detail: errorDetail,
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
    errorDetail = "";
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

  function confirmPreparation(): void {
    openRecoveryProposal({
      title: $translation("research-session-prepare-confirm-title"),
      message: $translation("research-session-prepare-confirm-detail"),
      consequence: $translation("research-session-prepare-confirm-safety"),
      actionLabel: $translation("research-session-prepare-confirm-action"),
      closeBeforeRun: true,
      run: prepareSession,
    });
  }

  async function prepareSession(): Promise<void> {
    busy = true;
    errorMessage = "";
    errorCode = "";
    errorDetail = "";
    try {
      status = await prepareObservationOnlySession();
      sessionProgress = status.session.progress;
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-session-prepare-success"),
        tone: "success",
        dedupeKey: "research.session.prepare",
      });
    } catch (error) {
      sessionProgress = await getResearchSessionProgress().catch(
        () => sessionProgress,
      );
      errorMessage = describeError(error);
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-session-error-summary"),
        tone: "error",
        dedupeKey: "research.session.prepare",
        technicalDetails: {
          code: errorCode,
          operation: "prepare_observation_only_session",
          detail: errorDetail,
        },
      });
      throw error;
    } finally {
      busy = false;
    }
  }

  function confirmLaunch(): void {
    openRecoveryProposal({
      title: $translation("research-session-launch-confirm-title"),
      message: $translation("research-session-launch-confirm-detail"),
      consequence: $translation("research-session-launch-confirm-safety"),
      actionLabel: $translation("research-session-launch-confirm-action"),
      closeBeforeRun: true,
      run: launchSession,
    });
  }

  async function launchSession(): Promise<void> {
    busy = true;
    errorMessage = "";
    errorCode = "";
    errorDetail = "";
    try {
      status = await launchObservationOnlySession();
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-session-launch-success"),
        tone: "success",
        dedupeKey: "research.session.launch",
      });
    } catch (error) {
      errorMessage = describeError(error);
      notify({
        title: $translation("research-setup-title"),
        message: $translation("research-session-launch-failure"),
        tone: "error",
        dedupeKey: "research.session.launch",
        technicalDetails: {
          code: errorCode,
          operation: "launch_observation_only_session",
          detail: errorDetail,
        },
      });
      throw error;
    } finally {
      busy = false;
    }
  }

  function sessionStateLabel(): string {
    const waitingStage = status?.session.report_collection_stage;
    const key =
      status?.session.state === "report_available"
        ? status.session.report_snapshot_count > 0
          ? "research-session-state-report"
          : waitingStage === "waiting_for_game_state"
            ? "research-session-state-waiting-game"
            : waitingStage === "waiting_for_loaded_republic"
              ? "research-session-state-waiting-republic"
              : waitingStage === "stopped_at_record_limit"
                ? "research-session-state-limit"
                : "research-session-state-waiting"
        : status?.session.state === "prepared"
          ? "research-session-state-prepared"
          : status?.session.state === "ready_to_prepare"
            ? "research-session-state-ready"
            : status?.session.state === "invalid"
              ? "research-session-state-repair"
              : "research-session-state-required";
    return $translation(key);
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
          <ErrorSummary
            message={errorMessage}
            technicalDetails={{
              code: errorCode,
              operation: "research_setup",
              detail: errorDetail,
            }}
          />
        </div>
      {/if}

      <div class="research-content" bind:this={researchContent}>
        {#if downloadProgressView && downloadProgress?.state !== "idle"}
          <TaskProgressPanel
            view={downloadProgressView}
            headingId="research-download-progress-title"
          />
        {/if}
        {#if sessionProgressView && sessionProgress?.state !== "idle"}
          <TaskProgressPanel
            view={sessionProgressView}
            headingId="research-session-progress-title"
          />
        {/if}
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

          <li
            data-ready={status?.checkout_state === "reviewed" &&
              status?.session.reviewed_loader_source_available}
          >
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
                  (!status?.session.reviewed_loader_source_available ||
                    status?.checkout_state !== "reviewed")
                    ? $translation("research-setup-repair-download")
                    : $translation("research-setup-download-action")}</button
                >
              </div>
              <p class="download-privacy">
                {$translation("research-setup-download-privacy-short")}
              </p>
            </div>
            <strong
              >{status?.checkout_state === "reviewed" &&
              status?.session.reviewed_loader_source_available
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

          <li
            data-ready={status?.session.state === "prepared" ||
              status?.session.state === "report_available"}
          >
            <span>05</span>
            <div>
              <h3>{$translation("research-session-prepare-title")}</h3>
              <p>{$translation("research-session-prepare-detail")}</p>
              <p
                class="session-boundary guidance-surface"
                data-guidance-surface="boundary"
                data-guidance-layout="compact"
                role="note"
              >
                <strong
                  >{$translation(
                    "research-session-save-boundary-title",
                  )}</strong
                >
                {$translation("research-session-save-boundary-detail")}
              </p>
              <button
                type="button"
                class="primary"
                disabled={busy || !status?.session.can_prepare}
                onclick={confirmPreparation}
                >{status?.session.state === "invalid"
                  ? $translation("research-session-repair-action")
                  : $translation("research-session-prepare-action")}</button
              >
              {#if status && !status.session.game_configured}
                <p>{$translation("research-session-game-folder-required")}</p>
              {/if}
              <p class="safe-location">
                {$translation("research-session-managed-folder")}
                <code>{status?.session.managed_folder}</code>
              </p>
            </div>
            <strong>{sessionStateLabel()}</strong>
          </li>

          <li data-ready={status?.session.state === "report_available"}>
            <span>06</span>
            <div>
              <h3>{$translation("research-session-launch-title")}</h3>
              <p>{$translation("research-session-launch-detail")}</p>
              <button
                type="button"
                class="primary"
                disabled={busy || !status?.session.can_launch}
                onclick={confirmLaunch}
                >{$translation("research-session-launch-action")}</button
              >
            </div>
            <strong
              >{status?.session.state === "report_available"
                ? status.session.report_snapshot_count > 0
                  ? $translation("research-session-state-report")
                  : sessionStateLabel()
                : status?.session.can_launch
                  ? $translation("research-session-state-launch-ready")
                  : $translation("research-session-state-required")}</strong
            >
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
