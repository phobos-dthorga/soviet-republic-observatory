<script lang="ts">
  import { onMount } from "svelte";
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import type { TranslationKey } from "../i18n/catalog";
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import { exactObservationChartBindings } from "../navigation/chartBindings";
  import { registerNavigationGuard } from "../navigation/navigationGuards";
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
  import WorkspaceTaskDrawer from "../tasks/WorkspaceTaskDrawer.svelte";
  import type { WorkspaceTaskRoute } from "../tasks/workspaceTaskRoutes";
  import ScopedFilterBar from "./ScopedFilterBar.svelte";
  import WorkspaceSectionHeader from "./WorkspaceSectionHeader.svelte";
  import WorkspaceToolbar from "./WorkspaceToolbar.svelte";

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
    activeTask = null,
    onopentask,
    onclosetask,
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
    activeTask?: WorkspaceTaskRoute | null;
    onopentask: (route: WorkspaceTaskRoute, origin?: HTMLElement) => void;
    onclosetask: () => void;
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
  let canvas = $state<HTMLElement | null>(null);
  let activeSection = $state<string>("environment-overview");

  const carbonDraftDirty = $derived(
    Boolean(
      factorName ||
      factorBoundary ||
      factorReason ||
      factorResource ||
      factorSource ||
      factorEntryReason ||
      factorReference ||
      factorEntries.length ||
      importCsv,
    ),
  );
  const managementDraftDirty = $derived(Boolean(deleteConfirmation));

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
  const checkedArchives = $derived(
    indexingProgress
      ? indexingProgress.completed_archives +
          indexingProgress.missing_archives +
          indexingProgress.changed_archives +
          indexingProgress.failed_archives +
          indexingProgress.duplicate_archives
      : 0,
  );
  const liveReadingSupported = $derived(
    Boolean(
      workspace?.source_availability.live_pollution ||
      workspace?.source_availability.live_radiation ||
      workspace?.source_availability.live_water_and_sewage,
    ),
  );

  function indexingTitleKey(
    phase: EnvironmentIndexingProgress["phase"],
  ): TranslationKey {
    if (phase === "complete") return "environment-index-progress-complete";
    if (phase === "paused") return "environment-index-progress-paused";
    if (phase === "failed") return "environment-index-progress-failed";
    return "environment-index-progress-active";
  }

  function recordingStateKey(): TranslationKey {
    if (workspace?.recording.state === "ready")
      return "environment-recording-state-ready";
    if (workspace?.recording.enabled)
      return "environment-recording-state-on-unavailable";
    if (!liveReadingSupported) return "environment-recording-state-unavailable";
    return "environment-recording-state-off";
  }

  $effect(() => {
    activeSection = location.section;
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
      clearCarbonDraft();
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

  function clearCarbonDraft(): void {
    factorName = "";
    factorBoundary = "";
    factorReason = "";
    factorResource = "";
    factorChannel = "production";
    factorValue = 0;
    factorSource = "";
    factorEntryReason = "";
    factorReference = "";
    factorEntries = [];
    importCsv = "";
    importPreview = null;
  }

  onMount(() =>
    registerNavigationGuard(
      "environment-task-drafts",
      () => carbonDraftDirty || managementDraftDirty,
    ),
  );

  onMount(() => {
    if (!canvas || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort(
            (left, right) => right.intersectionRatio - left.intersectionRatio,
          )[0];
        if (visible?.target.id) activeSection = visible.target.id;
      },
      {
        root: canvas,
        rootMargin: "-8% 0px -68% 0px",
        threshold: [0, 0.1, 0.4],
      },
    );
    for (const section of canvas.querySelectorAll<HTMLElement>(".panel[id]")) {
      observer.observe(section);
    }
    return () => observer.disconnect();
  });
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
      <a
        href="#environment-overview"
        aria-current={activeSection === "environment-overview"
          ? "location"
          : undefined}
        use:containedSectionNavigation
        ><span>01</span>{$translation("environment-section-overview")}</a
      >
      <a
        href="#environment-pollution"
        aria-current={activeSection === "environment-pollution"
          ? "location"
          : undefined}
        use:containedSectionNavigation
        ><span>02</span>{$translation("environment-section-pollution")}</a
      >
      <a
        href="#environment-waste"
        aria-current={activeSection === "environment-waste"
          ? "location"
          : undefined}
        use:containedSectionNavigation
        ><span>03</span>{$translation("environment-section-waste")}</a
      >
      <a
        href="#environment-water"
        aria-current={activeSection === "environment-water"
          ? "location"
          : undefined}
        use:containedSectionNavigation
        ><span>04</span>{$translation("environment-section-water")}</a
      >
      <a
        href="#environment-carbon"
        aria-current={activeSection === "environment-carbon"
          ? "location"
          : undefined}
        use:containedSectionNavigation
        ><span>05</span>{$translation("environment-section-carbon")}</a
      >
      <a
        href="#environment-recording"
        aria-current={activeSection === "environment-recording"
          ? "location"
          : undefined}
        use:containedSectionNavigation
        ><span>06</span>{$translation("environment-section-recording")}</a
      >
    </div>
    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note"
      ><span aria-hidden="true">◇</span>
      <p>{$translation("environment-sidebar-boundary")}</p></GuidanceSurface
    >
  </aside>

  <section class="canvas" bind:this={canvas}>
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong>{$translation("environment-evidence-banner")}</strong>
      <span>{$translation("environment-evidence-banner-detail")}</span>
    </GuidanceSurface>
    <WorkspaceSectionHeader
      level="page"
      eyebrow={$translation("environment-heading-eyebrow")}
      title={$translation("environment-heading-title")}
      description={$translation("environment-heading-description")}
    >
      {#snippet actions()}
        <WorkspaceToolbar label={$translation("environment-heading-title")}>
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
        </WorkspaceToolbar>
      {/snippet}
    </WorkspaceSectionHeader>

    {#if indexingProgress && indexingProgress.phase !== "idle"}
      <section
        class="progress-card"
        data-phase={indexingProgress.phase}
        aria-live="polite"
      >
        <strong
          >{$translation(indexingTitleKey(indexingProgress.phase), {
            current: indexingProgress.current_archive,
            checked: checkedArchives,
            total: indexingProgress.total_archives,
          })}</strong
        >
        <progress max="100" value={indexingProgress.progress_percent ?? 0}
        ></progress>
        {#if indexingProgress.phase === "complete"}
          <span
            >{$translation("environment-index-result", {
              added: indexingProgress.completed_archives,
              unchanged: indexingProgress.duplicate_archives,
              missing: indexingProgress.missing_archives,
              changed: indexingProgress.changed_archives,
              failed: indexingProgress.failed_archives,
            })}</span
          >
        {:else}
          <span
            >{$translation("environment-index-detail-active", {
              records: indexingProgress.records_processed,
              rows: indexingProgress.rows_processed,
            })}</span
          >
        {/if}
      </section>
    {/if}

    <section id="environment-overview" class="panel" tabindex="-1">
      <WorkspaceSectionHeader
        eyebrow={$translation("environment-overview-eyebrow")}
        title={$translation("environment-overview-title")}
        description={$translation("environment-overview-description")}
      />
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
      <ScopedFilterBar label={$translation("environment-recorded-history")}>
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
      </ScopedFilterBar>
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
      <WorkspaceSectionHeader
        eyebrow={$translation("environment-live-eyebrow")}
        title={$translation("environment-pollution-title")}
      />
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
      <WorkspaceSectionHeader
        eyebrow={$translation("environment-save-waste-eyebrow")}
        title={$translation("environment-waste-title")}
        description={$translation("environment-waste-description")}
      />
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
      <WorkspaceSectionHeader
        eyebrow={$translation("environment-live-eyebrow")}
        title={$translation("environment-water-title")}
      />
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
      <WorkspaceSectionHeader
        eyebrow={$translation("environment-carbon-eyebrow")}
        title={$translation("environment-carbon-title")}
        description={$translation("environment-carbon-description")}
      >
        {#snippet actions()}
          <button
            type="button"
            class="primary"
            onclick={(event) =>
              onopentask("environment-carbon-study", event.currentTarget)}
          >
            {$translation("environment-carbon-open-task")}
          </button>
        {/snippet}
      </WorkspaceSectionHeader>
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
    </section>

    <section id="environment-recording" class="panel" tabindex="-1">
      <WorkspaceSectionHeader
        eyebrow={$translation("environment-recording-eyebrow")}
        title={$translation("environment-recording-title")}
        description={$translation("environment-recording-description")}
      >
        {#snippet actions()}
          <button
            type="button"
            onclick={(event) =>
              onopentask(
                "environment-recording-management",
                event.currentTarget,
              )}
          >
            {$translation("environment-recording-manage")}
          </button>
        {/snippet}
      </WorkspaceSectionHeader>
      <section class="recording-card readable-card">
        <GuidanceSurface kind="instruction" layout="block">
          <strong>{$translation(recordingStateKey())}</strong>
          <span
            >{$translation(
              liveReadingSupported
                ? "environment-recording-ready-detail"
                : "environment-recording-contract-waiting",
            )}</span
          >
        </GuidanceSurface>
        {#if workspace?.recording.enabled}
          <div class="actions">
            <button
              type="button"
              onclick={async () =>
                onupdate(await disableEnvironmentRecording())}
              >{$translation("environment-recording-disable")}</button
            >
            <button type="button" onclick={onopenresearch}
              >{$translation("environment-open-checked-session")}</button
            >
          </div>
        {:else if liveReadingSupported}
          <label class="consent"
            ><input type="checkbox" bind:checked={consent} />{$translation(
              "environment-recording-consent",
            )}</label
          >
          <button
            type="button"
            disabled={!consent}
            onclick={async () =>
              onupdate(await enableEnvironmentRecording(true))}
            >{$translation("environment-recording-enable")}</button
          >
        {:else}
          <button type="button" onclick={onopenresearch}
            >{$translation("environment-recording-view-research")}</button
          >
        {/if}
      </section>
    </section>
  </section>

  <WorkspaceTaskDrawer
    open={activeTask === "environment-carbon-study"}
    route="environment-carbon-study"
    eyebrow={$translation("environment-carbon-eyebrow")}
    title={$translation("environment-carbon-task-title")}
    description={$translation("environment-carbon-task-description")}
    closeLabel={$translation("action-close")}
    onclose={onclosetask}
  >
    <div class="task-form">
      <fieldset class="factor-group study-details">
        <legend>{$translation("environment-factor-study-heading")}</legend>
        <p>{$translation("environment-factor-study-description")}</p>
        <div class="factor-fields study-fields">
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
        </div>
      </fieldset>
      <fieldset class="factor-group factor-details">
        <legend>{$translation("environment-factor-entry-heading")}</legend>
        <p>{$translation("environment-factor-entry-description")}</p>
        <div class="factor-fields entry-fields">
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
          <label class="wide-field"
            >{$translation("environment-factor-entry-reason")}<input
              bind:value={factorEntryReason}
            /></label
          >
          <label class="wide-field"
            >{$translation("environment-factor-reference")}<input
              bind:value={factorReference}
            /></label
          >
        </div>
      </fieldset>
      <div class="actions factor-actions">
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
    </div>
  </WorkspaceTaskDrawer>

  <WorkspaceTaskDrawer
    open={activeTask === "environment-recording-management"}
    route="environment-recording-management"
    eyebrow={$translation("environment-delete-eyebrow")}
    title={$translation("environment-delete-title")}
    description={$translation("environment-delete-description")}
    closeLabel={$translation("action-close")}
    onclose={onclosetask}
  >
    <div class="danger-zone">
      <GuidanceSurface kind="boundary" layout="block">
        <strong>{$translation("environment-delete-title")}</strong>
        <span>{$translation("environment-delete-description")}</span>
      </GuidanceSurface>
      <div class="danger-actions">
        <input
          aria-label={$translation("environment-delete-confirmation-label")}
          bind:value={deleteConfirmation}
          placeholder="DELETE LIVE ENVIRONMENT RECORDINGS"
        />
        <button
          type="button"
          disabled={deleteConfirmation !== "DELETE LIVE ENVIRONMENT RECORDINGS"}
          onclick={async () => {
            onupdate(await deleteLiveEnvironmentRecordings(deleteConfirmation));
            deleteConfirmation = "";
            onclosetask();
          }}>{$translation("environment-delete-action")}</button
        >
      </div>
    </div>
  </WorkspaceTaskDrawer>
</section>

<style>
  .environment-workspace {
    display: grid;
    grid-template-columns: 236px minmax(580px, 1fr);
    min-height: 0;
    --environment-space-1: 8px;
    --environment-space-2: 12px;
    --environment-space-3: 18px;
  }
  .canvas {
    min-width: 0;
  }
  .panel {
    margin-bottom: var(--environment-space-3);
    padding: var(--environment-space-3);
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
    scroll-margin-top: var(--environment-space-2);
  }
  .summary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 18rem));
    gap: var(--environment-space-2);
    justify-content: start;
  }
  .summary-card,
  .carbon-result,
  .revision-list article {
    display: grid;
    gap: 0.35rem;
    padding: var(--environment-space-2);
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
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
    gap: var(--environment-space-1);
  }
  .summary-card small {
    color: var(--colour-muted);
    line-height: 1.45;
  }
  .definition-context {
    display: grid;
    gap: var(--environment-space-2);
    margin-top: var(--environment-space-3);
  }
  .definition-context h4,
  .definition-context p {
    margin: 0;
  }
  .definition-context .summary-grid {
    grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  }
  label {
    display: grid;
    gap: 0.35rem;
  }
  label > :is(input, select) {
    width: 100%;
    min-width: 0;
  }
  .factor-group,
  .recording-card {
    min-width: 0;
    margin: 0;
    border: 1px solid var(--colour-line-faint);
    padding: var(--environment-space-2);
    background: var(--colour-surface-raised);
  }
  .factor-group legend {
    padding-inline: 0.35rem;
    color: var(--colour-gold);
    font-family: var(--font-display);
    font-size: 1.1rem;
  }
  .factor-group > p {
    margin: 0 0 var(--environment-space-2);
    color: var(--colour-muted);
    line-height: 1.45;
  }
  .factor-fields {
    display: grid;
    gap: var(--environment-space-2);
  }
  .study-fields {
    grid-template-columns: 1fr;
  }
  .entry-fields {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .entry-fields .wide-field {
    grid-column: span 2;
  }
  .factor-actions {
    justify-content: flex-end;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--environment-space-1);
    margin-top: var(--environment-space-1);
  }
  .revision-list {
    display: grid;
    gap: var(--environment-space-1);
    margin-top: var(--environment-space-2);
  }
  .task-form {
    width: min(100%, 66rem);
    display: grid;
    gap: var(--environment-space-2);
  }
  .readable-card {
    width: min(100%, 72ch);
    margin-top: var(--environment-space-2);
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
    border-bottom: 1px solid var(--colour-line-faint);
    text-align: left;
  }
  .data-link {
    min-height: 24px;
    padding: 2px 0;
    border: 0;
    color: var(--colour-observed);
    background: transparent;
    text-decoration: underline;
  }
  .progress-card {
    display: grid;
    gap: 0.5rem;
    padding: var(--environment-space-2);
    border: 1px solid var(--colour-gold);
  }
  .progress-card[data-phase="complete"] {
    border-color: var(--colour-observed);
  }
  progress {
    width: 100%;
  }
  .danger-zone {
    margin: 0;
    padding: var(--environment-space-2);
    border: 1px solid var(--colour-risk);
    background: color-mix(
      in srgb,
      var(--colour-risk) 5%,
      var(--colour-surface-raised)
    );
  }
  .danger-actions {
    display: grid;
    grid-template-columns: minmax(12rem, 1fr) auto;
    gap: var(--environment-space-1);
  }
  .consent {
    grid-template-columns: auto 1fr;
    align-items: start;
    margin: var(--environment-space-2) 0;
  }
  @media (max-width: 1180px) {
    .environment-workspace {
      grid-template-columns: 210px minmax(540px, 1fr);
    }
    .entry-fields {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 860px) {
    .environment-workspace {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 620px) {
    .entry-fields,
    .danger-actions {
      grid-template-columns: 1fr;
    }
    .entry-fields .wide-field {
      grid-column: auto;
    }
  }
</style>
