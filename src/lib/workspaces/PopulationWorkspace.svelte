<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import AttentionCue from "../attention/AttentionCue.svelte";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import MetricContextHelp from "../ui/MetricContextHelp.svelte";
  import { formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import { exactObservationChartBindings } from "../navigation/chartBindings";
  import {
    destinationsForSubject,
    type RelatedDataDestination,
    type WorkspaceFilters,
    type WorkspaceLocation,
  } from "../navigation/relatedData";
  import type {
    PopulationDataset,
    PublishedMetricContext,
  } from "../observations/types";
  import {
    metricContextHelpFor,
    publishedMetricContext,
  } from "../presentation/metricContext";
  import {
    createCityMovementChart,
    createEducationProfileChart,
    createPopulationMovementChart,
    createPopulationStatusChart,
    populationFact,
    populationFactLabel,
  } from "../presentation/population";
  import WorkspaceSectionHeader from "./WorkspaceSectionHeader.svelte";

  let {
    dataset = null,
    metricContexts = [],
    desktopAvailable,
    location,
    onlocationchange,
    onrelatednavigate,
    onopenresearch,
  }: {
    dataset?: PopulationDataset | null;
    metricContexts?: PublishedMetricContext[];
    desktopAvailable: boolean;
    location: WorkspaceLocation;
    onlocationchange?: (filters: WorkspaceFilters) => void;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
    onopenresearch: () => void;
  } = $props();

  const metricLabel = (metricId: string): string =>
    populationFactLabel(metricId, $translation);
  const adultsContext = $derived(
    publishedMetricContext(
      metricContexts,
      "source.stats.citizens.adults",
      "exact",
    ),
  );
  const childrenContext = $derived(
    publishedMetricContext(
      metricContexts,
      "source.stats.citizens.small_children",
      "exact",
    ),
  );
  const unemployedContext = $derived(
    publishedMetricContext(
      metricContexts,
      "source.stats.citizens.unemployed",
      "exact",
    ),
  );
  const educationHelp = $derived(
    metricHelp("source.stats.citizens.no_education", "history"),
  );
  const movementHelp = $derived(
    metricHelp("source.stats.citizens.born", "history"),
  );

  let selectedCityId = $state("");
  const latest = $derived(dataset?.observations.at(-1) ?? null);
  const selectedCity = $derived(
    dataset?.cities.find((city) => city.scope_id === selectedCityId) ??
      dataset?.cities[0] ??
      null,
  );
  const statusChart = $derived(
    createPopulationStatusChart(dataset ?? emptyDataset(), $translation),
  );
  const movementChart = $derived(
    createPopulationMovementChart(dataset ?? emptyDataset(), $translation),
  );
  const educationChart = $derived(
    createEducationProfileChart(dataset ?? emptyDataset(), $translation),
  );
  const cityChart = $derived(
    createCityMovementChart(
      selectedCity,
      dataset ?? emptyDataset(),
      $translation,
    ),
  );
  const statusNavigation = $derived(
    dataset
      ? exactObservationChartBindings(statusChart, dataset.observations, {
          workspace: "population",
          section: "population-status",
          filters: {},
        })
      : [],
  );
  const movementNavigation = $derived(
    dataset
      ? exactObservationChartBindings(movementChart, dataset.observations, {
          workspace: "population",
          section: "population-movement",
          filters: {},
        })
      : [],
  );

  $effect(() => {
    const requestedCity = location.filters.cityId;
    if (
      requestedCity &&
      dataset?.cities.some((city) => city.scope_id === requestedCity)
    ) {
      selectedCityId = requestedCity;
      return;
    }
    const firstCity = dataset?.cities[0]?.scope_id ?? "";
    if (!dataset?.cities.some((city) => city.scope_id === selectedCityId)) {
      selectedCityId = firstCity;
    }
  });

  function selectCity(cityId: string): void {
    selectedCityId = cityId;
    onlocationchange?.({ cityId });
  }

  function openMetric(metricId: string, origin: HTMLElement): void {
    onlocationchange?.({ metricId });
    onrelatednavigate?.(
      destinationsForSubject({ kind: "metric", metricId }),
      origin,
    );
  }

  function openCity(origin: HTMLElement): void {
    if (!selectedCity) return;
    onrelatednavigate?.(
      destinationsForSubject({ kind: "city", cityId: selectedCity.scope_id }),
      origin,
    );
  }

  function emptyDataset(): PopulationDataset {
    return {
      analysis_context: {
        context_id: "unavailable",
        selected_branch_id: "unassigned",
        head_interpretation_id: null,
        original_branch_id: null,
        mode: "latest",
        origin: "automatic",
        is_tip: true,
        membership_revision: 0,
        compatibility_profile_id: null,
        compatibility_profile_hash: null,
        observation_watermark: null,
        catalogue_generation_id: null,
        resource_catalogue_revision_id: null,
        overlay_revision: null,
      },
      observations: [],
      cities: [],
      observation_limit: 256,
      city_limit: 512,
      tesmio_probe: {
        state: "not_configured",
        read_only: true,
        optional: true,
        persisted: false,
        probe_id: null,
        probe_version: null,
        loader_api_version: null,
        target_game_version: null,
        executable_timestamp: null,
        content_hash: null,
        snapshot_count: 0,
        sample_count: 0,
        latest_year: null,
        latest_day: null,
        latest_population_count: null,
        collection_stage: null,
        people_readings_ready: false,
        resource_readings_ready: false,
        environment_readings_ready: false,
        facility_contract_version: null,
        last_report_at_ms: null,
        warnings: [],
      },
    };
  }

  function factValue(factId: string): string {
    const fact = latest ? populationFact(latest.facts, factId) : null;
    return fact
      ? formatNumber(fact.value, $activeLocale)
      : $translation("chart-unavailable");
  }

  function gameDate(): string {
    if (!latest) return $translation("chart-unavailable");
    return $translation("observation-game-date-compact", {
      year: latest.sampled_year,
      day: String(latest.sampled_day).padStart(3, "0"),
    });
  }

  function probeStateLabel(): string {
    switch (dataset?.tesmio_probe.state) {
      case "available":
        return $translation("population-probe-state-available");
      case "warning":
        return $translation("population-probe-state-warning");
      case "invalid":
        return $translation("population-probe-state-invalid");
      case "missing":
        return $translation("population-probe-state-missing");
      default:
        return $translation("population-probe-state-not-configured");
    }
  }

  function metricHelp(metricId: string, mode: "exact" | "history") {
    const context = publishedMetricContext(metricContexts, metricId, mode);
    return context
      ? metricContextHelpFor(metricId, context, $translation, metricLabel)
      : null;
  }
</script>

<section class="workspace population-workspace">
  <aside
    class="navigator"
    aria-label={$translation("population-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("population-directorate")}</span>
        <h2>{$translation("nav-population")}</h2>
      </div>
      <span class="edition">{$translation("population-edition")}</span>
    </div>

    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span>
        <strong>{dataset?.analysis_context.selected_branch_id ?? "—"}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("population-context-mode")}</span>
        <strong
          >{$translation(
            dataset?.analysis_context.mode === "historical_preview"
              ? "population-context-historical"
              : "population-context-latest",
          )}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("population-observations")}</span>
        <strong
          >{formatNumber(
            dataset?.observations.length ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
    </div>

    <div class="section-list">
      <a href="#population-status" use:containedSectionNavigation
        ><span>01</span>{$translation("population-section-status")}</a
      >
      <a href="#population-movement" use:containedSectionNavigation
        ><span>02</span>{$translation("population-section-movement")}</a
      >
      <a href="#population-cities" use:containedSectionNavigation
        ><span>03</span>{$translation("population-section-cities")}</a
      >
      <a href="#population-identity" use:containedSectionNavigation
        ><span>04</span>{$translation("population-section-lives")}</a
      >
    </div>

    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("population-sidebar-boundary")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong>{$translation("population-evidence-banner")}</strong>
      <span>{$translation("population-evidence-banner-detail")}</span>
    </GuidanceSurface>

    <WorkspaceSectionHeader
      level="page"
      eyebrow={$translation("population-heading-eyebrow")}
      title={$translation("population-heading-title")}
      description={$translation("population-heading-description")}
    >
      {#snippet actions()}
        <div class="date-stamp">
          <span>{$translation("population-selected-head")}</span>
          <strong>{latest ? "✓" : "—"}</strong>
          <small>{gameDate()}</small>
        </div>
      {/snippet}
    </WorkspaceSectionHeader>

    {#if !desktopAvailable}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("archive-desktop-required")}</span>
        <h3>{$translation("population-desktop-required")}</h3>
        <p>{$translation("population-desktop-required-detail")}</p>
      </section>
    {:else if !latest}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("population-no-evidence")}</span>
        <h3>{$translation("population-no-observations")}</h3>
        <p>{$translation("population-no-observations-detail")}</p>
        <AttentionCue
          cueId="research.setup.entry"
          contentRevision={1}
          heading={$translation("research-setup-entry-cue-title")}
          detail={$translation("research-setup-entry-cue-detail")}
          dismissLabel={$translation("attention-dismiss")}
        >
          <button
            type="button"
            class="population-probe-setup"
            onclick={onopenresearch}
          >
            {$translation("research-setup-open")}
          </button>
        </AttentionCue>
      </section>
    {:else}
      <section
        class="kpi-grid population-kpis"
        aria-label={$translation("population-head-summary")}
      >
        <article class="kpi-card">
          <header>
            <span>{$translation("population-fact-adults")}</span><span
              class="coverage">{$translation("evidence-save-fact")}</span
            >
            {#if adultsContext}
              <MetricContextHelp
                metricId="source.stats.citizens.adults"
                context={adultsContext}
                {metricLabel}
                placement="left"
              />
            {/if}
          </header>
          <strong>{factValue("source.stats.citizens.adults")}</strong>
          <p>{$translation("population-direct-source-count")}</p>
          <button
            type="button"
            class="related-data-link"
            onclick={(event) =>
              openMetric("source.stats.citizens.adults", event.currentTarget)}
            >{$translation("related-nav-open")}</button
          >
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("population-fact-small-children")}</span><span
              class="coverage">{$translation("evidence-save-fact")}</span
            >
            {#if childrenContext}
              <MetricContextHelp
                metricId="source.stats.citizens.small_children"
                context={childrenContext}
                {metricLabel}
                placement="left"
              />
            {/if}
          </header>
          <strong>{factValue("source.stats.citizens.small_children")}</strong>
          <p>{$translation("population-direct-source-count")}</p>
          <button
            type="button"
            class="related-data-link"
            onclick={(event) =>
              openMetric(
                "source.stats.citizens.small_children",
                event.currentTarget,
              )}>{$translation("related-nav-open")}</button
          >
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("population-fact-unemployed")}</span><span
              class="coverage">{$translation("evidence-save-fact")}</span
            >
            {#if unemployedContext}
              <MetricContextHelp
                metricId="source.stats.citizens.unemployed"
                context={unemployedContext}
                {metricLabel}
                placement="left"
              />
            {/if}
          </header>
          <strong>{factValue("source.stats.citizens.unemployed")}</strong>
          <p>{$translation("population-no-rate-denominator")}</p>
          <button
            type="button"
            class="related-data-link"
            onclick={(event) =>
              openMetric(
                "source.stats.citizens.unemployed",
                event.currentTarget,
              )}>{$translation("related-nav-open")}</button
          >
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("population-city-scopes")}</span><span
              class="coverage">{$translation("coverage-complete")}</span
            >
          </header>
          <strong
            >{formatNumber(dataset?.cities.length ?? 0, $activeLocale)}</strong
          >
          <p>{$translation("population-city-identifiers-neutral")}</p>
          <button
            type="button"
            class="related-data-link"
            disabled={!selectedCity}
            onclick={(event) => openCity(event.currentTarget)}
            >{$translation("related-nav-open")}</button
          >
        </article>
      </section>

      <section id="population-status" class="population-chart-grid">
        <ObservatoryChart
          spec={statusChart}
          height="285px"
          eyebrow={$translation("population-section-status")}
          navigation={statusNavigation}
          {onrelatednavigate}
        />
        <ObservatoryChart
          spec={educationChart}
          eyebrow={$translation("population-section-education")}
          help={educationHelp}
        />
      </section>

      <section id="population-movement" class="population-chart-wide">
        <div class="causation-warning" role="note">
          <strong>{$translation("population-window-unverified")}</strong>
          <span>{$translation("population-window-unverified-detail")}</span>
        </div>
        <ObservatoryChart
          spec={movementChart}
          height="285px"
          eyebrow={$translation("population-section-movement")}
          help={movementHelp}
          navigation={movementNavigation}
          {onrelatednavigate}
        />
      </section>

      <section id="population-cities" class="population-city-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("population-city-assay")}</span>
            <h2>{$translation("population-city-title")}</h2>
            <p>{$translation("population-city-description")}</p>
          </div>
          <label>
            <span>{$translation("population-select-city")}</span>
            <select
              bind:value={selectedCityId}
              onchange={() => selectCity(selectedCityId)}
            >
              {#each dataset?.cities ?? [] as city}
                <option value={city.scope_id}
                  >{$translation("population-city-neutral-label", {
                    id: city.scope_id,
                  })}</option
                >
              {/each}
            </select>
          </label>
        </header>
        <ObservatoryChart
          spec={cityChart}
          eyebrow={$translation("population-section-cities")}
        />
      </section>

      <section id="population-identity" class="population-identity-gate">
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("population-life-research")}</span
            >
            <h2>{$translation("population-life-title")}</h2>
            <p>{$translation("population-life-description")}</p>
          </div>
          <ContextHelp
            topic="population-life-history"
            title={$translation("population-life-help-title")}
            text={$translation("population-life-help-text")}
            placement="left"
          />
        </header>
        <div class="population-gate-grid">
          <article data-state="observed">
            <span>{$translation("population-gate-record-layout")}</span>
            <strong>{$translation("population-gate-research-observed")}</strong>
            <p>{$translation("population-gate-record-layout-detail")}</p>
          </article>
          <article data-state="blocked">
            <span>{$translation("population-gate-stable-identity")}</span>
            <strong>{$translation("population-gate-not-proven")}</strong>
            <p>{$translation("population-gate-stable-identity-detail")}</p>
          </article>
          <article data-state="blocked">
            <span>{$translation("population-gate-family-links")}</span>
            <strong>{$translation("population-gate-not-proven")}</strong>
            <p>{$translation("population-gate-family-links-detail")}</p>
          </article>
          <article data-state="blocked">
            <span>{$translation("population-gate-life-events")}</span>
            <strong>{$translation("population-gate-unavailable")}</strong>
            <p>{$translation("population-gate-life-events-detail")}</p>
          </article>
        </div>
        <section
          class="population-probe-panel"
          data-state={dataset?.tesmio_probe.state ?? "not_configured"}
          aria-labelledby="population-probe-title"
        >
          <header>
            <div>
              <span class="eyebrow"
                >{$translation("population-probe-eyebrow")}</span
              >
              <h3 id="population-probe-title">
                {$translation("population-probe-title")}
              </h3>
            </div>
            <span class="status-chip">{probeStateLabel()}</span>
          </header>
          <p>{$translation("population-probe-description")}</p>
          <AttentionCue
            cueId="research.setup.entry"
            contentRevision={1}
            heading={$translation("research-setup-entry-cue-title")}
            detail={$translation("research-setup-entry-cue-detail")}
            dismissLabel={$translation("attention-dismiss")}
          >
            <button
              type="button"
              class="population-probe-setup"
              disabled={!desktopAvailable}
              onclick={onopenresearch}
            >
              {$translation("research-setup-open")}
            </button>
          </AttentionCue>
          <div class="population-probe-contract">
            <article>
              <strong>{$translation("population-probe-read-only")}</strong>
              <span>{$translation("population-probe-read-only-detail")}</span>
            </article>
            <article>
              <strong>{$translation("population-probe-ephemeral")}</strong>
              <span>{$translation("population-probe-ephemeral-detail")}</span>
            </article>
            <article>
              <strong>{$translation("population-probe-no-identity")}</strong>
              <span>{$translation("population-probe-no-identity-detail")}</span>
            </article>
          </div>
          {#if dataset?.tesmio_probe.state === "available" || dataset?.tesmio_probe.state === "warning"}
            <dl class="population-probe-readings">
              <div>
                <dt>{$translation("population-probe-snapshots")}</dt>
                <dd>
                  {formatNumber(
                    dataset.tesmio_probe.snapshot_count,
                    $activeLocale,
                  )}
                </dd>
              </div>
              <div>
                <dt>{$translation("population-probe-samples")}</dt>
                <dd>
                  {formatNumber(
                    dataset.tesmio_probe.sample_count,
                    $activeLocale,
                  )}
                </dd>
              </div>
              <div>
                <dt>{$translation("population-probe-latest-population")}</dt>
                <dd>
                  {dataset.tesmio_probe.latest_population_count === null
                    ? $translation("chart-unavailable")
                    : formatNumber(
                        dataset.tesmio_probe.latest_population_count,
                        $activeLocale,
                      )}
                </dd>
              </div>
              <div>
                <dt>{$translation("population-probe-target-build")}</dt>
                <dd>{dataset.tesmio_probe.target_game_version ?? "—"}</dd>
              </div>
            </dl>
          {:else if dataset?.tesmio_probe.state === "invalid"}
            <p class="population-probe-warning" role="alert">
              {$translation("population-probe-invalid-detail")}
            </p>
          {:else}
            <GuidanceSurface
              kind="instruction"
              layout="compact"
              class="population-probe-note"
            >
              {$translation("population-probe-missing-detail")}
            </GuidanceSurface>
          {/if}
        </section>
      </section>
    {/if}
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("population-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("population-inspector-eyebrow")}</span
        >
        <h2>{$translation("population-inspector-title")}</h2>
      </div>
      <span class="status-chip" data-status={latest ? "stable" : "watch"}
        >{$translation(
          latest ? "evidence-save-fact" : "chart-unavailable",
        )}</span
      >
    </div>

    {#if latest}
      <div class="selected-reading">
        <span>{$translation("population-selected-observation")}</span>
        <strong>{gameDate()}</strong>
        <small>{latest.source_file_name}</small>
        <p>{$translation("population-head-exact-note")}</p>
      </div>
      <div class="fact-grid">
        <article>
          <span>{$translation("population-observations")}</span><strong
            >{dataset?.observations.length ?? 0}</strong
          >
        </article>
        <article>
          <span>{$translation("population-city-scopes")}</span><strong
            >{dataset?.cities.length ?? 0}</strong
          >
        </article>
        <article>
          <span>{$translation("population-profile")}</span><strong
            >{latest.profile_version}</strong
          >
        </article>
        <article>
          <span>{$translation("population-mapping")}</span><strong
            >{latest.mapping_classification}</strong
          >
        </article>
      </div>

      <section class="population-evidence-list">
        <span class="eyebrow">{$translation("population-source-ledger")}</span>
        {#each latest.facts as fact}
          <article>
            <strong>{populationFactLabel(fact.fact_id, $translation)}</strong>
            <span>{formatNumber(fact.value, $activeLocale)}</span>
            <code>{fact.source_field} · L{fact.source_line}</code>
          </article>
        {/each}
      </section>
    {:else}
      <div class="archive-inspector-empty">
        {$translation("population-no-source-ledger")}
      </div>
    {/if}

    <section class="provenance-key">
      <span class="eyebrow">{$translation("population-evidence-boundary")}</span
      >
      <div>
        <i data-kind="save_fact"></i><span
          >{$translation("population-boundary-save-fact")}</span
        >
      </div>
      <div>
        <i data-kind="calculation"></i><span
          >{$translation("population-boundary-no-interpolation")}</span
        >
      </div>
      <div>
        <i data-kind="estimate"></i><span
          >{$translation("population-boundary-no-causality")}</span
        >
      </div>
      <div>
        <i data-kind="recommendation"></i><span
          >{$translation("population-boundary-no-biographies")}</span
        >
      </div>
    </section>
  </aside>
</section>

<style>
  .population-kpis {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin-bottom: 10px;
  }

  .population-chart-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(340px, 0.65fr);
    gap: 10px;
  }

  .population-chart-wide,
  .population-city-panel,
  .population-identity-gate {
    margin-top: 10px;
    border: 1px solid var(--colour-line-faint);
    padding: 14px;
    background: rgba(13, 23, 33, 0.52);
  }

  .population-chart-wide :global(.chart-card),
  .population-city-panel :global(.chart-card) {
    border: 0;
  }

  .population-city-panel > header {
    align-items: end;
    margin-bottom: 12px;
  }

  .population-city-panel label {
    min-width: 220px;
    display: grid;
    gap: 6px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .population-city-panel select {
    border: 1px solid var(--colour-line);
    padding: 8px 10px;
    color: var(--colour-text);
    background: var(--colour-surface-raised);
    text-transform: none;
  }

  .population-gate-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    margin-top: 14px;
  }

  .population-gate-grid article {
    min-width: 0;
    border: 1px solid var(--colour-line-faint);
    border-inline-start: 3px solid var(--colour-risk);
    padding: 12px;
    background: var(--colour-surface);
  }

  .population-gate-grid article[data-state="observed"] {
    border-inline-start-color: var(--colour-observed);
  }

  .population-gate-grid span,
  .population-gate-grid strong,
  .population-gate-grid p {
    display: block;
  }

  .population-probe-panel {
    margin-top: 12px;
    border: 1px solid var(--colour-line-faint);
    border-inline-start: 3px solid var(--colour-observed);
    padding: 13px;
    background: var(--colour-surface-raised);
  }

  .population-probe-panel[data-state="invalid"] {
    border-inline-start-color: var(--colour-risk);
  }

  .population-probe-panel[data-state="warning"],
  .population-probe-panel[data-state="missing"] {
    border-inline-start-color: var(--colour-gold);
  }

  .population-probe-panel > header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .population-probe-panel h3 {
    margin-top: 4px;
    font-size: 1.0625rem;
  }

  .population-probe-panel > p {
    margin-top: 8px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.55;
  }

  .population-probe-setup {
    margin-top: 10px;
    border: 1px solid var(--colour-observed);
    padding: 8px 11px;
    color: var(--colour-text);
    background: var(--colour-observed-soft);
    cursor: pointer;
  }

  .population-probe-setup:disabled {
    cursor: not-allowed;
  }

  .population-probe-contract,
  .population-probe-readings {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin-top: 12px;
  }

  .population-probe-contract article,
  .population-probe-readings > div {
    min-width: 0;
    border: 1px solid var(--colour-line-faint);
    padding: 9px;
    background: var(--colour-surface);
  }

  .population-probe-contract strong,
  .population-probe-contract span,
  .population-probe-readings dt,
  .population-probe-readings dd {
    display: block;
    font-size: var(--type-caption);
  }

  .population-probe-contract span,
  .population-probe-readings dt {
    margin-top: 4px;
    color: var(--colour-muted);
    line-height: 1.45;
  }

  .population-probe-readings {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .population-probe-readings {
    margin-bottom: 0;
  }

  .population-probe-readings dd {
    margin: 5px 0 0;
    color: var(--colour-observed);
    font-weight: 700;
  }

  .population-probe-warning {
    color: var(--colour-risk) !important;
  }

  .population-gate-grid span {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .population-gate-grid strong {
    margin-top: 7px;
    color: var(--colour-text);
    font-size: var(--type-body);
  }

  .population-gate-grid p {
    margin-top: 7px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.5;
  }

  .population-evidence-list {
    display: grid;
    gap: 7px;
    margin-top: 18px;
  }

  .population-evidence-list article {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 4px 8px;
    border-top: 1px solid var(--colour-line-faint);
    padding-top: 8px;
  }

  .population-evidence-list strong,
  .population-evidence-list span {
    font-size: var(--type-caption);
  }

  .population-evidence-list code {
    grid-column: 1 / -1;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    overflow-wrap: anywhere;
  }

  @media (max-width: 1180px) {
    .population-kpis,
    .population-gate-grid,
    .population-probe-contract,
    .population-probe-readings {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .population-chart-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 760px) {
    .population-kpis,
    .population-gate-grid,
    .population-probe-contract,
    .population-probe-readings {
      grid-template-columns: 1fr;
    }

    .population-city-panel > header {
      align-items: stretch;
      flex-direction: column;
    }

    .population-city-panel label {
      min-width: 0;
    }
  }
</style>
