<script lang="ts">
  import type { TranslationKey } from "../i18n/catalog";
  import { translation } from "../i18n/runtime";
  import { modalFocus } from "../ui/modalFocus";
  import TaskProgressPanel from "../tasks/TaskProgressPanel.svelte";
  import { reinterpretationProgressView } from "../tasks/reinterpretationProgress";
  import {
    chooseDirectory,
    configureDirectory,
    createLocalCompatibilityOverride,
    getSetupState,
    observeLatestSave,
    observerErrorCode,
    reinterpretLatestSave,
    reloadLocalCompatibilityOverride,
    setAutomaticObservation,
  } from "./desktopClient";
  import type {
    CompatibilityCatalogueScopeStatus,
    DirectoryKind,
    ObserverErrorCode,
    ReceiverDataset,
    ReinterpretationProgress,
    SetupState,
  } from "./types";

  let {
    open,
    desktopAvailable,
    setup,
    dataset,
    reinterpretationProgress,
    onclose,
    onsetupchange,
    onobservation,
  }: {
    open: boolean;
    desktopAvailable: boolean;
    setup: SetupState | null;
    dataset: ReceiverDataset | null;
    reinterpretationProgress: ReinterpretationProgress | null;
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
    malformed_snapshot: "error-observer-malformed-snapshot",
    receiver_history_unavailable: "error-observer-receiver-unavailable",
    storage_unavailable: "error-observer-storage-unavailable",
    unknown_branch: "error-observer-unknown-branch",
    incompatible_comparison: "error-observer-incompatible-comparison",
    same_observation_comparison: "error-observer-same-comparison",
    unknown_observation: "error-observer-unknown-observation",
    warehouse_write_limit: "error-observer-warehouse-write-limit",
    invalid_compatibility_profile: "error-observer-invalid-compatibility",
    binary_compatibility_mismatch: "error-observer-binary-compatibility",
    critical_task_busy: "error-observer-critical-task-busy",
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
          : kind === "game"
            ? "observer-choose-game-folder"
            : "observer-choose-workshop-folder",
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

  async function toggleAutomaticObservation(enabled: boolean): Promise<void> {
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      onsetupchange(await setAutomaticObservation(enabled));
      statusMessage = $translation(
        enabled ? "observer-automatic-enabled" : "observer-automatic-disabled",
      );
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function createCompatibilityOverride(): Promise<void> {
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      const update = await createLocalCompatibilityOverride();
      if (setup) onsetupchange({ ...setup, compatibility: update.status });
      statusMessage = $translation("compatibility-created");
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function reloadCompatibility(): Promise<void> {
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      const update = await reloadLocalCompatibilityOverride();
      if (setup) onsetupchange({ ...setup, compatibility: update.status });
      statusMessage = $translation("compatibility-reloaded");
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function reinterpret(): Promise<void> {
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      const result = await reinterpretLatestSave();
      onobservation(result.dataset);
      onsetupchange(await getSetupState());
      statusMessage = $translation("compatibility-reinterpreted");
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  function compatibilityBadge(): string {
    return setup?.compatibility.active.mapping_classification ===
      "player_mapped"
      ? $translation("compatibility-player-mapped")
      : $translation("compatibility-reviewed");
  }

  function validationLabel(): string {
    const validation = setup?.compatibility.local_validation ?? "missing";
    return $translation(
      validation === "valid"
        ? "compatibility-validation-valid"
        : validation === "invalid"
          ? "compatibility-validation-invalid"
          : "compatibility-validation-missing",
    );
  }

  function scopeStateLabel(
    state: CompatibilityCatalogueScopeStatus["state"],
  ): string {
    switch (state) {
      case "matched":
        return $translation("compatibility-scope-matched");
      case "dormant":
        return $translation("compatibility-scope-dormant");
      case "updated_unreviewed":
        return $translation("compatibility-scope-updated");
      case "conflict":
        return $translation("compatibility-scope-conflict");
    }
  }

  function scopeGuidance(
    state: CompatibilityCatalogueScopeStatus["state"],
  ): string {
    switch (state) {
      case "matched":
        return $translation("compatibility-scope-guidance-matched");
      case "dormant":
        return $translation("compatibility-scope-guidance-dormant");
      case "updated_unreviewed":
        return $translation("compatibility-scope-guidance-updated");
      case "conflict":
        return $translation("compatibility-scope-guidance-conflict");
    }
  }

  function scopeDataStatus(
    state: CompatibilityCatalogueScopeStatus["state"],
  ): "stable" | "watch" | "exposed" | "neutral" {
    if (state === "matched") return "stable";
    if (state === "conflict") return "exposed";
    if (state === "updated_unreviewed") return "watch";
    return "neutral";
  }

  function automaticStatusText(): string {
    const observer = setup?.automatic_observer;
    if (!observer) return $translation("observer-automatic-disabled");
    switch (observer.phase) {
      case "disabled":
        return $translation("observer-automatic-disabled");
      case "not_configured":
        return $translation("observer-automatic-needs-folder");
      case "waiting_for_stability":
        return $translation("observer-automatic-waiting", {
          file: observer.candidate_file_name ?? "—",
        });
      case "retrying":
        return $translation("observer-automatic-retrying", {
          file: observer.candidate_file_name ?? "—",
          attempt: observer.retry_attempt,
        });
      case "observed":
        return $translation("observer-automatic-observed", {
          file: observer.last_observed_file_name ?? "—",
        });
      case "failed":
        return $translation("observer-automatic-failed", {
          file: observer.candidate_file_name ?? "—",
        });
      default:
        return $translation("observer-automatic-watching");
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

          <article class:configured={Boolean(setup?.workshop_directory)}>
            <header>
              <div>
                <span class="eyebrow">03</span>
                <h3>{$translation("observer-workshop-folder")}</h3>
              </div>
              <span
                class="status-chip"
                data-status={setup?.workshop_directory ? "stable" : "watch"}
              >
                {setup?.workshop_directory
                  ? $translation("coverage-complete")
                  : $translation("observer-optional")}
              </span>
            </header>
            <p>{$translation("observer-workshop-folder-detail")}</p>
            <div class="observer-source-state">
              <strong>
                {setup?.workshop_directory
                  ? $translation("observer-folder-selected", {
                      name: setup.workshop_directory.name,
                    })
                  : $translation("observer-automatic-discovery")}
              </strong>
              <span>{$translation("observer-workshop-private")}</span>
            </div>
            <button
              type="button"
              disabled={busy}
              onclick={() => void selectDirectory("workshop")}
            >
              {$translation("observer-choose-workshop-folder")}
            </button>
          </article>
        </div>

        <section class="observer-automatic-card">
          <div>
            <span class="eyebrow"
              >{$translation("observer-automatic-eyebrow")}</span
            >
            <strong>{$translation("observer-automatic-title")}</strong>
            <p>{$translation("observer-automatic-detail")}</p>
            <small role="status">{automaticStatusText()}</small>
          </div>
          <label class="observer-switch">
            <input
              type="checkbox"
              checked={setup?.automatic_observer.enabled ?? false}
              disabled={busy || !setup?.save_directory}
              onchange={(event) =>
                void toggleAutomaticObservation(event.currentTarget.checked)}
            />
            <span>{$translation("observer-automatic-toggle")}</span>
          </label>
        </section>

        {#if setup?.compatibility}
          <section
            class="compatibility-card"
            aria-labelledby="compatibility-title"
          >
            <header>
              <div>
                <span class="eyebrow"
                  >{$translation("compatibility-eyebrow")}</span
                >
                <h3 id="compatibility-title">
                  {$translation("compatibility-title")}
                </h3>
              </div>
              <span
                class="status-chip"
                data-status={setup.compatibility.active
                  .mapping_classification === "player_mapped"
                  ? "watch"
                  : "stable"}>{compatibilityBadge()}</span
              >
            </header>
            <p>{$translation("compatibility-description")}</p>
            <div class="compatibility-facts">
              <div>
                <span>{$translation("compatibility-active-profile")}</span>
                <strong>{setup.compatibility.active.id}</strong>
                <small
                  >{$translation("compatibility-version-hash", {
                    version: setup.compatibility.active.version,
                    hash: setup.compatibility.active.resolved_hash.slice(0, 12),
                  })}</small
                >
              </div>
              <div>
                <span>{$translation("compatibility-detected")}</span>
                <strong
                  >{setup.compatibility.detected_game_version ??
                    setup.compatibility.detected_build_id ??
                    $translation("compatibility-target-unknown")}</strong
                >
                <small
                  >{setup.compatibility.active.target_game_versions.join(
                    ", ",
                  ) || $translation("compatibility-target-unknown")}</small
                >
              </div>
              <div>
                <span>{$translation("compatibility-base")}</span>
                <strong
                  >{setup.compatibility.active.base_profile_id ??
                    setup.compatibility.reviewed_base.id}</strong
                >
                <small
                  >{setup.compatibility.active.base_profile_hash?.slice(
                    0,
                    12,
                  ) ??
                    setup.compatibility.reviewed_base.content_hash.slice(
                      0,
                      12,
                    )}</small
                >
              </div>
              <div>
                <span>{$translation("compatibility-local-file")}</span>
                <strong>{validationLabel()}</strong>
                <small title={setup.compatibility.local_file_path}
                  >{setup.compatibility.local_file_path}</small
                >
              </div>
            </div>
            <p class="compatibility-coverage">
              {$translation("compatibility-coverage", {
                markers: setup.compatibility.coverage.stats_markers,
                fields: setup.compatibility.coverage.stats_fields,
                operations: setup.compatibility.coverage.definition_operations,
                layouts: setup.compatibility.coverage.binary_layouts,
                scopes: setup.compatibility.coverage.catalogue_scopes,
              })}
            </p>
            {#if setup.compatibility.catalogue_scopes.length}
              <section
                class="compatibility-scopes"
                aria-labelledby="compatibility-scopes-title"
              >
                <header>
                  <div>
                    <span class="eyebrow"
                      >{$translation("compatibility-scopes-eyebrow")}</span
                    >
                    <h4 id="compatibility-scopes-title">
                      {$translation("compatibility-scopes-title")}
                    </h4>
                  </div>
                  <span>{setup.compatibility.catalogue_scopes.length}</span>
                </header>
                <div class="compatibility-scope-list">
                  {#each setup.compatibility.catalogue_scopes as scope}
                    <article data-status={scopeDataStatus(scope.state)}>
                      <header>
                        <div>
                          <strong>{scope.package_name ?? scope.id}</strong>
                          <code>{scope.source_id}</code>
                        </div>
                        <span
                          class="status-chip"
                          data-status={scopeDataStatus(scope.state)}
                          >{scopeStateLabel(scope.state)}</span
                        >
                      </header>
                      <p>{scopeGuidance(scope.state)}</p>
                      <small
                        >{$translation("compatibility-scope-evidence", {
                          policy:
                            scope.update_policy === "exact"
                              ? $translation("compatibility-policy-exact")
                              : $translation("compatibility-policy-track"),
                          mappings: scope.mapping_count,
                          acknowledged: scope.acknowledged_content_hash.slice(
                            0,
                            12,
                          ),
                          current:
                            scope.current_content_hash?.slice(0, 12) ?? "—",
                        })}</small
                      >
                    </article>
                  {/each}
                </div>
              </section>
            {/if}
            {#if setup.compatibility.last_validation_error}
              <p class="compatibility-warning" role="alert">
                {validationLabel()} · {setup.compatibility
                  .last_validation_error}
              </p>
            {/if}
            <p class="compatibility-boundary">
              {$translation("compatibility-no-editor")}
            </p>
            <div class="compatibility-actions">
              <button
                type="button"
                disabled={busy || setup.compatibility.local_file_exists}
                onclick={() => void createCompatibilityOverride()}
                >{$translation("compatibility-create")}</button
              >
              <button
                type="button"
                disabled={busy || !setup.compatibility.local_file_exists}
                onclick={() => void reloadCompatibility()}
                >{$translation("compatibility-reload")}</button
              >
              <button
                type="button"
                disabled={busy || !setup.save_directory}
                onclick={() => void reinterpret()}
                >{$translation("compatibility-reinterpret")}</button
              >
            </div>
            {#if reinterpretationProgress && reinterpretationProgress.phase !== "idle"}
              <div class="compatibility-progress">
                <TaskProgressPanel
                  view={reinterpretationProgressView(
                    reinterpretationProgress,
                    $translation,
                  )}
                  headingId="compatibility-reinterpretation-title"
                />
              </div>
            {/if}
          </section>
        {/if}

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
