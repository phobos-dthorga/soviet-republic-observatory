<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import { exactObservationChartBindings } from "../navigation/chartBindings";
  import {
    destinationsForSubject,
    type RelatedDataDestination,
    type WorkspaceFilters,
    type WorkspaceLocation,
  } from "../navigation/relatedData";
  import {
    applyCarbonFactorImport,
    deleteLiveEnvironmentRecordings,
    disableEnvironmentRecording,
    enableEnvironmentRecording,
    exportCarbonFactorSet,
    indexAvailableSavesForEnvironment,
    previewCarbonFactorImport,
    removeCarbonFactorSet,
    resumeEnvironmentIndexing,
    rollbackCarbonFactorSet,
    saveCarbonFactorSet,
    selectCarbonFactorSet,
  } from "../observations/desktopClient";
  import type {
    CarbonFactorEntry,
    CarbonFactorImportPreview,
    CarbonFactorSetDraft,
    EnvironmentActivityChannel,
    EnvironmentIndexingProgress,
    EnvironmentWorkspaceModel,
  } from "../observations/types";
  import {
    carbonContributorsChart,
    carbonEstimateHelp,
    environmentActivityChart,
    environmentActivityHelp,
    environmentChannelLabel,
    formatCo2e,
  } from "../presentation/environment";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import ContextHelp from "../ui/ContextHelp.svelte";

  let {
    workspace = null,
    indexingProgress = null,
    desktopAvailable,
    location,
    onupdate,
    onprogress,
    onlocationchange,
    onrelatednavigate,
    onopenresearch,
  }: {
    workspace?: EnvironmentWorkspaceModel | null;
    indexingProgress?: EnvironmentIndexingProgress | null;
    desktopAvailable: boolean;
    location: WorkspaceLocation;
    onupdate: (workspace: EnvironmentWorkspaceModel) => void;
    onprogress: (progress: EnvironmentIndexingProgress) => void;
    onlocationchange?: (filters: WorkspaceFilters) => void;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
    onopenresearch: () => void;
  } = $props();

  const channels: EnvironmentActivityChannel[] = [
    "production",
    "construction_use",
    "factory_use",
    "shop_use",
    "vehicle_use",
    "factory_waste",
    "citizen_waste",
    "demolition_waste",
  ];
  const carbonChannels = channels.filter(
    (channel) => !channel.endsWith("_waste"),
  );
  let selectedChannel = $state<EnvironmentActivityChannel>("production");
  let selectedResource = $state("");
  let busy = $state(false);
  let consent = $state(false);
  let factorName = $state("");
  let factorBoundary = $state("");
  let factorReason = $state("");
  let factorResource = $state("");
  let factorChannel = $state<EnvironmentActivityChannel>("production");
  let factorValue = $state(0);
  let factorSource = $state("");
  let factorYear = $state(new Date().getFullYear());
  let factorEntryReason = $state("");
  let factorReference = $state("");
  let factorEntries = $state<CarbonFactorEntry[]>([]);
  let importCsv = $state("");
  let importPreview = $state<CarbonFactorImportPreview | null>(null);
  let deleteConfirmation = $state("");

  const activityResources = $derived(
    Array.from(
      new Set(
        (workspace?.activity ?? [])
          .filter((point) => point.activity_channel === selectedChannel)
          .map((point) => point.resource_token),
      ),
    ).sort(),
  );
  const effectiveResource = $derived(
    activityResources.includes(selectedResource)
      ? selectedResource
      : (activityResources[0] ?? ""),
  );
  const chart = $derived(
    environmentActivityChart(
      workspace,
      selectedChannel,
      effectiveResource,
      $translation,
    ),
  );
  const chartPoints = $derived(
    (workspace?.activity ?? []).filter(
      (point) =>
        point.activity_channel === selectedChannel &&
        point.resource_token === effectiveResource,
    ),
  );
  const chartNavigation = $derived(
    exactObservationChartBindings(chart, chartPoints, {
      workspace: "environment",
      section: selectedChannel.endsWith("_waste")
        ? "environment-waste"
        : "environment-overview",
      filters: {
        resourceToken: effectiveResource,
        activityChannel: selectedChannel,
      },
    }),
  );
  const wasteRows = $derived(
    (workspace?.activity ?? []).filter((point) =>
      point.activity_channel.endsWith("_waste"),
    ),
  );
  const carbonEstimate = $derived(workspace?.carbon_estimate ?? null);
  const carbonChart = $derived(
    carbonContributorsChart(carbonEstimate, $translation),
  );

  $effect(() => {
    const requested = location.filters.resourceToken;
    if (requested && workspace?.resources.includes(requested)) {
      selectedResource = requested;
    }
    const requestedChannel = location.filters.activityChannel;
    if (
      requestedChannel &&
      channels.includes(requestedChannel as EnvironmentActivityChannel)
    ) {
      selectedChannel = requestedChannel as EnvironmentActivityChannel;
    }
  });

  async function runIndexing(resume: boolean): Promise<void> {
    if (!desktopAvailable || busy) return;
    busy = true;
    try {
      onprogress(
        resume
          ? await resumeEnvironmentIndexing()
          : await indexAvailableSavesForEnvironment(),
      );
    } finally {
      busy = false;
    }
  }

  function chooseChannel(value: string): void {
    selectedChannel = value as EnvironmentActivityChannel;
    selectedResource = "";
    onlocationchange?.({ activityChannel: value });
  }

  function chooseResource(value: string): void {
    selectedResource = value;
    onlocationchange?.({ resourceToken: value });
  }

  function addFactor(): void {
    if (
      !factorResource ||
      !factorSource ||
      !factorEntryReason ||
      factorValue < 0
    )
      return;
    factorEntries = [
      ...factorEntries.filter(
        (entry) =>
          entry.resource_token !== factorResource ||
          entry.activity_channel !== factorChannel,
      ),
      {
        resource_token: factorResource,
        activity_channel: factorChannel,
        grams_co2e_per_unit: factorValue,
        source_name: factorSource,
        source_year: factorYear,
        reason: factorEntryReason,
        reference: factorReference || null,
      },
    ];
  }

  async function saveFactors(): Promise<void> {
    const draft: CarbonFactorSetDraft = {
      factor_set_id: null,
      name: factorName,
      accounting_boundary: factorBoundary,
      reason: factorReason,
      entries: factorEntries,
    };
    busy = true;
    try {
      onupdate(await saveCarbonFactorSet(draft));
      factorEntries = [];
    } finally {
      busy = false;
    }
  }

  async function exportFactors(id: string, revision: number): Promise<void> {
    const csv = await exportCarbonFactorSet(id, revision);
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = `${id}-r${revision}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  }

  async function readImport(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    importCsv = await file.text();
    importPreview = await previewCarbonFactorImport(importCsv);
  }
</script>

<section class="workspace environment-workspace">
  <aside
    class="navigator"
    aria-label={$translation("environment-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("environment-directorate")}</span>
        <h2>{$translation("nav-environment")}</h2>
      </div>
      <span class="edition">V1</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span><strong
          >{workspace?.analysis_context.selected_branch_id ?? "—"}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("environment-history-records")}</span><strong
          >{formatNumber(
            workspace?.history_records ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("environment-resources")}</span><strong
          >{formatNumber(
            workspace?.resources.length ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
    </div>
    <div class="section-list">
      <a href="#environment-overview" use:containedSectionNavigation
        ><span>01</span>{$translation("environment-section-overview")}</a
      >
      <a href="#environment-pollution" use:containedSectionNavigation
        ><span>02</span>{$translation("environment-section-pollution")}</a
      >
      <a href="#environment-waste" use:containedSectionNavigation
        ><span>03</span>{$translation("environment-section-waste")}</a
      >
      <a href="#environment-water" use:containedSectionNavigation
        ><span>04</span>{$translation("environment-section-water")}</a
      >
      <a href="#environment-carbon" use:containedSectionNavigation
        ><span>05</span>{$translation("environment-section-carbon")}</a
      >
      <a href="#environment-recording" use:containedSectionNavigation
        ><span>06</span>{$translation("environment-section-recording")}</a
      >
    </div>
    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note"
      ><span aria-hidden="true">◇</span>
      <p>{$translation("environment-sidebar-boundary")}</p></GuidanceSurface
    >
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong>{$translation("environment-evidence-banner")}</strong>
      <span>{$translation("environment-evidence-banner-detail")}</span>
    </GuidanceSurface>
    <header class="page-heading">
      <div>
        <span class="eyebrow"
          >{$translation("environment-heading-eyebrow")}</span
        >
        <h2>{$translation("environment-heading-title")}</h2>
        <p>{$translation("environment-heading-description")}</p>
      </div>
      <button
        type="button"
        disabled={!desktopAvailable || busy}
        onclick={() => runIndexing(indexingProgress?.phase === "paused")}
        >{$translation(
          indexingProgress?.phase === "paused"
            ? "environment-action-resume"
            : "environment-action-index",
        )}</button
      >
    </header>

    {#if indexingProgress && indexingProgress.phase !== "idle"}
      <section class="progress-card" aria-live="polite">
        <strong
          >{$translation("environment-index-progress", {
            current: indexingProgress.current_archive,
            total: indexingProgress.total_archives,
          })}</strong
        >
        <progress max="100" value={indexingProgress.progress_percent ?? 0}
        ></progress>
        <span
          >{$translation("environment-index-detail", {
            records: indexingProgress.records_processed,
            rows: indexingProgress.rows_processed,
          })}</span
        >
      </section>
    {/if}

    <section id="environment-overview" class="panel" tabindex="-1">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("environment-overview-eyebrow")}</span
          >
          <h3>{$translation("environment-overview-title")}</h3>
          <p>{$translation("environment-overview-description")}</p>
        </div>
      </header>
      <div class="summary-grid">
        {#each workspace?.summaries ?? [] as summary}
          <article class="summary-card">
            <header>
              <span
                >{environmentChannelLabel(
                  summary.activity_channel,
                  $translation,
                )}</span
              ><ContextHelp
                {...environmentActivityHelp(
                  workspace,
                  summary.activity_channel,
                  null,
                  $translation,
                )}
                placement="left"
              />
            </header>
            <strong
              >{summary.latest_recorded_value === null
                ? "—"
                : formatNumber(
                    summary.latest_recorded_value,
                    $activeLocale,
                  )}</strong
            >
            <small
              >{$translation(
                !summary.quantity_is_publishable
                  ? "environment-quantity-unverified"
                  : summary.latest_recorded_value === null
                    ? "environment-quantity-separated"
                    : "environment-quantity-publishable",
              )}</small
            >
          </article>
        {/each}
      </div>
      <section
        class="definition-context"
        aria-labelledby="environment-definition-heading"
      >
        <header>
          <h4 id="environment-definition-heading">
            {$translation("environment-definition-title")}
          </h4>
          <p>{$translation("environment-definition-description")}</p>
        </header>
        {#if workspace?.definition_context.available}
          <div class="summary-grid">
            <article class="summary-card">
              <span>{$translation("environment-definition-buildings")}</span>
              <strong
                >{formatNumber(
                  workspace.definition_context.building_count,
                  $activeLocale,
                )}</strong
              >
            </article>
            <article class="summary-card">
              <span>{$translation("environment-definition-pollution")}</span>
              <strong
                >{formatNumber(
                  workspace.definition_context.pollution_class_facts,
                  $activeLocale,
                )}</strong
              >
            </article>
            <article class="summary-card">
              <span>{$translation("environment-definition-sewage")}</span>
              <strong
                >{formatNumber(
                  workspace.definition_context.sewage_pollution_factors,
                  $activeLocale,
                )}</strong
              >
            </article>
            <article class="summary-card">
              <span>{$translation("environment-definition-water")}</span>
              <strong
                >{formatNumber(
                  workspace.definition_context.water_quality_facts,
                  $activeLocale,
                )}</strong
              >
            </article>
            <article class="summary-card">
              <span>{$translation("environment-definition-connections")}</span>
              <strong
                >{formatNumber(
                  workspace.definition_context.connection_capability_facts,
                  $activeLocale,
                )}</strong
              >
            </article>
          </div>
        {:else}
          <GuidanceSurface kind="instruction" layout="compact">
            <strong
              >{$translation(
                "environment-definition-unavailable-title",
              )}</strong
            >
            <span>{$translation("environment-definition-unavailable")}</span>
          </GuidanceSurface>
        {/if}
      </section>
      <div class="filters">
        <label
          >{$translation("environment-filter-channel")}<select
            value={selectedChannel}
            onchange={(event) => chooseChannel(event.currentTarget.value)}
            >{#each channels as channel}<option value={channel}
                >{environmentChannelLabel(channel, $translation)}</option
              >{/each}</select
          ></label
        >
        <label
          >{$translation("environment-filter-resource")}<select
            value={effectiveResource}
            onchange={(event) => chooseResource(event.currentTarget.value)}
            >{#each activityResources as resource}<option value={resource}
                >{resource}</option
              >{/each}</select
          ></label
        >
      </div>
      <ObservatoryChart
        spec={chart}
        eyebrow={$translation("environment-recorded-history")}
        help={environmentActivityHelp(
          workspace,
          selectedChannel,
          effectiveResource,
          $translation,
        )}
        navigation={chartNavigation}
        {onrelatednavigate}
      />
    </section>

    <section id="environment-pollution" class="panel" tabindex="-1">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">{$translation("environment-live-eyebrow")}</span
          >
          <h3>{$translation("environment-pollution-title")}</h3>
        </div>
      </header>
      <GuidanceSurface kind="instruction" layout="block"
        ><strong>{$translation("environment-live-unavailable-title")}</strong
        ><span>{$translation("environment-pollution-unavailable")}</span><button
          type="button"
          onclick={onopenresearch}
          >{$translation("environment-open-checked-session")}</button
        ></GuidanceSurface
      >
    </section>

    <section id="environment-waste" class="panel" tabindex="-1">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("environment-save-waste-eyebrow")}</span
          >
          <h3>{$translation("environment-waste-title")}</h3>
          <p>{$translation("environment-waste-description")}</p>
        </div>
      </header>
      <GuidanceSurface kind="boundary" layout="compact"
        ><strong>{$translation("environment-waste-boundary-title")}</strong
        ><span>{$translation("environment-waste-boundary")}</span
        ></GuidanceSurface
      >
      <div class="table-scroll">
        <table>
          <thead
            ><tr
              ><th>{$translation("environment-column-date")}</th><th
                >{$translation("environment-column-source")}</th
              ><th>{$translation("environment-column-resource")}</th><th
                >{$translation("environment-primary-source-value")}</th
              ><th>{$translation("environment-secondary-source-value")}</th></tr
            ></thead
          ><tbody
            >{#each wasteRows.slice(-250).reverse() as row}<tr
                ><td>{row.year} · {String(row.day).padStart(3, "0")}</td><td
                  >{environmentChannelLabel(
                    row.activity_channel,
                    $translation,
                  )}</td
                ><td
                  ><button
                    class="data-link"
                    type="button"
                    onclick={(event) =>
                      onrelatednavigate?.(
                        destinationsForSubject({
                          kind: "resource",
                          resourceToken: row.resource_token,
                        }),
                        event.currentTarget,
                      )}>{row.resource_token}</button
                  ></td
                ><td>{formatNumber(row.primary_value, $activeLocale)}</td><td
                  >{formatNumber(row.secondary_value, $activeLocale)}</td
                ></tr
              >{/each}</tbody
          >
        </table>
      </div>
    </section>

    <section id="environment-water" class="panel" tabindex="-1">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">{$translation("environment-live-eyebrow")}</span
          >
          <h3>{$translation("environment-water-title")}</h3>
        </div>
      </header>
      <GuidanceSurface kind="instruction" layout="block"
        ><strong>{$translation("environment-live-unavailable-title")}</strong
        ><span>{$translation("environment-water-unavailable")}</span><button
          type="button"
          onclick={onopenresearch}
          >{$translation("environment-open-checked-session")}</button
        ></GuidanceSurface
      >
    </section>

    <section id="environment-carbon" class="panel" tabindex="-1">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("environment-carbon-eyebrow")}</span
          >
          <h3>{$translation("environment-carbon-title")}</h3>
          <p>{$translation("environment-carbon-description")}</p>
        </div>
      </header>
      <GuidanceSurface kind="boundary" layout="compact"
        ><strong>{$translation("environment-carbon-boundary-title")}</strong
        ><span>{$translation("environment-carbon-boundary")}</span
        ></GuidanceSurface
      >
      {#if carbonEstimate?.available && carbonEstimate.estimated_grams_co2e !== null}
        <div class="carbon-result">
          <span>{$translation("environment-carbon-result")}</span><strong
            >{formatCo2e(
              carbonEstimate.estimated_grams_co2e,
              $activeLocale,
            )}</strong
          ><span
            >{$translation("environment-carbon-coverage", {
              coverage: carbonEstimate.coverage_percent.toFixed(1),
              covered: carbonEstimate.covered_rows,
              eligible: carbonEstimate.eligible_rows,
            })}</span
          >
        </div>
        <ObservatoryChart
          spec={carbonChart}
          eyebrow={$translation("environment-carbon-estimate-eyebrow")}
          help={carbonEstimateHelp(workspace, $translation)}
        />
        <div class="table-scroll">
          <table>
            <thead
              ><tr
                ><th>{$translation("environment-column-resource")}</th><th
                  >{$translation("environment-factor-channel")}</th
                ><th>{$translation("environment-carbon-quantity")}</th><th
                  >{$translation("environment-carbon-factor")}</th
                ><th>{$translation("environment-carbon-contribution")}</th></tr
              ></thead
            ><tbody
              >{#each carbonEstimate.contributions as contribution}<tr
                  ><td>{contribution.resource_token}</td><td
                    >{environmentChannelLabel(
                      contribution.activity_channel,
                      $translation,
                    )}</td
                  ><td
                    >{formatNumber(
                      contribution.recorded_quantity,
                      $activeLocale,
                    )}</td
                  ><td
                    >{formatNumber(
                      contribution.grams_co2e_per_unit,
                      $activeLocale,
                    )} g CO₂e</td
                  ><td
                    >{formatCo2e(
                      contribution.estimated_grams_co2e,
                      $activeLocale,
                    )}</td
                  ></tr
                >{/each}</tbody
            >
          </table>
        </div>
        {#if carbonEstimate.missing_factors.length}
          <GuidanceSurface kind="instruction" layout="compact">
            <strong>{$translation("environment-carbon-missing-title")}</strong>
            <span
              >{$translation("environment-carbon-missing", {
                count: carbonEstimate.missing_factors.length,
              })}</span
            >
          </GuidanceSurface>
        {/if}
      {:else}<p>{$translation("environment-carbon-no-estimate")}</p>{/if}
      <GuidanceSurface kind="instruction" layout="compact">
        <strong>{$translation("environment-carbon-intensity-title")}</strong>
        <span>{$translation("environment-carbon-intensity-unavailable")}</span>
      </GuidanceSurface>
      <div class="factor-editor">
        <label
          >{$translation("environment-factor-name")}<input
            bind:value={factorName}
          /></label
        >
        <label
          >{$translation("environment-factor-boundary")}<input
            bind:value={factorBoundary}
          /></label
        >
        <label
          >{$translation("environment-factor-reason")}<input
            bind:value={factorReason}
          /></label
        >
        <label
          >{$translation("environment-factor-resource")}<select
            bind:value={factorResource}
            ><option value=""
              >{$translation("environment-factor-choose-resource")}</option
            >{#each workspace?.resources ?? [] as resource}<option
                value={resource}>{resource}</option
              >{/each}</select
          ></label
        >
        <label
          >{$translation("environment-factor-channel")}<select
            bind:value={factorChannel}
            >{#each carbonChannels as channel}<option value={channel}
                >{environmentChannelLabel(channel, $translation)}</option
              >{/each}</select
          ></label
        >
        <label
          >{$translation("environment-factor-value")}<input
            type="number"
            min="0"
            step="any"
            bind:value={factorValue}
          /></label
        >
        <label
          >{$translation("environment-factor-source")}<input
            bind:value={factorSource}
          /></label
        >
        <label
          >{$translation("environment-factor-year")}<input
            type="number"
            min="1900"
            max="9999"
            bind:value={factorYear}
          /></label
        >
        <label
          >{$translation("environment-factor-entry-reason")}<input
            bind:value={factorEntryReason}
          /></label
        >
        <label
          >{$translation("environment-factor-reference")}<input
            bind:value={factorReference}
          /></label
        >
      </div>
      <div class="actions">
        <button type="button" onclick={addFactor}
          >{$translation("environment-factor-add")}</button
        ><button
          type="button"
          disabled={busy || factorEntries.length === 0}
          onclick={saveFactors}
          >{$translation("environment-factor-save")}</button
        >
      </div>
      {#if factorEntries.length}<p>
          {$translation("environment-factor-pending", {
            count: factorEntries.length,
          })}
        </p>{/if}
      <div class="revision-list">
        {#each workspace?.factor_sets ?? [] as set}<article>
            <strong>{set.name} · r{set.revision}</strong><span
              >{set.accounting_boundary}</span
            >
            <div class="actions">
              <button
                type="button"
                onclick={async () =>
                  onupdate(
                    await selectCarbonFactorSet(
                      set.factor_set_id,
                      set.revision,
                    ),
                  )}>{$translation("environment-factor-select")}</button
              ><button
                type="button"
                onclick={async () =>
                  onupdate(await rollbackCarbonFactorSet(set.factor_set_id))}
                >{$translation("environment-factor-rollback")}</button
              ><button
                type="button"
                onclick={() => exportFactors(set.factor_set_id, set.revision)}
                >{$translation("environment-factor-export")}</button
              ><button
                type="button"
                onclick={async () =>
                  onupdate(await removeCarbonFactorSet(set.factor_set_id))}
                >{$translation("action-remove")}</button
              >
            </div>
          </article>{/each}
      </div>
      <div class="csv-import">
        <label
          >{$translation("environment-factor-import")}<input
            type="file"
            accept=".csv,text/csv"
            onchange={readImport}
          /></label
        >{#if importPreview}<p>
            {$translation(
              importPreview.valid
                ? "environment-factor-import-valid"
                : "environment-factor-import-invalid",
              { count: importPreview.row_count },
            )}
          </p>
          <button
            type="button"
            disabled={!importPreview.valid}
            onclick={async () =>
              onupdate(await applyCarbonFactorImport(importCsv))}
            >{$translation("environment-factor-import-apply")}</button
          >{/if}
      </div>
    </section>

    <section id="environment-recording" class="panel" tabindex="-1">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("environment-recording-eyebrow")}</span
          >
          <h3>{$translation("environment-recording-title")}</h3>
          <p>{$translation("environment-recording-description")}</p>
        </div>
      </header>
      <GuidanceSurface kind="instruction" layout="block"
        ><strong
          >{$translation("environment-recording-status", {
            state: workspace?.recording.state ?? "disabled",
          })}</strong
        ><span>{$translation("environment-recording-contract-waiting")}</span
        ></GuidanceSurface
      >
      {#if workspace?.recording.enabled}<button
          type="button"
          onclick={async () => onupdate(await disableEnvironmentRecording())}
          >{$translation("environment-recording-disable")}</button
        >{:else}<label class="consent"
          ><input type="checkbox" bind:checked={consent} />{$translation(
            "environment-recording-consent",
          )}</label
        ><button
          type="button"
          disabled={!consent}
          onclick={async () => onupdate(await enableEnvironmentRecording(true))}
          >{$translation("environment-recording-enable")}</button
        >{/if}
      <div class="danger-zone">
        <h4>{$translation("environment-delete-title")}</h4>
        <p>{$translation("environment-delete-description")}</p>
        <input
          aria-label={$translation("environment-delete-confirmation-label")}
          bind:value={deleteConfirmation}
          placeholder="DELETE LIVE ENVIRONMENT RECORDINGS"
        /><button
          type="button"
          disabled={deleteConfirmation !== "DELETE LIVE ENVIRONMENT RECORDINGS"}
          onclick={async () => {
            onupdate(await deleteLiveEnvironmentRecordings(deleteConfirmation));
            deleteConfirmation = "";
          }}>{$translation("environment-delete-action")}</button
        >
      </div>
    </section>
  </section>
</section>

<style>
  .environment-workspace {
    display: grid;
    grid-template-columns: 236px minmax(580px, 1fr);
    min-height: 0;
  }
  .canvas {
    min-width: 0;
  }
  .panel {
    margin-bottom: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    background: var(--surface-1);
    scroll-margin-top: var(--space-2);
  }
  .summary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
    gap: var(--space-2);
  }
  .summary-card,
  .carbon-result,
  .revision-list article {
    display: grid;
    gap: 0.35rem;
    padding: var(--space-2);
    border: 1px solid var(--border);
    background: var(--surface-2);
  }
  .summary-card strong,
  .carbon-result strong {
    font-family: var(--font-display);
    font-size: 1.55rem;
  }
  .summary-card header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-1);
  }
  .filters,
  .factor-editor {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
    gap: var(--space-2);
    margin: var(--space-2) 0;
  }
  .definition-context {
    display: grid;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
  .definition-context h4,
  .definition-context p {
    margin: 0;
  }
  label {
    display: grid;
    gap: 0.35rem;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    margin-top: var(--space-1);
  }
  .revision-list {
    display: grid;
    gap: var(--space-1);
    margin-top: var(--space-2);
  }
  .table-scroll {
    overflow: auto;
    max-height: 28rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 0.55rem;
    border-bottom: 1px solid var(--border);
    text-align: left;
  }
  .data-link {
    padding: 0;
    border: 0;
    color: var(--accent-cyan);
    background: transparent;
    text-decoration: underline;
  }
  .progress-card {
    display: grid;
    gap: 0.5rem;
    padding: var(--space-2);
    border: 1px solid var(--accent-gold);
  }
  progress {
    width: 100%;
  }
  .danger-zone {
    margin-top: var(--space-3);
    padding: var(--space-2);
    border: 1px solid var(--status-error);
  }
  .consent {
    grid-template-columns: auto 1fr;
    align-items: start;
    margin: var(--space-2) 0;
  }
  @media (max-width: 1180px) {
    .environment-workspace {
      grid-template-columns: 210px minmax(540px, 1fr);
    }
  }
  @media (max-width: 860px) {
    .environment-workspace {
      grid-template-columns: 1fr;
    }
  }
</style>
