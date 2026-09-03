<script lang="ts">
  import { untrack } from "svelte";
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import type { TranslationKey } from "../i18n/catalog";
  import {
    captureEnvironmentValidationSnapshot,
    desktopHostAvailable,
    getEnvironmentTelemetryCapability,
    recordEnvironmentValidationComparison,
  } from "../observations/desktopClient";
  import type {
    EnvironmentTelemetryCapability,
    EnvironmentValidationControl,
    EnvironmentValidationField,
    EnvironmentValidationResult,
  } from "../observations/types";

  const fieldLabels: Record<EnvironmentValidationField, TranslationKey> = {
    production: "environment-study-field-production",
    pollution: "environment-study-field-pollution",
    water_amount: "environment-study-field-water-amount",
    water_capacity: "environment-study-field-water-capacity",
    water_quality: "environment-study-field-water-quality",
    sewage_amount: "environment-study-field-sewage-amount",
    sewage_capacity: "environment-study-field-sewage-capacity",
    sewage_quality: "environment-study-field-sewage-quality",
  };
  const controlLabels: Record<EnvironmentValidationControl, TranslationKey> = {
    positive_value: "environment-study-control-positive-value",
    zero_value: "environment-study-control-zero-value",
    disconnected_facility: "environment-study-control-disconnected-facility",
    consecutive_frame_stability:
      "environment-study-control-consecutive-frame-stability",
    save_reload: "environment-study-control-save-reload",
    application_restart: "environment-study-control-application-restart",
  };
  const resultLabels: Record<EnvironmentValidationResult, TranslationKey> = {
    matches: "environment-study-result-matches",
    does_not_match: "environment-study-result-does-not-match",
    uncertain: "environment-study-result-uncertain",
  };
  const controls = Object.keys(controlLabels) as EnvironmentValidationControl[];
  const results = Object.keys(resultLabels) as EnvironmentValidationResult[];

  let { enabled = false }: { enabled?: boolean } = $props();
  let open = $state(false);
  let busy = $state(false);
  let capability = $state<EnvironmentTelemetryCapability | null>(null);
  let facilityIndex = $state(0);
  let field = $state<EnvironmentValidationField>("pollution");
  let wrValue = $state(0);
  let control = $state<EnvironmentValidationControl>("positive_value");
  let result = $state<EnvironmentValidationResult>("uncertain");
  let note = $state("");
  let message = $state("");

  const snapshot = $derived(capability?.latest_validation_snapshot ?? null);
  const facility = $derived(
    snapshot?.facilities.find(
      (item) => item.facility_index === facilityIndex,
    ) ?? null,
  );
  const availableFields = $derived.by(() => {
    if (!facility) return [] as EnvironmentValidationField[];
    return (
      [
        "production",
        "pollution",
        "water_amount",
        "water_capacity",
        "water_quality",
        "sewage_amount",
        "sewage_capacity",
        "sewage_quality",
      ] as EnvironmentValidationField[]
    ).filter((key) => facility[key] !== null);
  });
  const researchValue = $derived(
    facility && availableFields.includes(field) ? facility[field] : null,
  );

  $effect(() => {
    if (!enabled || !desktopHostAvailable()) return;
    untrack(() => void refresh());
  });

  $effect(() => {
    if (availableFields.length && !availableFields.includes(field)) {
      field = availableFields[0];
    }
  });

  async function refresh(): Promise<void> {
    busy = true;
    message = "";
    try {
      capability = await getEnvironmentTelemetryCapability();
      const first = capability.latest_validation_snapshot?.facilities.find(
        (item) =>
          item.production !== null ||
          item.pollution !== null ||
          item.water_amount !== null ||
          item.sewage_amount !== null,
      );
      if (first) facilityIndex = first.facility_index;
    } catch {
      message = $translation("environment-study-refresh-failed");
    } finally {
      busy = false;
    }
  }

  async function requestReading(): Promise<void> {
    busy = true;
    message = "";
    try {
      await captureEnvironmentValidationSnapshot();
      await refresh();
    } catch {
      message = $translation("environment-study-refresh-failed");
      busy = false;
    }
  }

  async function saveComparison(): Promise<void> {
    if (!snapshot || researchValue === null || !Number.isFinite(wrValue))
      return;
    busy = true;
    message = "";
    try {
      await recordEnvironmentValidationComparison({
        snapshot_id: snapshot.snapshot_id,
        facility_index: facilityIndex,
        field,
        wr_value: wrValue,
        control,
        result,
        note: note.trim() || null,
      });
      message = $translation("environment-study-saved");
      note = "";
      result = "uncertain";
    } catch {
      message = $translation("environment-study-save-failed");
    } finally {
      busy = false;
    }
  }
</script>

<section class="study" aria-labelledby="environment-study-title">
  <div class="study-introduction">
    <div>
      <span class="eyebrow">{$translation("environment-study-eyebrow")}</span>
      <h3 id="environment-study-title">
        {$translation("environment-study-title")}
      </h3>
      <p>{$translation("environment-study-introduction")}</p>
    </div>
    <button type="button" onclick={() => (open = !open)}>
      {$translation(open ? "environment-study-hide" : "environment-study-open")}
    </button>
  </div>

  {#if open}
    <p class="boundary" role="note">
      <strong>{$translation("environment-study-boundary-title")}</strong>
      {$translation("environment-study-boundary-detail")}
    </p>

    {#if !desktopHostAvailable()}
      <p>{$translation("environment-study-desktop-only")}</p>
    {:else if capability?.state === "snapshot_rejected"}
      <p class="warning" role="alert">
        {$translation("environment-study-rejected")}
      </p>
    {:else if !snapshot}
      <p>{$translation("environment-study-waiting")}</p>
      <button
        type="button"
        disabled={busy}
        onclick={() => void requestReading()}
      >
        {$translation("environment-study-check-again")}
      </button>
    {:else}
      <div class="study-grid">
        <label>
          <span>{$translation("environment-study-facility")}</span>
          <input
            type="number"
            min="0"
            max={Math.max(0, snapshot.facilities.length - 1)}
            bind:value={facilityIndex}
          />
        </label>
        <label>
          <span>{$translation("environment-study-field")}</span>
          <select bind:value={field}>
            {#each availableFields as item (item)}
              <option value={item}>{$translation(fieldLabels[item])}</option>
            {/each}
          </select>
        </label>
        <div class="research-value">
          <span>{$translation("environment-study-research-value")}</span>
          <strong>
            {researchValue === null
              ? "—"
              : formatNumber(researchValue, $activeLocale, {
                  maximumFractionDigits: 4,
                })}
          </strong>
        </div>
        <label>
          <span>{$translation("environment-study-wr-value")}</span>
          <input type="number" min="0" step="any" bind:value={wrValue} />
        </label>
        <label>
          <span>{$translation("environment-study-control")}</span>
          <select bind:value={control}>
            {#each controls as item}
              <option value={item}>{$translation(controlLabels[item])}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>{$translation("environment-study-result")}</span>
          <select bind:value={result}>
            {#each results as item}
              <option value={item}>{$translation(resultLabels[item])}</option>
            {/each}
          </select>
        </label>
        <label class="note">
          <span>{$translation("environment-study-note")}</span>
          <textarea maxlength="500" bind:value={note}></textarea>
        </label>
      </div>
      <div class="actions">
        <button type="button" disabled={busy} onclick={() => void refresh()}>
          {$translation("environment-study-refresh")}
        </button>
        <button
          type="button"
          class="primary"
          disabled={busy || researchValue === null}
          onclick={() => void saveComparison()}
        >
          {$translation("environment-study-save")}
        </button>
      </div>
    {/if}
    {#if message}<p class="message" role="status">{message}</p>{/if}
  {/if}
</section>

<style>
  .study {
    margin-block: 10px;
    border: 1px solid var(--colour-line-faint);
    padding: 12px;
    background: var(--colour-surface-raised);
  }
  .study-introduction,
  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h3,
  p {
    margin: 4px 0 0;
  }
  p,
  label,
  .research-value {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.5;
  }
  button,
  input,
  select,
  textarea {
    border: 1px solid var(--colour-line);
    padding: 8px 10px;
    color: var(--colour-text);
    background: var(--colour-surface);
  }
  button {
    cursor: pointer;
  }
  button.primary {
    border-color: var(--colour-gold);
    background: var(--colour-gold-soft);
  }
  button:disabled {
    cursor: not-allowed;
  }
  .boundary,
  .warning {
    margin-block: 10px;
    border-inline-start: 3px solid var(--colour-guidance);
    padding: 9px 11px;
    color: var(--colour-text);
    background: var(--colour-guidance-soft);
  }
  .warning {
    border-color: var(--colour-risk);
    background: var(--colour-risk-soft);
  }
  .boundary strong {
    color: var(--colour-guidance);
  }
  .study-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  label,
  .research-value {
    display: grid;
    align-content: start;
    gap: 5px;
  }
  .research-value {
    border: 1px solid var(--colour-line-faint);
    padding: 8px 10px;
  }
  .research-value strong {
    color: var(--colour-gold);
    font-size: 1.1rem;
  }
  .note {
    grid-column: 1 / -1;
  }
  textarea {
    min-height: 64px;
    resize: vertical;
  }
  .actions {
    justify-content: flex-end;
    margin-top: 10px;
  }
  .message {
    color: var(--colour-text);
  }
  @media (max-width: 720px) {
    .study-introduction {
      align-items: stretch;
      flex-direction: column;
    }
    .study-grid {
      grid-template-columns: 1fr;
    }
    .note {
      grid-column: auto;
    }
  }
</style>
