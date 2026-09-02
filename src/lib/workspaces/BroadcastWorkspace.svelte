<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import type { TranslationKey } from "../i18n/catalog";
  import { formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import { exactObservationChartBindings } from "../navigation/chartBindings";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import {
    defaultWorkspaceLocation,
    destinationsForSubject,
    electronicsEconomyDestinations,
    type ChartNavigationBinding,
    type RelatedDataDestination,
    type WorkspaceFilters,
    type WorkspaceLocation,
  } from "../navigation/relatedData";
  import ReceiverEvidence from "../observations/ReceiverEvidence.svelte";
  import type {
    BroadcastIndexingProgress,
    BroadcastOutcomeModel,
    BroadcastOutcomeRequest,
    BroadcastWorkspaceModel,
    PublishedMetricContext,
  } from "../observations/types";
  import {
    broadcastIndexingChecked,
    broadcastIndexingHasIssues,
    broadcastMetricLabel,
    broadcastOutcomeAvailabilityLabel,
    createBroadcastOutcomeChart,
  } from "../presentation/broadcast";
  import {
    metricContextHelpFor,
    publishedMetricContext,
  } from "../presentation/metricContext";
  import { createObservedReceiverChart } from "../presentation/receiverObservation";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";

  let {
    workspace = null,
    outcome = null,
    indexingProgress = null,
    desktopAvailable = false,
    metricContexts = [],
    location,
    onlocationchange,
    onrelatednavigate,
    onoutcomerequest,
    onindexrequest,
  }: {
    workspace?: BroadcastWorkspaceModel | null;
    outcome?: BroadcastOutcomeModel | null;
    indexingProgress?: BroadcastIndexingProgress | null;
    desktopAvailable?: boolean;
    metricContexts?: PublishedMetricContext[];
    location: WorkspaceLocation;
    onlocationchange?: (filters: WorkspaceFilters) => void;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
    onoutcomerequest?: (
      request: BroadcastOutcomeRequest,
    ) => Promise<BroadcastOutcomeModel | null>;
    onindexrequest?: (resume: boolean) => Promise<void>;
  } = $props();

  const sections: Array<{
    label: TranslationKey;
    href: string;
    marker: string;
  }> = [
    { label: "broadcast-section-pulse", href: "#pulse", marker: "01" },
    { label: "broadcast-section-receivers", href: "#receivers", marker: "02" },
    { label: "broadcast-section-audience", href: "#audience", marker: "03" },
    { label: "broadcast-section-programme", href: "#programme", marker: "04" },
    { label: "broadcast-section-outcomes", href: "#outcomes", marker: "05" },
    { label: "broadcast-section-bulletin", href: "#bulletin", marker: "06" },
  ];
  const stationIds = ["radio", "television"] as const;
  const receiverMetricIds = [
    "core.citizens.electronics.none",
    "core.citizens.electronics.radio",
    "core.citizens.electronics.television",
    "core.citizens.electronics.computer",
  ];
  const lags = [0, 1, 2, 4, 8] as const;
  const electronicsResources = [
    {
      token: "eletronics" as const,
      label: "broadcast-related-electronics" as const,
    },
    {
      token: "ecomponents" as const,
      label: "broadcast-related-electronic-components" as const,
    },
  ];
  let selectedStation = $state<(typeof stationIds)[number]>("radio");
  let selectedReceiverMetric = $state(receiverMetricIds[1]);
  let selectedStatusMetric = $state("core.citizens.status.happiness");
  let selectedLag = $state<(typeof lags)[number]>(0);
  let localOutcome = $state<BroadcastOutcomeModel | null>(null);
  let outcomeBusy = $state(false);
  let indexingBusy = $state(false);
  const receiverDataset = $derived(workspace?.receiver ?? null);
  const latestReceiverPoint = $derived(receiverDataset?.points.at(-1) ?? null);
  const receiverLadder = $derived(
    receiverDataset
      ? createObservedReceiverChart(receiverDataset, $translation)
      : null,
  );
  const receiverNavigation = $derived(
    receiverDataset && receiverLadder
      ? expandExactNavigation(
          exactObservationChartBindings(
            receiverLadder,
            receiverDataset.points,
            defaultWorkspaceLocation("broadcast"),
          ),
        )
      : [],
  );
  const receiverHelp = $derived.by(() => {
    const context = publishedMetricContext(
      metricContexts,
      "core.citizens.electronics.classified_total",
      "history",
    );
    return context
      ? metricContextHelpFor(
          "core.citizens.electronics.classified_total",
          context,
          $translation,
          (metricId) => broadcastMetricLabel(metricId, $translation),
        )
      : null;
  });
  const outcomeChart = $derived(
    localOutcome?.availability === "available"
      ? createBroadcastOutcomeChart(localOutcome, $translation)
      : null,
  );
  const outcomeNavigation = $derived(
    localOutcome && outcomeChart
      ? expandExactNavigation(
          exactObservationChartBindings(
            outcomeChart,
            localOutcome.pairs.map((pair) => ({
              game_day: pair.status_game_day,
              exact_observation: pair.exact_observation,
            })),
            {
              ...defaultWorkspaceLocation("broadcast"),
              section: "outcomes",
            },
          ),
        )
      : [],
  );
  const outcomeHelp = $derived.by(() => {
    const context = publishedMetricContext(
      metricContexts,
      selectedStatusMetric,
      "history",
    );
    return context
      ? metricContextHelpFor(
          selectedStatusMetric,
          context,
          $translation,
          (metricId) => broadcastMetricLabel(metricId, $translation),
        )
      : null;
  });
  const selectedRequirement = $derived(
    workspace?.station_requirements.find(
      (requirement) => requirement.station_kind === selectedStation,
    ) ?? null,
  );
  const leadingReceiver = $derived(
    workspace?.pulse?.classes.reduce((leading, item) =>
      item.count > leading.count ? item : leading,
    ) ?? null,
  );
  const indexingActive = $derived(
    indexingProgress !== null &&
      !["idle", "complete", "failed", "paused"].includes(
        indexingProgress.phase,
      ),
  );
  const indexingChecked = $derived(
    indexingProgress ? broadcastIndexingChecked(indexingProgress) : 0,
  );
  const indexingHasIssues = $derived(
    indexingProgress ? broadcastIndexingHasIssues(indexingProgress) : false,
  );

  $effect(() => {
    localOutcome = outcome;
  });

  $effect(() => {
    const station = location.filters.stationId;
    if (station === "radio" || station === "television") {
      selectedStation = station;
    }
  });

  function selectStation(stationId: (typeof stationIds)[number]): void {
    selectedStation = stationId;
    onlocationchange?.({ stationId });
  }

  function expandExactNavigation(
    bindings: ChartNavigationBinding[],
  ): ChartNavigationBinding[] {
    return bindings.map((binding) => {
      const reference = binding.destinations[0]?.exactObservation;
      if (!reference) return binding;
      return {
        ...binding,
        destinations: destinationsForSubject({
          kind: "observation",
          reference,
        }).filter(
          (destination) =>
            destination.location.workspace === "broadcast" ||
            destination.location.workspace === "archive",
        ),
      };
    });
  }

  function openElectronicsContext(
    resourceToken: "eletronics" | "ecomponents",
    origin: HTMLElement,
  ): void {
    onrelatednavigate?.(electronicsEconomyDestinations(resourceToken), origin);
  }

  function stationName(station: (typeof stationIds)[number]): string {
    return $translation(
      station === "radio" ? "station-radio" : "station-television",
    );
  }

  function formatChange(value: number): string {
    const formatted = formatNumber(Math.abs(value), $activeLocale);
    return `${value > 0 ? "+" : value < 0 ? "−" : ""}${formatted}`;
  }

  function dateRange(value: BroadcastOutcomeModel): string {
    if (
      value.start_year === null ||
      value.start_day === null ||
      value.end_year === null ||
      value.end_day === null
    ) {
      return "—";
    }
    return `${value.start_year} · ${String(value.start_day).padStart(3, "0")} — ${value.end_year} · ${String(value.end_day).padStart(3, "0")}`;
  }

  async function runOutcome(): Promise<void> {
    if (!onoutcomerequest || outcomeBusy) return;
    outcomeBusy = true;
    try {
      localOutcome = await onoutcomerequest({
        receiver_metric_id: selectedReceiverMetric,
        status_metric_id: selectedStatusMetric,
        lag_confirmed_records: selectedLag,
      });
    } finally {
      outcomeBusy = false;
    }
  }

  async function runIndexing(): Promise<void> {
    if (!onindexrequest || indexingBusy || indexingActive) return;
    indexingBusy = true;
    try {
      await onindexrequest(indexingProgress?.phase === "paused");
    } finally {
      indexingBusy = false;
    }
  }
</script>

<section class="workspace broadcast-workspace">
  <aside
    class="navigator"
    aria-label={$translation("broadcast-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("broadcast-editorial-desk")}</span>
        <h2>{$translation("nav-broadcast")}</h2>
      </div>
      <span class="edition">v2</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span>
        <strong>{workspace?.analysis_context.selected_branch_id ?? "—"}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("filter-window")}</span>
        <strong>{workspace?.status_coverage?.chartable_records ?? 0}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("filter-scope")}</span>
        <strong>{$translation("filter-whole-republic")}</strong>
      </div>
    </div>
    <div class="section-list">
      {#each sections as section}
        <a href={section.href} use:containedSectionNavigation>
          <span>{section.marker}</span>{$translation(section.label)}
        </a>
      {/each}
    </div>
    <GuidanceSurface kind="help" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("broadcast-evidence-sidebar-note")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface kind="boundary" layout="inline" semanticRole="status">
      <strong>{$translation("broadcast-evidence-desk")}</strong>
      <span>{$translation("broadcast-evidence-desk-detail")}</span>
    </GuidanceSurface>
    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("broadcast-heading-eyebrow")}</span>
        <h2>{$translation("broadcast-heading-title")}</h2>
        <p>{$translation("broadcast-heading-description")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("briefing-exact-head")}</span>
        <strong
          >{latestReceiverPoint?.year ?? "—"} · {latestReceiverPoint
            ? String(latestReceiverPoint.day).padStart(3, "0")
            : "—"}</strong
        >
        <small
          >{receiverDataset?.source_file_name ??
            $translation("chart-unavailable")}</small
        >
      </div>
    </header>

    {#if workspace && !workspace.warehouse_projection_available}
      <GuidanceSurface kind="help" layout="compact">
        <strong>{$translation("broadcast-analysis-database-delayed")}</strong>
      </GuidanceSurface>
    {/if}

    <section id="pulse" class="broadcast-panel">
      <span class="eyebrow">{$translation("broadcast-section-pulse")}</span>
      <h2>{$translation("broadcast-pulse-title")}</h2>
      <p>{$translation("broadcast-pulse-detail")}</p>
      {#if workspace?.pulse}
        <div class="pulse-grid">
          {#each workspace.pulse.classes as item}
            <article>
              <span>{broadcastMetricLabel(item.metric_id, $translation)}</span>
              <strong>{formatNumber(item.count, $activeLocale)}</strong>
              <small
                >{$translation("broadcast-pulse-share", {
                  share: item.share_percent.toFixed(1),
                })}</small
              >
              <em
                >{item.change_from_previous === 0
                  ? $translation("broadcast-pulse-no-change")
                  : item.change_from_previous === null
                    ? "—"
                    : $translation("broadcast-pulse-change", {
                        change: formatChange(item.change_from_previous),
                      })}</em
              >
            </article>
          {/each}
          <article class="classified-total">
            <span>{$translation("broadcast-pulse-classified")}</span>
            <strong
              >{formatNumber(
                workspace.pulse.classified_population,
                $activeLocale,
              )}</strong
            >
            <small>{$translation("evidence-save-fact")}</small>
          </article>
        </div>
      {:else}
        <GuidanceSurface kind="help" layout="block">
          <strong>{$translation("broadcast-no-receiver-title")}</strong>
          <span>{$translation("broadcast-no-receiver-detail")}</span>
        </GuidanceSurface>
      {/if}
    </section>

    <section id="receivers" class="broadcast-panel">
      {#if receiverLadder && receiverDataset}
        <ObservatoryChart
          spec={receiverLadder}
          height="285px"
          eyebrow={$translation("broadcast-section-receivers")}
          help={receiverHelp}
          navigation={receiverNavigation}
          {onrelatednavigate}
        />
        <ReceiverEvidence dataset={receiverDataset} />
        <GuidanceSurface kind="help" layout="block" class="related-economy">
          <div>
            <strong>{$translation("broadcast-related-economy-title")}</strong>
            <span>{$translation("broadcast-related-economy-detail")}</span>
            <small>{$translation("broadcast-related-economy-boundary")}</small>
          </div>
          <div class="related-economy-actions">
            {#each electronicsResources as resource}
              <button
                id={`broadcast-related-${resource.token}`}
                type="button"
                class="related-data-link"
                onclick={(event) =>
                  openElectronicsContext(resource.token, event.currentTarget)}
                >{$translation(resource.label)}</button
              >
            {/each}
          </div>
        </GuidanceSurface>
      {:else}
        <GuidanceSurface kind="help" layout="block">
          <strong>{$translation("broadcast-no-receiver-title")}</strong>
          <span>{$translation("broadcast-no-receiver-detail")}</span>
        </GuidanceSurface>
      {/if}
    </section>

    <section
      class="broadcast-panel index-panel"
      aria-labelledby="broadcast-index-heading"
    >
      <div>
        <span class="eyebrow">{$translation("broadcast-index-eyebrow")}</span>
        <h2 id="broadcast-index-heading">
          {$translation("broadcast-index-title")}
        </h2>
        <p>{$translation("broadcast-index-detail")}</p>
      </div>
      {#if indexingProgress && indexingProgress.total_archives > 0}
        <div class="index-progress" aria-live="polite">
          <strong
            >{$translation("broadcast-index-progress", {
              completed: indexingChecked,
              total: indexingProgress.total_archives,
            })}</strong
          >
          <progress max="100" value={indexingProgress.progress_percent ?? 0}
          ></progress>
          {#if indexingChecked > 0}
            <span class="index-breakdown"
              >{$translation("broadcast-index-breakdown", {
                added: indexingProgress.completed_archives,
                current: indexingProgress.duplicate_archives,
                missing: indexingProgress.missing_archives,
                changed: indexingProgress.changed_archives,
                failed: indexingProgress.failed_archives,
              })}</span
            >
          {/if}
          {#if indexingProgress.phase === "paused"}
            <span>{$translation("broadcast-index-paused")}</span>
          {:else if indexingProgress.phase === "failed"}
            <span>{$translation("broadcast-index-failed")}</span>
          {:else if indexingProgress.phase === "complete"}
            <span
              >{$translation(
                indexingHasIssues
                  ? "broadcast-index-complete-with-issues"
                  : "broadcast-index-current",
              )}</span
            >
          {/if}
        </div>
      {/if}
      <button
        type="button"
        class="primary-action"
        disabled={!desktopAvailable || indexingBusy || indexingActive}
        onclick={() => void runIndexing()}
        >{indexingProgress?.phase === "paused"
          ? $translation("broadcast-index-resume")
          : $translation("broadcast-index-action")}</button
      >
    </section>

    <section id="audience" class="broadcast-panel">
      <span class="eyebrow">{$translation("broadcast-section-audience")}</span>
      <h2>{$translation("broadcast-audience-context-title")}</h2>
      <p>{$translation("broadcast-audience-context-detail")}</p>
      {#if workspace?.station_requirements.length}
        <div class="station-requirements">
          {#each workspace.station_requirements as requirement}
            <article>
              <strong
                >{$translation("broadcast-station-requirement", {
                  station: stationName(
                    requirement.station_kind as "radio" | "television",
                  ),
                })}</strong
              >
              <span
                >{$translation("broadcast-station-workers", {
                  count: requirement.workers,
                })}</span
              >
              <span
                >{$translation("broadcast-station-professors", {
                  count: requirement.professors,
                })}</span
              >
            </article>
          {/each}
        </div>
      {:else}
        <GuidanceSurface kind="help" layout="compact">
          <strong
            >{$translation(
              "broadcast-station-requirements-unavailable",
            )}</strong
          >
        </GuidanceSurface>
      {/if}
      <GuidanceSurface kind="instruction" layout="block">
        <strong>{$translation("broadcast-audience-unavailable-title")}</strong>
        <span>{$translation("broadcast-audience-unavailable-detail")}</span>
      </GuidanceSurface>
    </section>

    <section id="programme" class="broadcast-panel">
      <span class="eyebrow">{$translation("broadcast-section-programme")}</span>
      <h2>{$translation("broadcast-programme-unavailable-title")}</h2>
      <div class="boundary-grid">
        <GuidanceSurface kind="help" layout="block">
          <strong>{$translation("broadcast-programme-mix-boundary")}</strong>
          <span>{$translation("broadcast-programme-mix-boundary-detail")}</span>
        </GuidanceSurface>
        <GuidanceSurface kind="help" layout="block">
          <strong>{$translation("broadcast-influence-boundary")}</strong>
          <span>{$translation("broadcast-influence-boundary-detail")}</span>
        </GuidanceSurface>
      </div>
    </section>

    <section id="outcomes" class="broadcast-panel outcome-panel">
      <span class="eyebrow">{$translation("broadcast-section-outcomes")}</span>
      <h2>{$translation("broadcast-outcome-title")}</h2>
      <p>{$translation("broadcast-outcome-detail")}</p>
      <div class="outcome-controls">
        <label>
          <span>{$translation("broadcast-outcome-receiver-label")}</span>
          <select bind:value={selectedReceiverMetric}>
            {#each receiverMetricIds as metricId}
              <option value={metricId}
                >{broadcastMetricLabel(metricId, $translation)}</option
              >
            {/each}
          </select>
        </label>
        <label>
          <span>{$translation("broadcast-outcome-status-label")}</span>
          <select bind:value={selectedStatusMetric}>
            {#each workspace?.status_metrics ?? [] as metric}
              <option value={metric.metric_id}
                >{broadcastMetricLabel(metric.metric_id, $translation)}</option
              >
            {/each}
          </select>
        </label>
        <label>
          <span>{$translation("broadcast-outcome-lag-label")}</span>
          <select bind:value={selectedLag}>
            {#each lags as lag}
              <option value={lag}
                >{lag === 0
                  ? $translation("broadcast-outcome-lag-zero")
                  : $translation("broadcast-outcome-lag-count", {
                      count: lag,
                    })}</option
              >
            {/each}
          </select>
        </label>
        <button
          type="button"
          class="primary-action"
          disabled={!workspace?.receiver || outcomeBusy || !onoutcomerequest}
          onclick={() => void runOutcome()}
          >{outcomeBusy
            ? $translation("broadcast-outcome-running")
            : $translation("broadcast-outcome-run")}</button
        >
      </div>

      {#if localOutcome}
        <GuidanceSurface
          kind={localOutcome.availability === "available"
            ? "help"
            : "instruction"}
          layout="compact"
          semanticRole="status"
        >
          <strong
            >{broadcastOutcomeAvailabilityLabel(
              localOutcome.availability,
              $translation,
            )}</strong
          >
        </GuidanceSurface>
        <div class="outcome-summary">
          <article>
            <span>{$translation("broadcast-outcome-score")}</span>
            <strong>{localOutcome.coefficient?.toFixed(3) ?? "—"}</strong>
            <small>{$translation("broadcast-outcome-score-detail")}</small>
          </article>
          <article>
            <span>{$translation("broadcast-outcome-pairs")}</span>
            <strong
              >{formatNumber(localOutcome.pair_count, $activeLocale)}</strong
            >
            <small
              >{$translation("broadcast-outcome-pair-count", {
                count: localOutcome.pair_count,
              })}</small
            >
          </article>
          <article>
            <span>{$translation("broadcast-outcome-span")}</span>
            <strong>{dateRange(localOutcome)}</strong>
          </article>
          <article>
            <span>{$translation("broadcast-outcome-cadence")}</span>
            <strong
              >{localOutcome.elapsed_days_median === null
                ? "—"
                : $translation("broadcast-outcome-days", {
                    days: localOutcome.elapsed_days_median.toFixed(1),
                  })}</strong
            >
          </article>
        </div>
      {/if}

      {#if outcomeChart}
        <ObservatoryChart
          spec={outcomeChart}
          height="300px"
          eyebrow={$translation("broadcast-section-outcomes")}
          help={outcomeHelp}
          navigation={outcomeNavigation}
          {onrelatednavigate}
        />
      {/if}
      <GuidanceSurface kind="boundary" layout="block">
        <strong>{$translation("broadcast-outcome-boundary-title")}</strong>
        <span>{$translation("broadcast-outcome-boundary-detail")}</span>
      </GuidanceSurface>
    </section>

    <section class="bulletin-panel" id="bulletin">
      <div class="bulletin-masthead">
        <span>{$translation("broadcast-evening-service")}</span>
        <strong>{$translation("broadcast-republic-signal")}</strong>
        <time
          >{latestReceiverPoint
            ? $translation("observation-game-date-compact", {
                year: latestReceiverPoint.year,
                day: String(latestReceiverPoint.day).padStart(3, "0"),
              })
            : $translation("chart-unavailable")}</time
        >
      </div>
      <div class="bulletin-body">
        <div class="dispatch-seal" aria-hidden="true">20</div>
        <div>
          <span class="eyebrow"
            >{$translation("broadcast-evidence-bulletin-eyebrow")}</span
          >
          <h2>
            {$translation(
              latestReceiverPoint
                ? "broadcast-bulletin-evidence-title"
                : "broadcast-bulletin-unavailable-title",
            )}
          </h2>
          <p>
            {latestReceiverPoint
              ? $translation("broadcast-bulletin-evidence-body", {
                  count: formatNumber(
                    latestReceiverPoint.classified_total,
                    $activeLocale,
                  ),
                })
              : $translation("broadcast-bulletin-unavailable-body")}
          </p>
          {#if leadingReceiver}
            <p>
              {$translation("broadcast-bulletin-leading-group", {
                group: broadcastMetricLabel(
                  leadingReceiver.metric_id,
                  $translation,
                ),
                share: leadingReceiver.share_percent.toFixed(1),
              })}
            </p>
          {/if}
          {#if localOutcome?.availability === "available" && localOutcome.coefficient !== null}
            <p>
              {$translation("broadcast-bulletin-pattern", {
                score: localOutcome.coefficient.toFixed(3),
                count: localOutcome.pair_count,
              })}
            </p>
          {/if}
          <div class="dispatch-links">
            <a href="#receivers" use:containedSectionNavigation
              >{$translation("broadcast-receiver-evidence")}</a
            >
            <a href="#outcomes" use:containedSectionNavigation
              >{$translation("broadcast-outcome-caveats")}</a
            >
          </div>
        </div>
      </div>
    </section>
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("broadcast-station-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("broadcast-station-inspector")}</span
        >
        <h2>{stationName(selectedStation)}</h2>
      </div>
      <span class="status-chip" data-status="watch"
        >{$translation("evidence-research")}</span
      >
    </div>
    <div
      class="station-switch"
      aria-label={$translation("broadcast-select-station")}
    >
      {#each stationIds as stationId}
        <button
          type="button"
          aria-pressed={selectedStation === stationId}
          class:active={selectedStation === stationId}
          onclick={() => selectStation(stationId)}
          >{stationName(stationId)}</button
        >
      {/each}
    </div>
    <div class="selected-reading">
      <span>{$translation("broadcast-station-telemetry")}</span>
      <strong>—</strong>
      <small>{$translation("chart-unavailable")}</small>
      <p>{$translation("broadcast-station-telemetry-detail")}</p>
    </div>
    <div class="fact-grid">
      <article>
        <span>{$translation("station-workers")}</span>
        <strong>{selectedRequirement?.workers ?? "—"}</strong>
      </article>
      <article>
        <span>{$translation("station-professors")}</span>
        <strong>{selectedRequirement?.professors ?? "—"}</strong>
      </article>
      <article>
        <span>{$translation("station-potential-reach")}</span>
        <strong>—</strong>
      </article>
      <article>
        <span>{$translation("station-current-audience")}</span>
        <strong>—</strong>
      </article>
    </div>
    <section class="evidence-ledger">
      <span class="eyebrow">{$translation("evidence-ledger")}</span>
      <div>
        <strong>{$translation("receiver-class")}</strong>
        <span
          >{$translation(
            receiverDataset
              ? "evidence-plain-text-save-fact"
              : "chart-unavailable",
          )}</span
        >
      </div>
      <div>
        <strong>{$translation("station-staffing-capacity")}</strong>
        <span
          >{$translation(
            selectedRequirement
              ? "evidence-game-definition"
              : "chart-unavailable",
          )}</span
        >
      </div>
      <div>
        <strong>{$translation("station-state")}</strong>
        <span>{$translation("evidence-binary-research-candidate")}</span>
      </div>
      <div>
        <strong>{$translation("station-outcome-attribution")}</strong>
        <span>{$translation("causality-experimental-prohibited")}</span>
      </div>
    </section>
  </aside>
</section>

<style>
  .broadcast-panel {
    margin-top: 10px;
    border: 1px solid var(--colour-line-faint);
    padding: 17px;
    background: var(--colour-surface-raised);
    scroll-margin-top: 18px;
  }

  .broadcast-panel > h2 {
    margin: 5px 0 6px;
    font-size: 22px;
  }

  .broadcast-panel > p {
    margin: 0 0 14px;
    color: var(--colour-muted);
  }

  .pulse-grid,
  .outcome-summary,
  .station-requirements {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  .pulse-grid article,
  .outcome-summary article,
  .station-requirements article {
    display: grid;
    align-content: start;
    gap: 5px;
    min-height: 98px;
    border: 1px solid var(--colour-line-faint);
    padding: 12px;
    background: var(--colour-surface-soft);
  }

  .pulse-grid strong,
  .outcome-summary strong {
    font-family: var(--font-display);
    font-size: 25px;
  }

  .pulse-grid small,
  .pulse-grid em,
  .outcome-summary small,
  .station-requirements span {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    font-style: normal;
  }

  .classified-total {
    border-color: var(--colour-observed) !important;
  }

  .boundary-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .index-panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 0.5fr) auto;
    align-items: center;
    gap: 14px;
  }

  .index-panel h2 {
    margin: 5px 0;
  }

  .index-panel p {
    margin: 0;
    color: var(--colour-muted);
  }

  .index-progress {
    display: grid;
    gap: 6px;
  }

  .index-progress progress {
    width: 100%;
  }

  .index-progress span {
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }

  .index-progress .index-breakdown {
    color: var(--colour-text);
  }

  .outcome-controls {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr)) auto;
    align-items: end;
    gap: 8px;
    margin-bottom: 10px;
  }

  .outcome-controls label {
    display: grid;
    gap: 5px;
  }

  .outcome-controls select {
    width: 100%;
  }

  .outcome-summary {
    margin: 10px 0;
  }

  :global(.related-economy) {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    margin-top: 8px;
  }

  :global(.related-economy > div:first-child) {
    display: grid;
    gap: 4px;
  }

  :global(.related-economy small) {
    color: var(--colour-muted);
  }

  .related-economy-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  @media (max-width: 1180px) {
    .pulse-grid,
    .outcome-summary {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .index-panel,
    :global(.related-economy),
    .outcome-controls {
      grid-template-columns: 1fr 1fr;
    }
  }

  @media (max-width: 900px) {
    .pulse-grid,
    .outcome-summary,
    .station-requirements,
    .boundary-grid,
    .index-panel,
    :global(.related-economy),
    .outcome-controls {
      grid-template-columns: 1fr;
    }

    .related-economy-actions {
      justify-content: flex-start;
    }
  }
</style>
