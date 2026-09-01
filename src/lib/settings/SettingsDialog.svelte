<script lang="ts">
  import { sourceLanguagePack } from "../i18n/catalog";
  import {
    activeLanguagePackId,
    activeLocale,
    translation,
  } from "../i18n/runtime";
  import { activeTheme } from "../theme/runtime";
  import { notify, openRecoveryProposal } from "../notifications/service";
  import {
    chooseAndConfigureDirectory,
    rebuildWarehouse,
  } from "../observations/desktopClient";
  import type { DirectoryKind, SetupState } from "../observations/types";
  import { noteAllAttentionCuesReplayed } from "../attention/service";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import { modalFocus } from "../ui/modalFocus";
  import {
    applicationSettingsHostAvailable,
    getApplicationSettings,
    replayAllAttentionCues,
    resetApplicationPreferences,
    updateApplicationPreferences,
  } from "./desktopClient";
  import { applyApplicationPreferences } from "./runtime";
  import type {
    ApplicationPreferences,
    ApplicationPreferencesDraft,
    ApplicationSettingsView,
  } from "./types";

  let {
    open,
    active = true,
    layer = 0,
    setup,
    onclose,
    onsetupchange,
    onopenlanguage,
    onopentheme,
    onopenobserver,
    onopenlegal,
    onopendiagnostics,
  }: {
    open: boolean;
    active?: boolean;
    layer?: number;
    setup: SetupState | null;
    onclose: () => void;
    onsetupchange: (setup: SetupState) => void;
    onopenlanguage: () => void;
    onopentheme: () => void;
    onopenobserver: () => void;
    onopenlegal: () => void;
    onopendiagnostics: () => void;
  } = $props();

  const desktopAvailable = applicationSettingsHostAvailable();
  let view = $state<ApplicationSettingsView | null>(null);
  let draft = $state<ApplicationPreferencesDraft | null>(null);
  let busy = $state(false);
  let errorMessage = $state("");
  let statusMessage = $state("");
  let loadedForOpen = $state(false);
  let rebuildConfirmationOpen = $state(false);

  function fallbackPreferences(): ApplicationPreferences {
    return {
      schema_version: 2,
      storage_patience_preset: "balanced",
      custom_storage_patience_seconds: null,
      effective_storage_patience_seconds: 60,
      background_work_priority: "gentle",
      text_scale_percent: 100,
      motion_preference: "system",
      wording_mode: "player_friendly",
      automatic_observation_enabled: setup?.automatic_observer.enabled ?? false,
    };
  }

  const currentSetup = $derived(view?.setup ?? setup);
  const hasUnsavedChanges = $derived(
    draft !== null &&
      view !== null &&
      JSON.stringify(draft) !==
        JSON.stringify({
          storage_patience_preset: view.preferences.storage_patience_preset,
          custom_storage_patience_seconds:
            view.preferences.custom_storage_patience_seconds,
          background_work_priority: view.preferences.background_work_priority,
          text_scale_percent: view.preferences.text_scale_percent,
          motion_preference: view.preferences.motion_preference,
          wording_mode: view.preferences.wording_mode,
          automatic_observation_enabled:
            view.preferences.automatic_observation_enabled,
        }),
  );

  function draftFrom(
    preferences: ApplicationPreferences,
  ): ApplicationPreferencesDraft {
    return {
      storage_patience_preset: preferences.storage_patience_preset,
      custom_storage_patience_seconds:
        preferences.custom_storage_patience_seconds,
      background_work_priority: preferences.background_work_priority,
      text_scale_percent: preferences.text_scale_percent,
      motion_preference: preferences.motion_preference,
      wording_mode: preferences.wording_mode,
      automatic_observation_enabled: preferences.automatic_observation_enabled,
    };
  }

  function accept(next: ApplicationSettingsView): void {
    view = next;
    draft = draftFrom(next.preferences);
    onsetupchange(next.setup);
    applyApplicationPreferences(next.preferences);
  }

  async function load(): Promise<void> {
    errorMessage = "";
    statusMessage = "";
    if (!desktopAvailable) {
      const preferences = fallbackPreferences();
      view = {
        preferences,
        setup: setup ?? ({} as SetupState),
        maintenance: {
          market_storage_contract_version: 2,
          cached_market_records: 2805,
          cached_market_fact_rows: 118899,
          market_interpretation_memberships: 7200,
          latest_indexing_phase: "complete",
          latest_cache_records_reused: 2600,
          latest_cache_rows_avoided: 103000,
          latest_contention_retries: 2,
          latest_contention_wait_ms: 410,
          latest_resume_count: 1,
        },
      };
      draft = draftFrom(preferences);
      return;
    }
    busy = true;
    try {
      accept(await getApplicationSettings());
    } catch {
      errorMessage = $translation("settings-error-unavailable");
    } finally {
      busy = false;
    }
  }

  async function save(): Promise<void> {
    if (!draft || busy || !desktopAvailable) return;
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      accept(await updateApplicationPreferences(draft));
      statusMessage = $translation("settings-saved");
      notify({
        title: $translation("settings-title"),
        message: statusMessage,
        tone: "success",
        dedupeKey: "settings-saved",
      });
    } catch {
      errorMessage = $translation("settings-error-invalid");
    } finally {
      busy = false;
    }
  }

  async function restoreDefaults(): Promise<void> {
    if (busy || !desktopAvailable) return;
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      accept(await resetApplicationPreferences());
      statusMessage = $translation("settings-defaults-restored");
    } catch {
      errorMessage = $translation("settings-error-unavailable");
    } finally {
      busy = false;
    }
  }

  async function selectDirectory(kind: DirectoryKind): Promise<void> {
    if (busy || !desktopAvailable) return;
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
      const nextSetup = await chooseAndConfigureDirectory(kind, title);
      onsetupchange(nextSetup);
      if (view) view = { ...view, setup: nextSetup };
      statusMessage = $translation("settings-source-updated");
    } catch {
      errorMessage = $translation("settings-error-directory");
    } finally {
      busy = false;
    }
  }

  async function replayGuidance(): Promise<void> {
    if (busy || !desktopAvailable) return;
    busy = true;
    errorMessage = "";
    try {
      await replayAllAttentionCues();
      noteAllAttentionCuesReplayed();
      statusMessage = $translation("settings-guidance-replayed");
    } catch {
      errorMessage = $translation("settings-error-unavailable");
    } finally {
      busy = false;
    }
  }

  async function rebuildAnalyticalWarehouse(): Promise<void> {
    if (busy || !desktopAvailable || !rebuildConfirmationOpen) return;
    busy = true;
    errorMessage = "";
    statusMessage = "";
    try {
      await rebuildWarehouse();
      rebuildConfirmationOpen = false;
      statusMessage = $translation("settings-rebuild-queued");
      notify({
        title: $translation("settings-maintenance-title"),
        message: statusMessage,
        tone: "success",
        dedupeKey: "warehouse-rebuild-queued",
      });
    } catch {
      errorMessage = $translation("settings-rebuild-error");
    } finally {
      busy = false;
    }
  }

  function discardDraft(): void {
    if (view) draft = draftFrom(view.preferences);
  }

  function confirmDiscard(run: () => void): void {
    openRecoveryProposal({
      title: $translation("settings-discard-title"),
      message: $translation("settings-discard-message"),
      consequence: $translation("settings-discard-safety"),
      actionLabel: $translation("settings-discard-action"),
      run: () => {
        discardDraft();
        run();
      },
    });
  }

  function requestClose(): void {
    if (busy) return;
    if (hasUnsavedChanges) {
      confirmDiscard(onclose);
      return;
    }
    onclose();
  }

  function openRelated(action: () => void): void {
    if (hasUnsavedChanges) {
      confirmDiscard(action);
      return;
    }
    action();
  }

  $effect(() => {
    if (open && !loadedForOpen) {
      loadedForOpen = true;
      void load();
    } else if (!open) {
      loadedForOpen = false;
    }
  });
</script>

{#if open}
  <div
    class="settings-backdrop"
    inert={!active}
    aria-hidden={!active}
    data-dialog-active={active}
    style:z-index={300 + layer}
  >
    <dialog
      use:modalFocus={{ onclose: requestClose, closeDisabled: busy, active }}
      open
      class="settings-dialog"
      aria-modal={active}
      aria-labelledby="settings-title"
      aria-describedby="settings-introduction"
    >
      <header class="settings-header">
        <div>
          <span class="eyebrow">{$translation("settings-eyebrow")}</span>
          <h2 id="settings-title">{$translation("settings-title")}</h2>
          <p id="settings-introduction">
            {$translation("settings-introduction")}
          </p>
        </div>
        <button
          class="dialog-close"
          type="button"
          disabled={busy}
          aria-label={$translation("action-close")}
          onclick={requestClose}>×</button
        >
      </header>

      {#if !desktopAvailable}
        <GuidanceSurface kind="instruction" layout="compact">
          <strong>{$translation("settings-desktop-required")}</strong>
          <p>{$translation("settings-desktop-required-detail")}</p>
        </GuidanceSurface>
      {/if}

      {#if errorMessage}
        <p class="settings-error" role="alert">{errorMessage}</p>
      {/if}
      {#if statusMessage}
        <p class="settings-status" role="status">{statusMessage}</p>
      {/if}

      <div class="settings-content" aria-busy={busy}>
        <section aria-labelledby="settings-sources-title">
          <div class="section-heading">
            <div>
              <span class="eyebrow"
                >{$translation("settings-sources-eyebrow")}</span
              >
              <h3 id="settings-sources-title">
                {$translation("settings-sources-title")}
              </h3>
            </div>
            <button type="button" onclick={() => openRelated(onopenobserver)}>
              {$translation("settings-open-observer")}
            </button>
          </div>

          <div class="source-grid" data-aligned-action-group="source-folders">
            <article data-aligned-action-item>
              <span>{$translation("observer-save-folder")}</span>
              <strong
                >{currentSetup?.save_directory?.name ??
                  $translation("observer-not-configured")}</strong
              >
              <small>{$translation("observer-save-folder-detail")}</small>
              <button
                data-aligned-action
                type="button"
                disabled={busy || !desktopAvailable}
                onclick={() => selectDirectory("save")}
                >{$translation("observer-choose-save-folder")}</button
              >
            </article>
            <article data-aligned-action-item>
              <span>{$translation("observer-game-folder")}</span>
              <strong
                >{currentSetup?.game_directory?.name ??
                  $translation("observer-not-configured")}</strong
              >
              <small>{$translation("observer-game-folder-detail")}</small>
              <button
                data-aligned-action
                type="button"
                disabled={busy || !desktopAvailable}
                onclick={() => selectDirectory("game")}
                >{$translation("observer-choose-game-folder")}</button
              >
            </article>
            <article data-aligned-action-item>
              <span>{$translation("observer-workshop-folder")}</span>
              <strong
                >{currentSetup?.workshop_directory?.name ??
                  $translation("observer-automatic-discovery")}</strong
              >
              <small>{$translation("observer-workshop-private")}</small>
              <button
                data-aligned-action
                type="button"
                disabled={busy || !desktopAvailable}
                onclick={() => selectDirectory("workshop")}
                >{$translation("observer-choose-workshop-folder")}</button
              >
            </article>
          </div>

          {#if draft}
            <label class="setting-row setting-toggle">
              <span>
                <strong>{$translation("observer-automatic-title")}</strong>
                <small>{$translation("observer-automatic-detail")}</small>
              </span>
              <input
                type="checkbox"
                bind:checked={draft.automatic_observation_enabled}
                disabled={!currentSetup?.save_directory}
              />
            </label>
          {/if}
        </section>

        <section aria-labelledby="settings-appearance-title">
          <div class="section-heading">
            <div>
              <span class="eyebrow"
                >{$translation("settings-appearance-eyebrow")}</span
              >
              <h3 id="settings-appearance-title">
                {$translation("settings-appearance-title")}
              </h3>
            </div>
          </div>

          <div class="related-grid">
            <button type="button" onclick={() => openRelated(onopenlanguage)}>
              <span>{$translation("settings-language")}</span>
              <strong>{$activeLocale}</strong>
            </button>
            <button type="button" onclick={() => openRelated(onopentheme)}>
              <span>{$translation("settings-theme")}</span>
              <strong
                >{$activeTheme?.name ??
                  $translation("theme-classic-name")}</strong
              >
            </button>
          </div>

          {#if draft}
            <div class="setting-grid">
              <label class="setting-row">
                <span>
                  <strong>{$translation("settings-text-scale")}</strong>
                  <small>{$translation("settings-text-scale-detail")}</small>
                </span>
                <select bind:value={draft.text_scale_percent}>
                  {#each [100, 125, 150, 175, 200] as scale}
                    <option value={scale}>{scale}%</option>
                  {/each}
                </select>
              </label>
              <label class="setting-row">
                <span>
                  <strong>{$translation("settings-motion")}</strong>
                  <small>{$translation("settings-motion-detail")}</small>
                </span>
                <select bind:value={draft.motion_preference}>
                  <option value="system"
                    >{$translation("settings-motion-system")}</option
                  >
                  <option value="reduced"
                    >{$translation("settings-motion-reduced")}</option
                  >
                </select>
              </label>
              <label class="setting-row">
                <span>
                  <strong>{$translation("settings-wording-style")}</strong>
                  <small>{$translation("settings-wording-style-detail")}</small>
                </span>
                <select bind:value={draft.wording_mode}>
                  <option value="player_friendly"
                    >{$translation("settings-wording-player")}</option
                  >
                  <option value="technical"
                    >{$translation("settings-wording-technical")}</option
                  >
                </select>
              </label>
            </div>
            {#if $activeLanguagePackId !== sourceLanguagePack.id}
              <GuidanceSurface kind="help" layout="compact">
                <p>{$translation("settings-wording-community-fallback")}</p>
              </GuidanceSurface>
            {/if}
          {/if}
        </section>

        <section aria-labelledby="settings-background-title">
          <div class="section-heading">
            <div>
              <span class="eyebrow"
                >{$translation("settings-background-eyebrow")}</span
              >
              <h3 id="settings-background-title">
                {$translation("settings-background-title")}
              </h3>
            </div>
          </div>
          <GuidanceSurface kind="help" layout="compact">
            <strong>{$translation("settings-recorder-priority-title")}</strong>
            <p>{$translation("settings-recorder-priority-detail")}</p>
          </GuidanceSurface>

          {#if draft}
            <div class="setting-grid">
              <label class="setting-row">
                <span>
                  <strong>{$translation("settings-storage-patience")}</strong>
                  <small
                    >{$translation("settings-storage-patience-detail")}</small
                  >
                </span>
                <select bind:value={draft.storage_patience_preset}>
                  <option value="short"
                    >{$translation("settings-patience-short")}</option
                  >
                  <option value="balanced"
                    >{$translation("settings-patience-balanced")}</option
                  >
                  <option value="patient"
                    >{$translation("settings-patience-patient")}</option
                  >
                  <option value="custom"
                    >{$translation("settings-patience-custom")}</option
                  >
                </select>
              </label>
              {#if draft.storage_patience_preset === "custom"}
                <label class="setting-row custom-timeout">
                  <span>
                    <strong>{$translation("settings-custom-seconds")}</strong>
                    <small
                      >{$translation("settings-custom-seconds-detail")}</small
                    >
                  </span>
                  <input
                    type="number"
                    min="5"
                    max="300"
                    step="1"
                    bind:value={draft.custom_storage_patience_seconds}
                    required
                  />
                </label>
              {/if}
              <label class="setting-row">
                <span>
                  <strong>{$translation("settings-background-priority")}</strong
                  >
                  <small
                    >{$translation(
                      "settings-background-priority-detail",
                    )}</small
                  >
                </span>
                <select bind:value={draft.background_work_priority}>
                  <option value="gentle"
                    >{$translation("settings-priority-gentle")}</option
                  >
                  <option value="balanced"
                    >{$translation("settings-priority-balanced")}</option
                  >
                  <option value="finish_sooner"
                    >{$translation("settings-priority-faster")}</option
                  >
                </select>
              </label>
            </div>
          {/if}

          <div class="maintenance-heading">
            <div>
              <span class="eyebrow"
                >{$translation("settings-maintenance-eyebrow")}</span
              >
              <h4>{$translation("settings-maintenance-title")}</h4>
            </div>
            <button
              type="button"
              disabled={busy || !desktopAvailable}
              onclick={() => (rebuildConfirmationOpen = true)}
            >
              {$translation("settings-rebuild-action")}
            </button>
          </div>

          {#if view}
            <div class="maintenance-grid">
              <div>
                <span>{$translation("settings-cache-records")}</span>
                <strong>{view.maintenance.cached_market_records}</strong>
              </div>
              <div>
                <span>{$translation("settings-cache-rows")}</span>
                <strong>{view.maintenance.cached_market_fact_rows}</strong>
              </div>
              <div>
                <span>{$translation("settings-cache-memberships")}</span>
                <strong
                  >{view.maintenance.market_interpretation_memberships}</strong
                >
              </div>
              <div>
                <span>{$translation("settings-cache-last-reuse")}</span>
                <strong>{view.maintenance.latest_cache_rows_avoided}</strong>
              </div>
            </div>
            <p class="maintenance-detail">
              {$translation("settings-cache-detail", {
                contract: view.maintenance.market_storage_contract_version,
                retries: view.maintenance.latest_contention_retries,
                seconds: Math.round(
                  view.maintenance.latest_contention_wait_ms / 1000,
                ),
              })}
            </p>
          {/if}

          {#if rebuildConfirmationOpen}
            <GuidanceSurface kind="instruction" layout="compact">
              <strong>{$translation("settings-rebuild-confirm-title")}</strong>
              <p>{$translation("settings-rebuild-confirm-detail")}</p>
              <div class="confirmation-actions">
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => (rebuildConfirmationOpen = false)}
                  >{$translation("action-cancel")}</button
                >
                <button
                  type="button"
                  class="primary-action"
                  disabled={busy || !desktopAvailable}
                  onclick={rebuildAnalyticalWarehouse}
                  >{$translation("settings-rebuild-confirm-action")}</button
                >
              </div>
            </GuidanceSurface>
          {/if}
        </section>

        <section aria-labelledby="settings-guidance-title">
          <div class="section-heading">
            <div>
              <span class="eyebrow"
                >{$translation("settings-guidance-eyebrow")}</span
              >
              <h3 id="settings-guidance-title">
                {$translation("settings-guidance-title")}
              </h3>
            </div>
          </div>
          <div class="support-actions">
            <button
              type="button"
              disabled={busy || !desktopAvailable}
              onclick={replayGuidance}
            >
              {$translation("settings-replay-guidance")}
            </button>
            <button type="button" onclick={() => openRelated(onopenlegal)}>
              {$translation("legal-open")}
            </button>
            <button
              type="button"
              disabled={!desktopAvailable}
              onclick={() => openRelated(onopendiagnostics)}
            >
              {$translation("diagnostics-open")}
            </button>
          </div>
        </section>
      </div>

      <footer class="settings-footer">
        <button
          type="button"
          disabled={busy || !desktopAvailable}
          onclick={restoreDefaults}
        >
          {$translation("settings-restore-defaults")}
        </button>
        <span
          >{hasUnsavedChanges
            ? $translation("settings-unsaved")
            : $translation("settings-up-to-date")}</span
        >
        <button
          data-modal-autofocus
          class="primary-action"
          type="button"
          disabled={busy || !desktopAvailable || !hasUnsavedChanges}
          onclick={save}
          >{busy
            ? $translation("settings-saving")
            : $translation("settings-save")}</button
        >
      </footer>
    </dialog>
  </div>
{/if}

<style>
  .settings-backdrop {
    position: fixed;
    z-index: 230;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(2, 7, 12, 0.86);
  }

  .settings-dialog {
    width: min(1180px, calc(100vw - 32px));
    max-height: calc(100vh - 40px);
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr) auto;
    overflow: hidden;
    border: 1px solid var(--colour-line);
    padding: 0;
    color: var(--colour-text);
    background: var(--colour-canvas);
    box-shadow: 0 22px 60px rgba(0, 0, 0, 0.55);
  }

  .settings-header,
  .settings-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 20px 24px;
    background: var(--colour-surface);
  }

  .settings-header {
    border-bottom: 1px solid var(--colour-line);
  }

  .settings-header h2,
  .settings-header p,
  h3,
  p {
    margin: 0;
  }

  .settings-header h2 {
    font-family: var(--font-display);
    font-size: 1.65rem;
  }

  .settings-header p {
    margin-top: 6px;
    color: var(--colour-muted);
    line-height: 1.5;
  }

  .settings-content {
    min-height: 0;
    overflow: auto;
    padding: 18px 24px 26px;
  }

  section {
    border: 1px solid var(--colour-line-faint);
    padding: 18px;
    background: var(--colour-surface);
  }

  section + section {
    margin-top: 14px;
  }

  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
  }

  h3 {
    font-family: var(--font-display);
    font-size: 1.2rem;
  }

  h4 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.05rem;
  }

  .source-grid,
  .related-grid,
  .setting-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .source-grid article,
  .setting-row,
  .related-grid button {
    min-width: 0;
    border: 1px solid var(--colour-line-faint);
    padding: 13px;
    background: var(--colour-surface-raised);
  }

  .source-grid article {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    align-content: start;
    gap: 7px;
  }

  .source-grid article > span,
  .related-grid span {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .source-grid small,
  .setting-row small {
    color: var(--colour-muted);
    line-height: 1.5;
  }

  .source-grid button {
    align-self: end;
    min-height: 40px;
    margin-top: 5px;
  }

  .related-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    margin-bottom: 10px;
  }

  .related-grid button {
    min-height: 62px;
    display: grid;
    justify-items: start;
    gap: 5px;
    text-align: start;
  }

  .setting-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    margin-top: 10px;
  }

  .setting-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(150px, 220px);
    align-items: center;
    gap: 18px;
  }

  .setting-row > span {
    display: grid;
    gap: 4px;
  }

  .setting-row select,
  .setting-row input[type="number"] {
    width: 100%;
    min-height: 40px;
    border: 1px solid var(--colour-line);
    padding: 7px 10px;
    color: var(--colour-text);
    background: var(--colour-surface-soft);
  }

  .setting-toggle {
    margin-top: 10px;
  }

  .setting-toggle input {
    width: 24px;
    height: 24px;
    justify-self: end;
    accent-color: var(--colour-observed);
  }

  .maintenance-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 18px;
    border-top: 1px solid var(--colour-line-faint);
    padding-top: 16px;
  }

  .maintenance-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    margin-top: 12px;
  }

  .maintenance-grid > div {
    display: grid;
    gap: 5px;
    min-width: 0;
    border: 1px solid var(--colour-line-faint);
    padding: 11px;
    background: var(--colour-surface-raised);
  }

  .maintenance-grid span,
  .maintenance-detail {
    color: var(--colour-muted);
    line-height: 1.45;
  }

  .maintenance-detail {
    margin-top: 9px;
  }

  .confirmation-actions {
    display: flex;
    justify-content: flex-end;
    gap: 9px;
    margin-top: 10px;
  }

  .support-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .support-actions button,
  .section-heading button,
  .maintenance-heading button,
  .confirmation-actions button,
  .settings-footer button {
    min-height: 40px;
    padding: 8px 14px;
  }

  .settings-footer {
    border-top: 1px solid var(--colour-line);
  }

  .settings-footer span {
    flex: 1;
    color: var(--colour-muted);
    text-align: end;
  }

  .primary-action {
    border-color: var(--colour-gold);
    color: var(--colour-gold);
  }

  .settings-error,
  .settings-status {
    margin: 12px 24px 0;
    border-inline-start: 3px solid var(--colour-risk);
    padding: 10px 12px;
    background: var(--colour-risk-soft);
  }

  .settings-status {
    border-color: var(--colour-success);
    background: var(--colour-success-soft);
  }

  @media (max-width: 900px) {
    .source-grid,
    .related-grid,
    .setting-grid,
    .maintenance-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 620px) {
    .settings-backdrop {
      padding: 6px;
    }

    .settings-dialog {
      width: calc(100vw - 12px);
      max-height: calc(100vh - 12px);
    }

    .settings-header,
    .settings-footer,
    .settings-content {
      padding: 14px;
    }

    .settings-footer {
      align-items: stretch;
      flex-direction: column;
    }

    .settings-footer span {
      text-align: start;
    }

    .setting-row {
      grid-template-columns: 1fr;
    }
  }
</style>
