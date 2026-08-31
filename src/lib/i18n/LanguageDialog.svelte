<script lang="ts">
  import { modalFocus } from "../ui/modalFocus";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import FilePicker from "../ui/FilePicker.svelte";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import { notify } from "../notifications/service";
  import { translation } from "./runtime";
  import type { LanguageStatus } from "./types";
  import type { TranslationKey } from "./catalog";
  import {
    installLanguagePack,
    inspectLanguagePack,
    languageErrorMessageKeys,
    languageStatus,
    LanguageServiceError,
    removeLanguagePack,
    selectLanguagePack,
    exportLanguagePack,
  } from "./service";

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();
  let busy = $state(false);
  let errorMessage = $state("");
  const MAX_MANIFEST_READ_BYTES = 256 * 1024 + 1;

  function trustLabel(trust: string): string {
    if (trust === "built_in") return $translation("security-language-built-in");
    if (trust === "reviewed") return $translation("security-language-reviewed");
    return $translation("security-language-community");
  }

  function storageAuthorityKey(
    authority: LanguageStatus["storage_authority"],
  ): TranslationKey {
    if (authority === "native_sqlite")
      return "security-language-storage-native";
    if (authority === "native_unavailable")
      return "security-language-storage-unavailable";
    return "security-language-storage-preview";
  }

  function reportError(error: unknown): void {
    const key =
      error instanceof LanguageServiceError
        ? languageErrorMessageKeys[error.code]
        : "error-language-storage-unavailable";
    errorMessage = $translation(key);
  }

  async function handleFile(file: File | null): Promise<void> {
    if (!file) return;
    busy = true;
    errorMessage = "";
    try {
      const manifestJson = await file.slice(0, MAX_MANIFEST_READ_BYTES).text();
      const inspection = await inspectLanguagePack(manifestJson);
      if (!inspection.valid || !inspection.manifest) {
        throw new LanguageServiceError(
          inspection.code ?? "invalid_manifest",
          inspection.detail,
        );
      }
      await installLanguagePack(manifestJson);
      notify({
        title: $translation("language-title"),
        message: $translation("language-installed-status", {
          name: inspection.manifest.name,
        }),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function selectPack(packId: string): Promise<void> {
    busy = true;
    try {
      await selectLanguagePack(packId);
      errorMessage = "";
      notify({
        title: $translation("language-title"),
        message: $translation("language-selected"),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function removePack(packId: string, name: string): Promise<void> {
    if (
      !window.confirm(
        $translation("destructive-language-remove-confirm", { name }),
      )
    ) {
      return;
    }
    busy = true;
    try {
      await removeLanguagePack(packId);
      errorMessage = "";
      notify({
        title: $translation("language-title"),
        message: $translation("language-removed-status", { name }),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  async function exportPack(packId: string, name: string): Promise<void> {
    busy = true;
    try {
      const json = await exportLanguagePack(packId);
      const url = URL.createObjectURL(
        new Blob([`${json}\n`], { type: "application/json" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = `${packId}.rolanguage.json`;
      document.body.append(link);
      link.click();
      link.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 0);
      errorMessage = "";
      notify({
        title: $translation("language-title"),
        message: $translation("language-exported-status", { name }),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }
</script>

{#if open}
  <div class="language-backdrop">
    <dialog
      use:modalFocus={{ onclose, closeDisabled: busy }}
      open
      class="language-dialog"
      aria-modal="true"
      aria-labelledby="language-title"
      aria-describedby="language-introduction"
    >
      <header>
        <div class="language-heading">
          <span class="eyebrow">{$translation("language-eyebrow")}</span>
          <div class="language-heading-row">
            <h2 id="language-title">{$translation("language-title")}</h2>
            <ContextHelp
              topic="language-packs"
              title={$translation("help-language-packs-title")}
              text={$translation("help-language-packs-text")}
              placement="right"
            />
          </div>
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

      <p id="language-introduction">{$translation("language-introduction")}</p>

      <div
        class="language-list"
        aria-label={$translation("language-list-label")}
      >
        {#each $languageStatus.packs as pack}
          <article
            class="language-card"
            class:selected={pack.manifest.id ===
              $languageStatus.selected_language_pack_id}
          >
            <span class="language-locale" lang={pack.manifest.locale}
              >{pack.manifest.locale}</span
            >
            <div class="language-copy">
              <strong
                lang={pack.manifest.locale}
                dir={pack.manifest.direction === "right_to_left"
                  ? "rtl"
                  : "ltr"}>{pack.manifest.name}</strong
              >
              <small>
                {trustLabel(pack.trust)}
                {#if pack.manifest.author}
                  · {$translation("language-by-author", {
                    author: pack.manifest.author,
                  })}
                {/if}
              </small>
              <small>
                {$translation("coverage-language-pack", {
                  translated: pack.translated_messages,
                  total: pack.eligible_messages,
                })}
                · {$translation(
                  pack.manifest.direction === "right_to_left"
                    ? "language-direction-rtl"
                    : "language-direction-ltr",
                )}
              </small>
            </div>
            <div class="language-actions">
              {#if pack.manifest.id === $languageStatus.selected_language_pack_id}
                <span class="language-active"
                  >{$translation("security-language-active")}</span
                >
              {:else}
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => void selectPack(pack.manifest.id)}
                >
                  {$translation("language-select")}
                </button>
              {/if}
              {#if pack.trust === "community"}
                <button
                  type="button"
                  disabled={busy}
                  onclick={() =>
                    void exportPack(pack.manifest.id, pack.manifest.name)}
                  >{$translation("language-export")}</button
                >
                <button
                  type="button"
                  class="language-remove"
                  disabled={busy}
                  title={$translation("language-remove-title")}
                  onclick={() =>
                    void removePack(pack.manifest.id, pack.manifest.name)}
                  >{$translation("action-remove")}</button
                >
              {/if}
            </div>
          </article>
        {/each}
      </div>

      <GuidanceSurface
        kind="boundary"
        layout="block"
        semanticRole="note"
        class="language-boundary"
      >
        <strong
          >{$translation("security-language-source-version", {
            version: $languageStatus.active_pack.source_catalog_version,
            revision: $languageStatus.active_pack.source_catalog_revision,
          })}</strong
        >
        <span>{$translation("security-language-boundary")}</span>
        <span>
          {$translation(storageAuthorityKey($languageStatus.storage_authority))}
        </span>
      </GuidanceSurface>

      {#if errorMessage}<p class="language-error" role="alert">
          {errorMessage}
        </p>{/if}
      <footer>
        <div>
          <strong>{$translation("language-community-files")}</strong>
          <span>{$translation("language-community-files-detail")}</span>
        </div>
        <FilePicker
          id="language-pack-file-input"
          accept="application/json,.json,.rolanguage.json"
          disabled={busy}
          label={busy
            ? $translation("language-installing")
            : $translation("language-choose")}
          showFileName={false}
          onselect={handleFile}
        />
      </footer>
    </dialog>
  </div>
{/if}
