<script lang="ts">
  let {
    id,
    accept,
    label,
    emptyLabel = "",
    disabled = false,
    showFileName = true,
    onselect,
  }: {
    id: string;
    accept: string;
    label: string;
    emptyLabel?: string;
    disabled?: boolean;
    showFileName?: boolean;
    onselect: (file: File | null) => void | Promise<void>;
  } = $props();

  let fileName = $state("");
  let input = $state<HTMLInputElement>();

  function handleChange(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0] ?? null;
    fileName = file?.name ?? "";
    input.value = "";
    void onselect(file);
  }
</script>

<span class="file-picker-shell">
  <input
    bind:this={input}
    class="file-picker-input"
    {id}
    type="file"
    {accept}
    {disabled}
    aria-hidden="true"
    tabindex="-1"
    onchange={handleChange}
  />
  <button
    class="file-picker-action"
    type="button"
    {disabled}
    onclick={() => input?.click()}>{label}</button
  >
  {#if showFileName}
    <span class="file-picker-name">{fileName || emptyLabel}</span>
  {/if}
</span>

<style>
  .file-picker-shell {
    display: contents;
  }

  .file-picker-input {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .file-picker-action {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    min-height: 2.15rem;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
    color: var(--colour-gold);
    padding: 0.45rem 0.68rem;
    cursor: pointer;
    font-size: max(0.75rem, var(--type-caption));
    line-height: 1.2;
  }

  .file-picker-action:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .file-picker-action:focus-visible {
    outline: 2px solid var(--colour-observed);
    outline-offset: 2px;
  }

  .file-picker-name {
    min-width: 0;
    color: var(--colour-muted);
    font-size: max(0.75rem, var(--type-caption));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
