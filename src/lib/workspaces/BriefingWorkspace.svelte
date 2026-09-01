<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import type { TranslationKey } from "../i18n/catalog";
  import {
    formatNumber,
    formatPercent,
    formatSignedNumber,
  } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import {
    destinationsForSubject,
    type RelatedDataDestination,
    type WorkspaceFilters,
    type WorkspaceLocation,
  } from "../navigation/relatedData";
  import type {
    BriefFinding,
    BriefMetric,
    RepublicBrief,
  } from "../observations/types";
  import {
    briefMetricLabel,
    createBriefChangeChart,
    createBriefEducationChart,
    createBriefReceiverChart,
  } from "../presentation/republicBrief";
  import {
    metricContextDetails,
    metricContextHelp,
    metricContextSummary,
  } from "../presentation/metricContext";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import MetricContextHelp from "../ui/MetricContextHelp.svelte";
  import TechnicalDetails from "../ui/TechnicalDetails.svelte";

  type LinkedWorkspace =
    "monitor" | "materials" | "population" | "archive" | "plan";

  let {
    brief = null,
    location,
    onopenworkspace = () => {},
    onlocationchange,
    onrelatednavigate,
  }: {
    brief?: RepublicBrief | null;
    location: WorkspaceLocation;
    onopenworkspace?: (workspace: LinkedWorkspace) => void;
    onlocationchange?: (filters: WorkspaceFilters) => void;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
  } = $props();

  const sections: Array<{
    label: TranslationKey;
    href: string;
    marker: string;
  }> = [
    { label: "briefing-section-state", href: "#briefing", marker: "01" },
    { label: "briefing-section-assays", href: "#assays", marker: "02" },
    {
      label: "briefing-section-capabilities",
      href: "#capabilities",
      marker: "03",
    },
    { label: "briefing-section-dispatch", href: "#dispatch", marker: "04" },
  ];

  const findingKeys = {
    no_observation: [
      "briefing-finding-no-observation-title",
      "briefing-finding-no-observation-detail",
    ],
    historical_preview: [
      "briefing-finding-historical-title",
      "briefing-finding-historical-detail",
    ],
    partial_coverage: [
      "briefing-finding-partial-title",
      "briefing-finding-partial-detail",
    ],
    player_mapping: [
      "briefing-finding-player-mapping-title",
      "briefing-finding-player-mapping-detail",
    ],
    mapping_changed: [
      "briefing-finding-mapping-change-title",
      "briefing-finding-mapping-change-detail",
    ],
    no_prior_observation: [
      "briefing-finding-no-prior-title",
      "briefing-finding-no-prior-detail",
    ],
    missing_metrics: [
      "briefing-finding-missing-title",
      "briefing-finding-missing-detail",
    ],
    recorder_attention: [
      "briefing-finding-recorder-attention-title",
      "briefing-finding-recorder-attention-detail",
    ],
    recorder_queue: [
      "briefing-finding-recorder-queue-title",
      "briefing-finding-recorder-queue-detail",
    ],
    warehouse_attention: [
      "briefing-finding-warehouse-attention-title",
      "briefing-finding-warehouse-attention-detail",
    ],
    warehouse_lagging: [
      "briefing-finding-warehouse-lag-title",
      "briefing-finding-warehouse-lag-detail",
    ],
    catalogue_unavailable: [
      "briefing-finding-catalogue-title",
      "briefing-finding-catalogue-detail",
    ],
  } as const satisfies Record<
    string,
    readonly [TranslationKey, TranslationKey]
  >;

  const dispatchKeys = {
    observation_ready: "briefing-dispatch-observation-ready",
    no_observation: "briefing-dispatch-no-observation",
    historical_preview: "briefing-dispatch-historical",
    partial_coverage: "briefing-dispatch-partial",
    player_mapping: "briefing-dispatch-player-mapping",
    mapping_changed: "briefing-dispatch-mapping-change",
    no_prior_observation: "briefing-dispatch-no-prior",
    missing_metrics: "briefing-dispatch-missing",
    recorder_attention: "briefing-dispatch-recorder-attention",
    recorder_queue: "briefing-dispatch-recorder-queue",
    warehouse_attention: "briefing-dispatch-warehouse-attention",
    warehouse_lagging: "briefing-dispatch-warehouse-lag",
    catalogue_unavailable: "briefing-dispatch-catalogue",
  } as const satisfies Record<string, TranslationKey>;

  const capabilityKeys = {
    plan_attainment: [
      "briefing-capability-plan-title",
      "briefing-capability-plan-detail",
    ],
    import_exposure: [
      "briefing-capability-import-title",
      "briefing-capability-import-detail",
    ],
    observed_material_reliance: [
      "briefing-capability-material-title",
      "briefing-capability-material-detail",
    ],
  } as const satisfies Record<
    string,
    readonly [TranslationKey, TranslationKey]
  >;

  const recorderPhaseKeys = {
    disabled: "briefing-recorder-disabled",
    not_configured: "briefing-recorder-not-configured",
    watching: "briefing-recorder-watching",
    waiting_for_stability: "briefing-recorder-settling",
    retrying: "briefing-recorder-retrying",
    observed: "briefing-recorder-observed",
    failed: "briefing-recorder-attention",
  } as const satisfies Record<string, TranslationKey>;

  const warehousePhaseKeys = {
    ready: "briefing-warehouse-ready",
    lagging: "briefing-warehouse-lagging",
    rebuilding: "briefing-warehouse-rebuilding",
    attention: "briefing-warehouse-attention",
  } as const satisfies Record<string, TranslationKey>;

  const severityKeys = {
    information: "briefing-severity-information",
    watch: "briefing-severity-watch",
    attention: "briefing-severity-attention",
  } as const satisfies Record<string, TranslationKey>;

  let selectedMetricId = $state("source.stats.citizens.adults");
  const headlineMetrics = $derived(
    brief?.metrics.filter((metric) => metric.role === "headline") ?? [],
  );
  const selectedMetric = $derived(
    brief?.metrics.find((metric) => metric.metric_id === selectedMetricId) ??
      headlineMetrics[0] ??
      brief?.metrics[0] ??
      null,
  );
  const changeChart = $derived(
    brief ? createBriefChangeChart(brief, $translation) : null,
  );
  const educationChart = $derived(
    brief ? createBriefEducationChart(brief, $translation) : null,
  );
  const receiverChart = $derived(
    brief ? createBriefReceiverChart(brief, $translation) : null,
  );
  const educationMetric = $derived(
    brief?.metrics.find((metric) => metric.role === "education") ?? null,
  );
  const receiverMetric = $derived(
    brief?.metrics.find((metric) => metric.role === "receiver_class") ?? null,
  );
  const metricLabel = (metricId: string): string =>
    briefMetricLabel(metricId, $translation);
  const educationHelp = $derived(
    educationMetric
      ? metricContextHelp(educationMetric, $translation, metricLabel)
      : null,
  );
  const receiverHelp = $derived(
    receiverMetric
      ? metricContextHelp(receiverMetric, $translation, metricLabel)
      : null,
  );
  const selectedMetricDetails = $derived(
    selectedMetric
      ? metricContextDetails(selectedMetric.context, $translation, metricLabel)
      : [],
  );

  $effect(() => {
    const metricId = location.filters.metricId;
    if (
      metricId &&
      brief?.metrics.some((metric) => metric.metric_id === metricId)
    ) {
      selectedMetricId = metricId;
    }
  });

  function selectMetric(metricId: string): void {
    selectedMetricId = metricId;
    onlocationchange?.({ metricId });
  }

  function openMetric(metricId: string, origin: HTMLElement): void {
    selectMetric(metricId);
    onrelatednavigate?.(
      destinationsForSubject({ kind: "metric", metricId }),
      origin,
    );
  }

  function findingCopy(finding: BriefFinding): [string, string] {
    const keys = findingKeys[finding.code as keyof typeof findingKeys];
    if (!keys)
      return [
        $translation("briefing-finding-unknown-title"),
        $translation("briefing-finding-unknown-summary"),
      ];
    return [
      $translation(keys[0]),
      $translation(keys[1], { count: finding.value ?? 0 }),
    ];
  }

  function isKnownFinding(finding: BriefFinding): boolean {
    return finding.code in findingKeys;
  }

  function dispatchCopy(code: string): string {
    const key = dispatchKeys[code as keyof typeof dispatchKeys];
    return $translation(key ?? "briefing-dispatch-unknown");
  }

  function capabilityCopy(capability: string): [string, string] {
    const keys = capabilityKeys[capability as keyof typeof capabilityKeys];
    if (!keys)
      return [
        $translation("briefing-capability-unknown-title"),
        $translation("briefing-capability-unknown-detail"),
      ];
    return [$translation(keys[0]), $translation(keys[1])];
  }

  function metricValue(metric: BriefMetric): string {
    return formatNumber(metric.value, $activeLocale);
  }

  function metricChange(metric: BriefMetric): string {
    return metric.delta == null
      ? $translation("briefing-prior-unavailable")
      : $translation("briefing-change-from-prior", {
          value: formatSignedNumber(metric.delta, $activeLocale),
        });
  }
</script>

<section class="workspace">
  <aside
    class="navigator"
    aria-label={$translation("briefing-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("briefing-directorate")}</span>
        <h2>{$translation("briefing-republic-brief")}</h2>
      </div>
      <span class="edition">v1</span>
    </div>

    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span>
        <strong>{brief?.analysis_context.selected_branch_id ?? "—"}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("briefing-analytical-head")}</span>
        <strong
          >{brief?.analysis_context.mode === "historical_preview"
            ? $translation("briefing-mode-historical")
            : $translation("briefing-mode-latest")}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("filter-scope")}</span>
        <strong>{$translation("filter-whole-republic")}</strong>
      </div>
    </div>

    <div class="section-list">
      {#each sections as section}
        <a href={section.href} use:containedSectionNavigation
          ><span>{section.marker}</span>{$translation(section.label)}</a
        >
      {/each}
    </div>

    <GuidanceSurface kind="help" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("briefing-evidence-sidebar-note")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas" id="briefing">
    <GuidanceSurface
      kind={brief?.observation ? "boundary" : "help"}
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong
        >{$translation(
          brief?.observation
            ? "briefing-save-evidence"
            : "briefing-no-observation",
        )}</strong
      >
      <span
        >{$translation(
          brief?.observation
            ? "briefing-save-evidence-detail"
            : "briefing-no-observation-detail",
        )}</span
      >
    </GuidanceSurface>

    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("briefing-heading-eyebrow")}</span>
        <h2>{$translation("briefing-heading-title")}</h2>
        <p>{$translation("briefing-heading-description")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("briefing-exact-head")}</span>
        <strong
          >{brief?.observation?.year ?? "—"} · {brief?.observation
            ? String(brief.observation.day).padStart(3, "0")
            : "—"}</strong
        >
        <small
          >{brief?.observation?.source_file_name ??
            $translation("chart-unavailable")}</small
        >
      </div>
    </header>

    {#if headlineMetrics.length}
      <section
        class="kpi-grid"
        aria-label={$translation("briefing-kpis-label")}
      >
        {#each headlineMetrics as metric}
          <div class="metric-card-shell">
            <article
              class="kpi-card metric-card"
              class:selected={selectedMetric?.metric_id === metric.metric_id}
            >
              <header>
                <button
                  type="button"
                  class="metric-select"
                  aria-pressed={selectedMetric?.metric_id === metric.metric_id}
                  onclick={() => selectMetric(metric.metric_id)}
                  >{briefMetricLabel(metric.metric_id, $translation)}</button
                >
                <span class="coverage"
                  >{$translation(
                    brief?.observation?.coverage_status === "complete"
                      ? "coverage-complete"
                      : "coverage-partial",
                  )}</span
                >
              </header>
              <strong>{metricValue(metric)}</strong>
              <p>{metricChange(metric)}</p>
              <button
                id={`briefing-related-${metric.metric_id}`}
                type="button"
                class="related-data-link"
                onclick={(event) =>
                  openMetric(metric.metric_id, event.currentTarget)}
                >{$translation("related-nav-open")}</button
              >
              <footer>
                <span>{metricContextSummary(metric.context, $translation)}</span
                >
                <span class="badge" data-kind={metric.evidence_kind}
                  >{$translation(
                    metric.evidence_kind === "save_fact"
                      ? "evidence-save-fact"
                      : "evidence-calculation",
                  )}</span
                >
              </footer>
            </article>
            <span class="metric-card-help">
              <MetricContextHelp
                metricId={metric.metric_id}
                context={metric.context}
                {metricLabel}
                placement="left"
              />
            </span>
          </div>
        {/each}
      </section>
    {:else}
      <GuidanceSurface kind="help" layout="block">
        <strong>{$translation("briefing-empty-title")}</strong>
        <span>{$translation("briefing-empty-detail")}</span>
      </GuidanceSurface>
    {/if}

    {#if brief && changeChart && educationChart && receiverChart}
      <section
        id="assays"
        class="brief-chart-grid"
        aria-label={$translation("briefing-assays-label")}
      >
        <ObservatoryChart
          spec={changeChart}
          eyebrow={$translation("briefing-comparison-assay")}
        />
        <ObservatoryChart
          spec={educationChart}
          eyebrow={$translation("briefing-education-assay")}
          help={educationHelp}
        />
        <ObservatoryChart
          spec={receiverChart}
          eyebrow={$translation("briefing-receiver-assay")}
          help={receiverHelp}
        />
      </section>
    {/if}

    <section id="capabilities" class="capability-panel brief-capabilities">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("briefing-capability-eyebrow")}</span
          >
          <h2>{$translation("briefing-capability-heading")}</h2>
          <p>{$translation("briefing-capability-description")}</p>
        </div>
      </header>
      <div class="capability-grid">
        {#if brief?.plan}
          <article class="available-capability">
            <span class="coverage">{$translation("briefing-plan-active")}</span>
            <h3>{brief.plan.name}</h3>
            <strong class="plan-attainment">
              {brief.plan.attainment_basis_points === null
                ? $translation("chart-unavailable")
                : formatPercent(
                    brief.plan.attainment_basis_points / 100,
                    $activeLocale,
                  )}
            </strong>
            <p>
              {$translation("briefing-plan-summary", {
                revision: brief.plan.revision,
                targets: brief.plan.target_count,
                year: brief.plan.end_year,
                day: String(brief.plan.end_day).padStart(3, "0"),
              })}
            </p>
            <button type="button" onclick={() => onopenworkspace("plan")}
              >{$translation("briefing-open-plan")}</button
            >
          </article>
        {/if}
        {#each brief?.unavailable_capabilities ?? ["plan_attainment", "import_exposure", "observed_material_reliance"] as capability}
          {@const copy = capabilityCopy(capability)}
          <article>
            <span class="coverage">{$translation("chart-unavailable")}</span>
            <h3>{copy[0]}</h3>
            <p>{copy[1]}</p>
            {#if capability === "observed_material_reliance"}
              <button type="button" onclick={() => onopenworkspace("materials")}
                >{$translation("briefing-open-materials")}</button
              >
            {:else if capability === "plan_attainment"}
              <button type="button" onclick={() => onopenworkspace("plan")}
                >{$translation("briefing-open-plan")}</button
              >
            {/if}
          </article>
        {/each}
      </div>
    </section>

    <section class="dispatch-panel" id="dispatch">
      <div class="dispatch-seal" aria-hidden="true">04</div>
      <div>
        <span class="eyebrow"
          >{$translation("briefing-ministry-dispatch-eyebrow")}</span
        >
        <h2>{$translation("briefing-ministry-dispatch-title")}</h2>
        <p>{dispatchCopy(brief?.dispatch_code ?? "no_observation")}</p>
        <div class="dispatch-links">
          <button type="button" onclick={() => onopenworkspace("population")}
            >{$translation("briefing-open-population")}</button
          >
          <button type="button" onclick={() => onopenworkspace("monitor")}
            >{$translation("briefing-open-monitor")}</button
          >
          <button type="button" onclick={() => onopenworkspace("archive")}
            >{$translation("briefing-open-archive")}</button
          >
        </div>
      </div>
    </section>
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("briefing-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("briefing-evidence-inspector")}</span
        >
        <h2>
          {selectedMetric
            ? briefMetricLabel(selectedMetric.metric_id, $translation)
            : $translation("briefing-snapshot-ledger")}
        </h2>
      </div>
      <span
        class="status-chip"
        data-status={brief?.observation?.coverage_status === "complete"
          ? "stable"
          : "watch"}
        >{$translation(
          brief?.observation?.coverage_status === "complete"
            ? "coverage-complete"
            : "coverage-partial",
        )}</span
      >
    </div>

    {#if selectedMetric}
      <div class="selected-reading">
        <span>{$translation("briefing-recorded-value")}</span>
        <strong>{metricValue(selectedMetric)}</strong>
        <small>{metricChange(selectedMetric)}</small>
        <p>
          {$translation("briefing-metric-evidence-detail", {
            count: selectedMetric.sources.length,
          })}
        </p>
      </div>
      <section class="evidence-ledger">
        <span class="eyebrow">{$translation("briefing-source-ledger")}</span>
        {#each selectedMetric.sources as source}
          <div>
            <strong>{source.source_field}</strong>
            <span
              >{$translation("evidence-source-line", {
                line: source.source_line,
              })}</span
            >
          </div>
        {/each}
      </section>
      <section class="metric-context-ledger">
        <span class="eyebrow">{$translation("metric-context-ledger")}</span>
        <dl>
          {#each selectedMetricDetails as detail}
            <div>
              <dt>{detail.label}</dt>
              <dd>{detail.value}</dd>
            </div>
          {/each}
        </dl>
      </section>
    {/if}

    <div class="fact-grid brief-operations">
      <article>
        <span>{$translation("briefing-recorder-state")}</span>
        <strong
          >{$translation(
            brief?.operations.recorder_phase
              ? (recorderPhaseKeys[brief.operations.recorder_phase] ??
                  "briefing-state-unavailable")
              : "briefing-state-unavailable",
          )}</strong
        >
      </article>
      <article>
        <span>{$translation("briefing-warehouse-state")}</span>
        <strong
          >{$translation(
            brief?.operations.warehouse_phase
              ? (warehousePhaseKeys[brief.operations.warehouse_phase] ??
                  "briefing-state-unavailable")
              : "briefing-state-unavailable",
          )}</strong
        >
      </article>
      <article>
        <span>{$translation("briefing-catalogue-entities")}</span>
        <strong
          >{brief?.operations.catalogue_entity_count == null
            ? "—"
            : formatNumber(
                brief.operations.catalogue_entity_count,
                $activeLocale,
              )}</strong
        >
      </article>
      <article>
        <span>{$translation("briefing-city-scopes")}</span>
        <strong
          >{formatNumber(
            brief?.operations.city_scope_count ?? 0,
            $activeLocale,
          )}</strong
        >
      </article>
    </div>

    <section class="attention-queue">
      <header>
        <span class="eyebrow">{$translation("briefing-attention-queue")}</span>
        <strong
          >{$translation("briefing-findings-count", {
            count: brief?.findings.length ?? 0,
          })}</strong
        >
      </header>
      {#each brief?.findings ?? [] as finding}
        {@const copy = findingCopy(finding)}
        <article data-severity={finding.severity}>
          <span>{$translation(severityKeys[finding.severity])}</span>
          <strong>{copy[0]}</strong>
          <p>{copy[1]}</p>
          {#if !isKnownFinding(finding)}
            <TechnicalDetails
              code={finding.code}
              operation="republic_brief_finding"
            />
          {/if}
        </article>
      {:else}
        <GuidanceSurface kind="boundary" layout="compact">
          <strong>{$translation("briefing-no-findings-title")}</strong>
          <span>{$translation("briefing-no-findings-detail")}</span>
        </GuidanceSurface>
      {/each}
    </section>
  </aside>
</section>

<style>
  .metric-card {
    width: 100%;
    height: 100%;
    color: inherit;
    text-align: start;
    cursor: default;
  }

  .metric-select {
    min-height: 32px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--colour-text);
    text-align: start;
  }

  .metric-card-shell {
    position: relative;
    min-width: 0;
  }

  .metric-card-shell .metric-card > header {
    padding-inline-end: 36px;
  }

  .metric-card-help {
    position: absolute;
    z-index: 12;
    inset-block-start: 9px;
    inset-inline-end: 9px;
  }

  @media (min-width: 1501px) {
    .metric-card-shell:nth-child(3n + 1)
      .metric-card-help
      :global(.context-tooltip) {
      inset-inline-start: 0;
      inset-inline-end: auto;
    }
  }

  @media (min-width: 761px) and (max-width: 1500px) {
    .kpi-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .metric-card-shell:nth-child(2n + 1)
      .metric-card-help
      :global(.context-tooltip) {
      inset-inline-start: 0;
      inset-inline-end: auto;
    }
  }

  @media (max-width: 760px) {
    .kpi-grid {
      grid-template-columns: 1fr;
    }
  }

  .metric-context-ledger {
    display: grid;
    gap: 10px;
    margin-top: 20px;
  }

  .metric-context-ledger dl {
    display: grid;
    gap: 0;
    margin: 0;
  }

  .metric-context-ledger dl > div {
    display: grid;
    gap: 3px;
    border-bottom: 1px solid var(--colour-line-faint);
    padding: 9px 0;
  }

  .metric-context-ledger dt,
  .metric-context-ledger dd {
    margin: 0;
  }

  .metric-context-ledger dt {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }

  .metric-context-ledger dd {
    color: var(--colour-text);
    font-size: var(--type-caption);
    line-height: 1.5;
  }

  .metric-card.selected {
    border-color: var(--colour-observed);
    box-shadow: inset 0 0 0 1px var(--colour-observed-soft);
  }

  .brief-chart-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-top: 10px;
    scroll-margin-top: 18px;
  }

  .brief-chart-grid :global(.chart-card:last-child) {
    grid-column: 1 / -1;
  }

  .brief-capabilities {
    scroll-margin-top: 18px;
  }

  .capability-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin-top: 14px;
  }

  .capability-grid article {
    min-width: 0;
    border: 1px solid var(--colour-line-faint);
    padding: 13px;
    background: var(--colour-surface-raised);
  }

  .capability-grid h3 {
    margin-top: 8px;
    font-size: var(--type-body);
  }

  .capability-grid p {
    margin-top: 7px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    line-height: 1.55;
  }

  .available-capability {
    border-color: var(--colour-success);
  }

  .plan-attainment {
    display: block;
    margin: 7px 0 4px;
    color: var(--colour-success);
    font-family: var(--font-display);
    font-size: 1.55rem;
  }

  .capability-grid button,
  .dispatch-links button {
    min-height: 32px;
    margin-top: 10px;
    border: 1px solid var(--colour-line);
    padding: 6px 10px;
    color: var(--colour-observed);
    background: var(--colour-surface-raised);
    cursor: pointer;
  }

  .dispatch-links button {
    margin-top: 0;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-size: var(--type-caption);
  }

  .attention-queue article[data-severity="attention"] > span {
    color: var(--colour-risk);
  }

  .attention-queue article[data-severity="information"] > span {
    color: var(--colour-observed);
  }

  @media (max-width: 1100px) {
    .brief-chart-grid,
    .capability-grid {
      grid-template-columns: 1fr;
    }

    .brief-chart-grid :global(.chart-card:last-child) {
      grid-column: auto;
    }
  }
</style>
