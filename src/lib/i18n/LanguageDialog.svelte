<script lang="ts">
  import { modalFocus } from "../ui/modalFocus";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import { notify } from "../notifications/service";
  import { translation } from "./runtime";
  import {
    installLanguagePack,
    languageErrorMessageKeys,
    languageStatus,
    LanguageServiceError,
    removeLanguagePack,
    selectLanguagePack,
  } from "./service";
  import { validateCommunityLanguagePackJson } from "./validation";

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();
  let fileInput = $state<HTMLInputElement>();
  let busy = $state(false);
  let errorMessage = $state("");
  const MAX_MANIFEST_READ_BYTES = 256 * 1024 + 1;

  function trustLabel(trust: string): string {
    if (trust === "built_in") return $translation("security-language-built-in");
    if (trust === "reviewed") return $translation("security-language-reviewed");
    return $translation("security-language-community");
  }

  function reportError(error: unknown): void {
    const key =
      error instanceof LanguageServiceError
        ? languageErrorMessageKeys[error.code]
        : "error-language-storage-unavailable";
    errorMessage = $translation(key);
  }

  async function handleFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    busy = true;
    errorMessage = "";
    try {
      const manifestJson = await file.slice(0, MAX_MANIFEST_READ_BYTES).text();
      const validation = validateCommunityLanguagePackJson(manifestJson);
      if (!validation.ok) {
        throw new LanguageServiceError(validation.code, validation.detail);
      }
      installLanguagePack(manifestJson);
      notify({
        title: $translation("language-title"),
        message: $translation("language-installed-status", {
          name: validation.manifest.name,
        }),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
    } finally {
      busy = false;
    }
  }

  function selectPack(packId: string): void {
    try {
      selectLanguagePack(packId);
      errorMessage = "";
      notify({
        title: $translation("language-title"),
        message: $translation("language-selected"),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
    }
  }

  function removePack(packId: string, name: string): void {
    if (
      !window.confirm(
        $translation("destructive-language-remove-confirm", { name }),
      )
    ) {
      return;
    }
    try {
      removeLanguagePack(packId);
      errorMessage = "";
      notify({
        title: $translation("language-title"),
        message: $translation("language-removed-status", { name }),
        tone: "success",
      });
    } catch (error) {
      reportError(error);
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
                  onclick={() => selectPack(pack.manifest.id)}
                >
                  {$translation("language-select")}
                </button>
              {/if}
              {#if pack.trust === "community"}
                <button
                  type="button"
                  class="language-remove"
                  disabled={busy}
                  title={$translation("language-remove-title")}
                  onclick={() =>
                    removePack(pack.manifest.id, pack.manifest.name)}
                  >{$translation("action-remove")}</button
                >
              {/if}
            </div>
          </article>
        {/each}
      </div>

      <aside class="language-boundary">
        <strong
          >{$translation("security-language-source-version", {
            version: $languageStatus.active_pack.source_catalog_version,
            revision: $languageStatus.active_pack.source_catalog_revision,
          })}</strong
        >
        <span>{$translation("security-language-boundary")}</span>
      </aside>

      {#if errorMessage}<p class="language-error" role="alert">
          {errorMessage}
        </p>{/if}
      <footer>
        <div>
          <strong>{$translation("language-community-files")}</strong>
          <span>{$translation("language-community-files-detail")}</span>
        </div>
        <input
          bind:this={fileInput}
          hidden
          aria-hidden="true"
          tabindex="-1"
          class="language-file-input"
          type="file"
          accept="application/json,.json,.rolanguage.json"
          onchange={(event) => void handleFile(event)}
        />
        <button
          type="button"
          disabled={busy}
          onclick={() => fileInput?.click()}
        >
          {busy
            ? $translation("language-installing")
            : $translation("language-choose")}
        </button>
      </footer>
    </dialog>
  </div>
{/if}
