<script lang="ts">
  import { modalFocus } from "../ui/modalFocus";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import FilePicker from "../ui/FilePicker.svelte";
  import { translation } from "../i18n/runtime";
  import type { TranslationKey } from "../i18n/catalog";
  import { notify } from "../notifications/service";
  import {
    exportTheme,
    importTheme,
    inspectTheme,
    removeTheme,
    selectTheme,
    themeStatus,
  } from "./service";
  import type {
    AvailableThemeRevision,
    ThemeInspection,
    ThemeManifest,
  } from "./types";

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();
  let busy = $state(false);
  let errorMessage = $state("");
  let inspection = $state<ThemeInspection | null>(null);
  let draft = $state<ThemeManifest | null>(null);
  let draftEditable = $state(false);
  let draftTimer: number | undefined;
  const MAX_THEME_BYTES = 32 * 1024 + 1;
  const errorKeys: Record<string, TranslationKey> = {
    theme_manifest_too_large: "theme-error-too-large",
    invalid_theme_manifest: "theme-error-invalid-manifest",
    unsupported_theme_version: "theme-error-unsupported-version",
    invalid_theme_identifier: "theme-error-invalid-id",
    invalid_theme_metadata: "theme-error-invalid-metadata",
    invalid_theme_colour: "theme-error-invalid-colour",
    theme_insufficient_contrast: "theme-error-insufficient-contrast",
    duplicate_theme: "theme-error-duplicate",
    theme_revision_conflict: "theme-error-revision-conflict",
    unknown_theme: "theme-error-unknown",
    active_theme_remove: "theme-error-active-remove",
    built_in_theme_remove: "theme-error-built-in-remove",
  };
  const remediationKeys: Record<
    import("./types").ThemeContrastCheck["remediation"],
    TranslationKey
  > = {
    increase_foreground_surface_difference:
      "theme-remediation-foreground-surface",
    strengthen_control_boundary: "theme-remediation-control-boundary",
    strengthen_decorative_divider: "theme-remediation-divider",
    strengthen_chart_surface_difference: "theme-remediation-chart-surface",
    adjust_derived_soft_fill: "theme-remediation-soft-fill",
    increase_chart_series_distinction: "theme-remediation-chart-distinction",
  };

  const colourRoles: Array<keyof ThemeManifest["colours"]> = [
    "canvas",
    "surface",
    "surface_raised",
    "surface_soft",
    "text",
    "text_muted",
    "line",
    "accent",
    "observed",
    "risk",
    "success",
    "comparison",
  ];

  function roleLabel(role: string): string {
    return role.replaceAll("_", " ");
  }

  function describeError(error: unknown): string {
    if (typeof error === "object" && error && "message" in error) {
      return String((error as { message: unknown }).message);
    }
    if (typeof error === "object" && error && "code" in error) {
      const code = String((error as { code: unknown }).code);
      return $translation(errorKeys[code] ?? "theme-error-generic");
    }
    return $translation("theme-error-generic");
  }

  function download(content: string, fileName: string): void {
    const url = URL.createObjectURL(
      new Blob([`${content.trim()}\n`], { type: "application/json" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = fileName;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
  }

  function manifestDocument(manifest: ThemeManifest): string {
    return JSON.stringify(manifest, null, 2);
  }

  async function handleFile(file: File | null): Promise<void> {
    if (!file) return;
    busy = true;
    errorMessage = "";
    try {
      const themeDocument = await file.slice(0, MAX_THEME_BYTES).text();
      const result = await inspectTheme(themeDocument);
      inspection = result;
      if (!result.structurally_valid || !result.manifest) {
        throw new Error(
          result.detail ?? $translation("theme-invalid-structure"),
        );
      }
      if (!result.report?.valid) {
        throw new Error($translation("theme-install-contrast-blocked"));
      }
      await importTheme(themeDocument);
      notify({
        title: $translation("theme-title"),
        message: $translation("theme-imported", { name: result.manifest.name }),
        tone: "success",
      });
    } catch (error) {
      errorMessage = describeError(error);
    } finally {
      busy = false;
    }
  }

  async function chooseTheme(theme: AvailableThemeRevision): Promise<void> {
    busy = true;
    errorMessage = "";
    try {
      await selectTheme(
        theme.manifest.id,
        theme.manifest.version,
        theme.content_hash,
      );
      notify({
        title: $translation("theme-title"),
        message: $translation("theme-selected", { name: theme.manifest.name }),
        tone: "success",
      });
    } catch (error) {
      errorMessage = describeError(error);
    } finally {
      busy = false;
    }
  }

  async function exportRevision(theme: AvailableThemeRevision): Promise<void> {
    busy = true;
    try {
      const exported = await exportTheme(
        theme.manifest.id,
        theme.manifest.version,
        theme.content_hash,
      );
      download(
        exported,
        `${theme.manifest.id}-${theme.manifest.version}.rotheme.json`,
      );
    } catch (error) {
      errorMessage = describeError(error);
    } finally {
      busy = false;
    }
  }

  async function removeRevision(theme: AvailableThemeRevision): Promise<void> {
    if (
      !window.confirm(
        $translation("theme-remove-confirm", { name: theme.manifest.name }),
      )
    ) {
      return;
    }
    busy = true;
    try {
      await removeTheme(theme.manifest.id, theme.manifest.version);
      notify({
        title: $translation("theme-title"),
        message: $translation("theme-removed", { name: theme.manifest.name }),
        tone: "success",
      });
    } catch (error) {
      errorMessage = describeError(error);
    } finally {
      busy = false;
    }
  }

  function beginDraft(theme: AvailableThemeRevision): void {
    draftEditable = true;
    draft = structuredClone(theme.manifest);
    draft.id = "org.example.observatory-theme";
    draft.version = "1.0.0";
    draft.name = `${theme.manifest.name} study`;
    draft.author = "Community theme author";
    inspection = null;
    scheduleDraftInspection();
  }

  function inspectRevision(theme: AvailableThemeRevision): void {
    draftEditable = false;
    draft = structuredClone(theme.manifest);
    inspection = {
      structurally_valid: true,
      manifest: structuredClone(theme.manifest),
      content_hash: theme.content_hash,
      report: structuredClone(theme.report),
    };
    errorMessage = "";
  }

  function scheduleDraftInspection(): void {
    if (!draftEditable) return;
    if (draftTimer) window.clearTimeout(draftTimer);
    draftTimer = window.setTimeout(() => void validateDraft(), 180);
  }

  async function validateDraft(): Promise<void> {
    if (!draft) return;
    try {
      inspection = await inspectTheme(manifestDocument(draft));
      errorMessage = inspection.structurally_valid
        ? ""
        : (inspection.detail ?? $translation("theme-invalid-structure"));
    } catch (error) {
      errorMessage = describeError(error);
    }
  }

  function exportDraft(): void {
    if (!draft || !inspection?.structurally_valid) return;
    download(
      manifestDocument(draft),
      `${draft.id}-${draft.version}.rotheme.json`,
    );
  }
</script>

{#if open}
  <div class="theme-backdrop">
    <dialog
      use:modalFocus={{ onclose, closeDisabled: busy }}
      open
      class="theme-dialog"
      aria-modal="true"
      aria-labelledby="theme-title"
      aria-describedby="theme-introduction"
    >
      <header>
        <div>
          <span class="eyebrow">{$translation("theme-eyebrow")}</span>
          <div class="heading-row">
            <h2 id="theme-title">{$translation("theme-title")}</h2>
            <ContextHelp
              topic="safe-themes"
              title={$translation("theme-help-title")}
              text={$translation("theme-help-text")}
              placement="right"
            />
          </div>
        </div>
        <button
          data-modal-autofocus
          class="close"
          type="button"
          aria-label={$translation("action-close")}
          disabled={busy}
          onclick={onclose}>×</button
        >
      </header>

      <p id="theme-introduction">{$translation("theme-introduction")}</p>

      {#if $themeStatus}
        <section aria-labelledby="installed-themes-heading">
          <div class="section-heading">
            <div>
              <span class="eyebrow"
                >{$translation("theme-installed-eyebrow")}</span
              >
              <h3 id="installed-themes-heading">
                {$translation("theme-installed-title")}
              </h3>
            </div>
            <FilePicker
              id="theme-file-input"
              accept="application/json,.json,.rotheme.json"
              disabled={busy}
              label={$translation("theme-import")}
              showFileName={false}
              onselect={handleFile}
            />
          </div>
          <div class="theme-list">
            {#each $themeStatus.themes as theme (`${theme.manifest.id}@${theme.manifest.version}`)}
              <article class:selected={theme.selected} class="theme-card">
                <div
                  class="swatches"
                  aria-label={$translation("theme-palette-label")}
                >
                  {#each theme.manifest.chart_palette as colour}
                    <span style:background={colour} title={colour}></span>
                  {/each}
                </div>
                <div class="theme-copy">
                  <strong>{theme.manifest.name}</strong>
                  <span>{theme.manifest.id} · v{theme.manifest.version}</span>
                  <span>
                    {$translation(
                      theme.source === "built_in"
                        ? "theme-source-built-in"
                        : "theme-source-local",
                    )}
                    {#if theme.manifest.author}
                      · {$translation("theme-author-unverified", {
                        author: theme.manifest.author,
                      })}
                    {/if}
                  </span>
                </div>
                <div class="theme-actions">
                  {#if theme.selected}
                    <span class="active">{$translation("theme-active")}</span>
                  {:else}
                    <button
                      type="button"
                      disabled={busy}
                      onclick={() => void chooseTheme(theme)}
                    >
                      {$translation(
                        $themeStatus.selected_theme_id === theme.manifest.id
                          ? "theme-rollback"
                          : "theme-select",
                      )}
                    </button>
                  {/if}
                  <button
                    type="button"
                    disabled={busy}
                    onclick={() => inspectRevision(theme)}
                  >
                    {$translation("theme-inspect")}
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onclick={() => beginDraft(theme)}
                  >
                    {$translation("theme-use-as-study")}
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onclick={() => void exportRevision(theme)}
                  >
                    {$translation("theme-export")}
                  </button>
                  {#if theme.source === "local_import" && !theme.selected}
                    <button
                      class="remove"
                      type="button"
                      disabled={busy}
                      onclick={() => void removeRevision(theme)}
                    >
                      {$translation("action-remove")}
                    </button>
                  {/if}
                </div>
              </article>
            {/each}
          </div>
        </section>
      {/if}

      {#if draft}
        <section class="laboratory" aria-labelledby="theme-laboratory-heading">
          <div class="section-heading">
            <div>
              <span class="eyebrow"
                >{$translation("theme-laboratory-eyebrow")}</span
              >
              <h3 id="theme-laboratory-heading">
                {$translation("theme-laboratory-title")}
              </h3>
            </div>
            <button
              type="button"
              disabled={!draftEditable || !inspection?.structurally_valid}
              onclick={exportDraft}
            >
              {$translation("theme-export-draft")}
            </button>
          </div>
          <p>{$translation("theme-laboratory-boundary")}</p>
          <div class="metadata-grid">
            <label>
              <span>{$translation("theme-field-id")}</span>
              <input
                bind:value={draft.id}
                disabled={!draftEditable}
                oninput={scheduleDraftInspection}
              />
            </label>
            <label>
              <span>{$translation("theme-field-version")}</span>
              <input
                bind:value={draft.version}
                disabled={!draftEditable}
                oninput={scheduleDraftInspection}
              />
            </label>
            <label>
              <span>{$translation("theme-field-name")}</span>
              <input
                bind:value={draft.name}
                disabled={!draftEditable}
                oninput={scheduleDraftInspection}
              />
            </label>
            <label>
              <span>{$translation("theme-field-author")}</span>
              <input
                bind:value={draft.author}
                disabled={!draftEditable}
                oninput={scheduleDraftInspection}
              />
            </label>
          </div>
          <div class="laboratory-grid">
            <div class="colour-grid">
              {#each colourRoles as role}
                <label>
                  <span>{roleLabel(role)}</span>
                  <input
                    type="color"
                    bind:value={draft.colours[role]}
                    disabled={!draftEditable}
                    oninput={scheduleDraftInspection}
                  />
                  <code>{draft.colours[role]}</code>
                </label>
              {/each}
              {#each draft.chart_palette as colour, index}
                <label>
                  <span
                    >{$translation("theme-chart-colour", {
                      number: index + 1,
                    })}</span
                  >
                  <input
                    type="color"
                    bind:value={draft.chart_palette[index]}
                    disabled={!draftEditable}
                    oninput={scheduleDraftInspection}
                  />
                  <code>{colour}</code>
                </label>
              {/each}
            </div>
            <div
              class="live-preview"
              style:background={draft.colours.canvas}
              style:color={draft.colours.text}
            >
              <span style:color={draft.colours.accent}
                >{$translation("theme-preview-eyebrow")}</span
              >
              <h3>{$translation("theme-preview-title")}</h3>
              <p style:color={draft.colours.text_muted}>
                {$translation("theme-preview-text")}
              </p>
              <div
                class="preview-surface"
                style:background={draft.colours.surface_raised}
                style:border-color={draft.colours.line}
              >
                <strong style:color={draft.colours.observed}
                  >{$translation("theme-preview-observed")}</strong
                >
                <button
                  style:background={draft.colours.surface_soft}
                  style:border-color={draft.colours.line}
                  style:color={draft.colours.accent}
                >
                  {$translation("theme-preview-control")}
                </button>
              </div>
              <div
                class="preview-chart"
                aria-label={$translation("theme-palette-label")}
              >
                {#each draft.chart_palette as colour, index}
                  <span
                    style:background={colour}
                    style:height={`${28 + index * 9}px`}
                  ></span>
                {/each}
              </div>
            </div>
          </div>

          {#if inspection?.report}
            <div class="assay-heading">
              <strong>{$translation("theme-assay-title")}</strong>
              <span
                class:valid={inspection.report.valid}
                class:invalid={!inspection.report.valid}
              >
                {$translation(
                  inspection.report.valid
                    ? "theme-assay-passed"
                    : "theme-assay-failed",
                  {
                    errors: inspection.report.errors,
                    warnings: inspection.report.warnings,
                  },
                )}
              </span>
            </div>
            <div class="assay-wrap">
              <table>
                <thead>
                  <tr>
                    <th scope="col">{$translation("theme-assay-pair")}</th>
                    <th scope="col">{$translation("theme-assay-ratio")}</th>
                    <th scope="col">{$translation("theme-assay-required")}</th>
                    <th scope="col">{$translation("theme-assay-result")}</th>
                  </tr>
                </thead>
                <tbody>
                  {#each inspection.report.checks as check}
                    <tr class:failed={!check.passes}>
                      <td
                        >{roleLabel(check.foreground)} / {roleLabel(
                          check.background,
                        )}</td
                      >
                      <td>{check.measured.toFixed(2)}:1</td>
                      <td>{check.minimum.toFixed(1)}:1</td>
                      <td
                        >{check.passes
                          ? $translation("theme-check-pass")
                          : $translation(
                              remediationKeys[check.remediation],
                            )}</td
                      >
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </section>
      {/if}

      {#if errorMessage}<p class="theme-error" role="alert">
          {errorMessage}
        </p>{/if}

      <aside class="theme-boundary">
        <strong>{$translation("theme-data-only-title")}</strong>
        <span>{$translation("theme-data-only-detail")}</span>
      </aside>
    </dialog>
  </div>
{/if}

<style>
  .theme-backdrop {
    position: fixed;
    inset: 0;
    z-index: 190;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.72);
  }

  .theme-dialog {
    width: min(1120px, calc(100vw - 40px));
    max-height: calc(100vh - 40px);
    overflow: auto;
    display: grid;
    gap: 18px;
    border: 1px solid var(--colour-line);
    padding: 22px;
    color: var(--colour-text);
    background: var(--colour-canvas);
    box-shadow: 0 24px 72px rgba(0, 0, 0, 0.62);
  }

  header,
  .section-heading,
  .heading-row,
  .theme-actions,
  .assay-heading {
    display: flex;
    align-items: center;
  }

  header,
  .section-heading,
  .assay-heading {
    justify-content: space-between;
    gap: 16px;
  }

  .heading-row {
    gap: 9px;
  }
  h2 {
    font-size: 1.65rem;
  }
  h3 {
    font-size: 1.15rem;
  }
  p {
    color: var(--colour-muted);
    line-height: 1.55;
  }

  .close {
    width: 38px;
    min-height: 38px;
    border: 1px solid var(--colour-line);
    color: var(--colour-text);
    background: var(--colour-surface-raised);
    font-size: 1.35rem;
  }

  section,
  .theme-boundary {
    display: grid;
    gap: 13px;
    border: 1px solid var(--colour-line-faint);
    padding: 16px;
    background: var(--colour-surface);
  }

  .theme-list {
    display: grid;
    gap: 8px;
  }
  .theme-card {
    display: grid;
    grid-template-columns: 150px minmax(220px, 1fr) auto;
    gap: 14px;
    align-items: center;
    border: 1px solid var(--colour-line-faint);
    padding: 12px;
    background: var(--colour-surface-raised);
  }
  .theme-card.selected {
    border-inline-start: 3px solid var(--colour-gold);
  }
  .swatches {
    display: flex;
    height: 35px;
  }
  .swatches span {
    flex: 1;
    min-width: 12px;
  }
  .theme-copy {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .theme-copy span {
    color: var(--colour-muted);
    overflow-wrap: anywhere;
  }
  .theme-actions {
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 7px;
  }
  button {
    min-height: 2.2rem;
    border: 1px solid var(--colour-line);
    padding: 0.42rem 0.68rem;
    color: var(--colour-gold);
    background: var(--colour-surface-raised);
    cursor: pointer;
  }
  button:disabled {
    cursor: not-allowed;
    color: var(--colour-muted);
  }
  .remove {
    color: var(--colour-risk);
  }
  .active,
  .valid {
    color: var(--colour-success);
    font-weight: 700;
  }
  .invalid {
    color: var(--colour-risk);
    font-weight: 700;
  }

  .metadata-grid,
  .laboratory-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  label {
    display: grid;
    gap: 6px;
    color: var(--colour-muted);
  }
  input {
    min-height: 2.35rem;
    border: 1px solid var(--colour-line);
    padding: 0.48rem 0.6rem;
    color: var(--colour-text);
    background: var(--colour-surface-raised);
  }
  .colour-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 7px;
  }
  .colour-grid label {
    grid-template-columns: minmax(90px, 1fr) 46px 74px;
    align-items: center;
    border-bottom: 1px solid var(--colour-line-faint);
    padding: 5px 0;
  }
  input[type="color"] {
    width: 44px;
    min-height: 34px;
    padding: 2px;
  }
  code {
    color: var(--colour-text);
  }

  .live-preview {
    min-height: 320px;
    display: grid;
    align-content: center;
    gap: 12px;
    padding: 22px;
  }
  .live-preview > span {
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .preview-surface {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid;
    padding: 15px;
  }
  .preview-chart {
    height: 100px;
    display: flex;
    align-items: end;
    gap: 9px;
  }
  .preview-chart span {
    flex: 1;
    min-width: 10px;
  }

  .assay-wrap {
    max-height: 310px;
    overflow: auto;
    border: 1px solid var(--colour-line-faint);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 9px 10px;
    border-bottom: 1px solid var(--colour-line-faint);
    text-align: start;
  }
  th {
    position: sticky;
    top: 0;
    color: var(--colour-gold);
    background: var(--colour-surface-raised);
  }
  tr.failed td {
    color: var(--colour-risk);
  }
  .theme-error {
    border-inline-start: 3px solid var(--colour-risk);
    padding: 12px;
    color: var(--colour-risk);
    background: var(--colour-risk-soft);
  }
  .theme-boundary {
    border-inline-start: 3px solid var(--colour-gold);
  }
  .theme-boundary span {
    color: var(--colour-muted);
    line-height: 1.5;
  }

  @media (max-width: 800px) {
    .theme-card {
      grid-template-columns: 1fr;
    }
    .theme-actions {
      justify-content: flex-start;
    }
    .metadata-grid,
    .laboratory-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
