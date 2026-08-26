<script lang="ts">
  import type { TranslationKey } from "../i18n/catalog";
  import { translation } from "../i18n/runtime";
  import { modalFocus } from "../ui/modalFocus";
  import {
    chooseDirectory,
    configureDirectory,
    getSetupState,
    observeLatestSave,
    observerErrorCode,
  } from "./desktopClient";
  import type {
    DirectoryKind,
    ObserverErrorCode,
    ReceiverDataset,
    SetupState,
  } from "./types";

  let {
    open,
    desktopAvailable,
    setup,
    dataset,
    onclose,
    onsetupchange,
    onobservation,
  }: {
    open: boolean;
    desktopAvailable: boolean;
    setup: SetupState | null;
    dataset: ReceiverDataset | null;
    onclose: () => void;
    onsetupchange: (setup: SetupState) => void;
    onobservation: (dataset: ReceiverDataset) => void;
  } = $props();

  let busy = $state(false);
  let errorMessage = $state("");
  let statusMessage = $state("");

  const errorKeys: Record<ObserverErrorCode, TranslationKey> = {
    invalid_directory: "error-observer-invalid-directory",
    invalid_game_directory: "error-observer-invalid-game-directory",
    save_directory_not_configured:
      "error-observer-save-directory-not-configured",
    no_save_candidate: "error-observer-no-save-candidate",
    invalid_save_candidate: "error-observer-invalid-save-candidate",
    save_changed_during_read: "error-observer-save-changed",
    invalid_archive: "error-observer-invalid-archive",
    missing_stats_payload: "error-observer-missing-stats",
    duplicate_stats_payload: "error-observer-duplicate-stats",
    stats_payload_too_large: "error-observer-stats-too-large",
    invalid_stats_encoding: "error-observer-invalid-encoding",
    stats_line_too_long: "error-observer-line-too-long",
    unsupported_stats_format: "error-observer-unsupported-format",
    malformed_receiver_history: "error-observer-malformed-history",
    receiver_history_unavailable: "error-observer-receiver-unavailable",
    storage_unavailable: "error-observer-storage-unavailable",
    unknown: "error-observer-unknown",
  };

  function reportError(error: unknown): void {
    errorMessage = $translation(errorKeys[observerErrorCode(error)]);
    statusMessage = "";
  }

  async function selectDirectory(kind: DirectoryKind): Promise<void> {
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      const title = $translation(
        kind === "save"
          ? "observer-choose-save-folder"
          : "observer-choose-game-folder",
      );
      const selected = await chooseDirectory(title);
      if (!selected) return;
      onsetupchange(await configureDirectory(kind, selected));
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function observe(): Promise<void> {
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      const result = await observeLatestSave();
      onobservation(result.dataset);
      onsetupchange(await getSetupState());
      statusMessage = $translation(
        result.outcome === "duplicate"
          ? "observer-duplicate"
          : "observer-imported",
      );
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }
</script>

{#if open}
  <div class="language-backdrop observer-backdrop">
    <dialog
      use:modalFocus={{ onclose, closeDisabled: busy }}
      open
      class="language-dialog observer-dialog"
      aria-modal="true"
      aria-labelledby="observer-title"
      aria-describedby="observer-introduction"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("observer-eyebrow")}</span>
          <h2 id="observer-title">{$translation("observer-title")}</h2>
        </div>
        <button
          data-modal-autofocus
          class="language-close"
          type="button"
          aria-label={$translation("action-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <p id="observer-introduction">
        {$translation("observer-introduction")}
      </p>

      {#if !desktopAvailable}
        <section class="observer-browser-state">
          <strong>{$translation("observer-browser-title")}</strong>
          <p>{$translation("observer-browser-detail")}</p>
        </section>
      {:else}
        <div class="observer-source-grid">
          <article class:configured={Boolean(setup?.save_directory)}>
            <header>
              <div>
                <span class="eyebrow">01</span>
                <h3>{$translation("observer-save-folder")}</h3>
              </div>
              <span
                class="status-chip"
                data-status={setup?.save_directory ? "stable" : "watch"}
              >
                {setup?.save_directory
                  ? $translation("coverage-complete")
                  : $translation("observer-not-configured")}
              </span>
            </header>
            <p>{$translation("observer-save-folder-detail")}</p>
            <div class="observer-source-state">
              <strong>
                {setup?.save_directory
                  ? $translation("observer-folder-selected", {
                      name: setup.save_directory.name,
                    })
                  : $translation("observer-not-configured")}
              </strong>
              <span>
                {$translation("observer-save-candidates", {
                  count: setup?.save_candidates ?? 0,
                })}
              </span>
            </div>
            <button
              type="button"
              disabled={busy}
              onclick={() => void selectDirectory("save")}
            >
              {$translation("observer-choose-save-folder")}
            </button>
          </article>

          <article class:configured={Boolean(setup?.game_directory)}>
            <header>
              <div>
                <span class="eyebrow">02</span>
                <h3>{$translation("observer-game-folder")}</h3>
              </div>
              <span
                class="status-chip"
                data-status={setup?.game_directory ? "stable" : "watch"}
              >
                {setup?.game_directory
                  ? $translation("coverage-complete")
                  : $translation("observer-not-configured")}
              </span>
            </header>
            <p>{$translation("observer-game-folder-detail")}</p>
            <div class="observer-source-state">
              <strong>
                {setup?.game_directory
                  ? $translation("observer-folder-selected", {
                      name: setup.game_directory.name,
                    })
                  : $translation("observer-not-configured")}
              </strong>
              <span>
                {$translation("observer-game-vocabularies", {
                  count: setup?.game_vocabularies.length ?? 0,
                })}
              </span>
            </div>
            <button
              type="button"
              disabled={busy}
              onclick={() => void selectDirectory("game")}
            >
              {$translation("observer-choose-game-folder")}
            </button>
          </article>
        </div>

        <aside class="observer-boundary">
          <strong>{$translation("save-safety-observer-boundary")}</strong>
          <span>{$translation("save-safety-private-paths")}</span>
          <span>{$translation("coverage-game-vocabulary-identities")}</span>
        </aside>

        {#if errorMessage}<p class="language-error" role="alert">
            {errorMessage}
          </p>{/if}
        {#if statusMessage}<p class="language-status" role="status">
            {statusMessage}
          </p>{/if}

        <footer class="observer-footer">
          <div>
            <strong>
              {dataset?.source_file_name ??
                $translation("observer-no-observation")}
            </strong>
            {#if dataset}
              <span>
                {$translation("coverage-receiver-records", {
                  chartable: dataset.coverage.chartable_records,
                  history: dataset.coverage.history_records,
                  dropped: dataset.coverage.dropped_records,
                })}
              </span>
            {/if}
          </div>
          <button
            type="button"
            class="observer-primary-action"
            disabled={busy || !setup?.save_directory}
            onclick={() => void observe()}
          >
            {busy
              ? $translation("observer-observing")
              : $translation("observer-observe-latest")}
          </button>
        </footer>
      {/if}
    </dialog>
  </div>
{/if}
