<script lang="ts">
  import { activeLocale, translation } from "../i18n/runtime";
  import { activeTheme } from "../theme/runtime";
  import { notify } from "../notifications/service";
  import {
    chooseDirectory,
    configureDirectory,
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

  function fallbackPreferences(): ApplicationPreferences {
    return {
      schema_version: 1,
      storage_patience_preset: "balanced",
      custom_storage_patience_seconds: null,
      effective_storage_patience_seconds: 60,
      background_work_priority: "gentle",
      text_scale_percent: 100,
      motion_preference: "system",
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
      const selected = await chooseDirectory(title);
      if (!selected) return;
      const nextSetup = await configureDirectory(kind, selected);
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

  function openRelated(action: () => void): void {
    if (hasUnsavedChanges) {
      errorMessage = $translation("settings-save-before-leaving");
      return;
    }
    onclose();
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
  <div class="settings-backdrop">
    <dialog
      use:modalFocus={{ onclose, closeDisabled: busy }}
      open
      class="settings-dialog"
      aria-modal="true"
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
          onclick={onclose}>×</button
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

          <div class="source-grid">
            <article>
              <span>{$translation("observer-save-folder")}</span>
              <strong
                >{currentSetup?.save_directory?.name ??
                  $translation("observer-not-configured")}</strong
              >
              <small>{$translation("observer-save-folder-detail")}</small>
              <button
                type="button"
                disabled={busy || !desktopAvailable}
                onclick={() => selectDirectory("save")}
                >{$translation("observer-choose-save-folder")}</button
              >
            </article>
            <article>
              <span>{$translation("observer-game-folder")}</span>
              <strong
                >{currentSetup?.game_directory?.name ??
                  $translation("observer-not-configured")}</strong
              >
              <small>{$translation("observer-game-folder-detail")}</small>
              <button
                type="button"
                disabled={busy || !desktopAvailable}
                onclick={() => selectDirectory("game")}
                >{$translation("observer-choose-game-folder")}</button
              >
            </article>
            <article>
              <span>{$translation("observer-workshop-folder")}</span>
              <strong
                >{currentSetup?.workshop_directory?.name ??
                  $translation("observer-automatic-discovery")}</strong
              >
              <small>{$translation("observer-workshop-private")}</small>
              <button
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
            </div>
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

  .support-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .support-actions button,
  .section-heading button,
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
    .setting-grid {
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
