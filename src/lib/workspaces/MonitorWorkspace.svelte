<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import {
    createCadenceChart,
    createReceiverChangeChart,
    largestObservationInterval,
    selectedBranchObservations,
  } from "../presentation/republicPulse";
  import { formatDate, formatNumber } from "../i18n/format";
  import type { TranslationKey } from "../i18n/catalog";
  import { activeLocale, translation } from "../i18n/runtime";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import TechnicalDetails from "../ui/TechnicalDetails.svelte";
  import type {
    ArchiveComparison,
    ArchiveOverview,
    ReceiverDataset,
    RecorderCandidateStatus,
    RecorderDiscoverySource,
    RecorderHealth,
    PublishedMetricContext,
  } from "../observations/types";

  let {
    health,
    archive,
    receiverDataset,
    desktopAvailable,
    oncompare,
    metricContexts = [],
  }: {
    health: RecorderHealth | null;
    archive: ArchiveOverview | null;
    receiverDataset: ReceiverDataset | null;
    desktopAvailable: boolean;
    metricContexts?: PublishedMetricContext[];
    oncompare: (
      fromPayloadHash: string,
      toPayloadHash: string,
    ) => Promise<ArchiveComparison>;
  } = $props();
  import {
    metricContextHelpFor,
    publishedMetricContext,
  } from "../presentation/metricContext";
  import { briefMetricLabel } from "../presentation/republicBrief";

  const sections: Array<{
    label: TranslationKey;
    href: string;
    marker: string;
  }> = [
    { label: "monitor-section-health", href: "#monitor-health", marker: "01" },
    { label: "monitor-section-pulse", href: "#monitor-pulse", marker: "02" },
    { label: "monitor-section-ledger", href: "#monitor-ledger", marker: "03" },
  ];
  const statusKeys: Record<RecorderCandidateStatus, TranslationKey> = {
    discovered: "recorder-status-discovered",
    stabilising: "recorder-status-stabilising",
    ready: "recorder-status-ready",
    reading: "recorder-status-reading",
    imported: "recorder-status-imported",
    duplicate: "recorder-status-duplicate",
    retryable_failure: "recorder-status-retryable-failure",
    terminal_failure: "recorder-status-terminal-failure",
    superseded: "recorder-status-superseded",
  };
  const discoveryKeys: Record<RecorderDiscoverySource, TranslationKey> = {
    migration: "recorder-source-migration",
    initial_scan: "recorder-source-initial-scan",
    filesystem_event: "recorder-source-filesystem-event",
    reconciliation: "recorder-source-reconciliation",
  };

  let comparison = $state<ArchiveComparison | null>(null);
  let comparisonKey = "";
  let comparisonFailed = $state(false);
  const branchObservations = $derived(selectedBranchObservations(archive));
  const latestObservation = $derived(branchObservations.at(-1) ?? null);
  const latestPoint = $derived(receiverDataset?.points.at(-1) ?? null);
  const latestEntry = $derived(health?.latest_entries[0] ?? null);
  const cadenceChart = $derived(
    createCadenceChart(archive, $translation, $activeLocale),
  );
  const receiverChangeChart = $derived(
    createReceiverChangeChart(comparison, $translation),
  );
  const largestInterval = $derived(largestObservationInterval(archive));
  const receiverChangeHelp = $derived.by(() => {
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
          (metricId) => briefMetricLabel(metricId, $translation),
        )
      : null;
  });

  $effect(() => {
    const from = branchObservations.at(-2)?.payload_hash;
    const to = branchObservations.at(-1)?.payload_hash;
    const nextKey = from && to ? `${from}:${to}` : "";
    if (nextKey === comparisonKey) return;
    comparisonKey = nextKey;
    comparison = null;
    comparisonFailed = false;
    if (!from || !to) return;
    void oncompare(from, to)
      .then((result) => {
        if (comparisonKey === nextKey) comparison = result;
      })
      .catch(() => {
        if (comparisonKey === nextKey) comparisonFailed = true;
      });
  });

  function phaseLabel(): string {
    const phase = health?.observer.phase;
    if (!health || !phase) return $translation("chart-unavailable");
    if (phase === "disabled")
      return $translation("observer-automatic-disabled");
    if (phase === "not_configured")
      return $translation("observer-automatic-needs-folder");
    if (phase === "waiting_for_stability")
      return $translation("scanner-waiting");
    if (phase === "retrying") return $translation("scanner-retrying");
    if (phase === "failed") return $translation("scanner-attention");
    if (phase === "observed") return $translation("scanner-observed");
    return $translation("scanner-watching");
  }

  function timestamp(value: number | null | undefined): string {
    if (value === null || value === undefined)
      return $translation("chart-unavailable");
    return formatDate(value, $activeLocale, {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }

  function latency(value: number | null | undefined): string {
    if (value === null || value === undefined)
      return $translation("chart-unavailable");
    return $translation("monitor-latency-seconds", {
      seconds: formatNumber(Math.max(0, value) / 1_000, $activeLocale, {
        maximumFractionDigits: 1,
      }),
    });
  }
</script>

<section class="workspace monitor-workspace">
  <aside
    class="navigator"
    aria-label={$translation("monitor-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("monitor-directorate")}</span>
        <h2>{$translation("monitor-title")}</h2>
      </div>
      <span class="edition">LIVE*</span>
    </div>

    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("monitor-recorder")}</span>
        <strong>{phaseLabel()}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("monitor-selected-branch")}</span>
        <strong
          >{archive?.selected_branch_id ?? $translation("archive-none")}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("monitor-window")}</span>
        <strong>{$translation("monitor-window-all")}</strong>
      </div>
    </div>

    <div class="section-list">
      {#each sections as section}
        <a href={section.href}
          ><span>{section.marker}</span>{$translation(section.label)}</a
        >
      {/each}
    </div>

    <GuidanceSurface kind="help" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("monitor-sidebar-note")}</p>
    </GuidanceSurface>
  </aside>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-scrollable region) -->
  <section
    class="canvas"
    id="monitor-health"
    aria-label={$translation("monitor-heading-title")}
    tabindex="0"
  >
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class="preview-banner monitor-banner"
    >
      <strong>{$translation("monitor-near-live-label")}</strong>
      <span>{$translation("monitor-near-live-detail")}</span>
    </GuidanceSurface>

    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("monitor-heading-eyebrow")}</span>
        <h2>{$translation("monitor-heading-title")}</h2>
        <p>{$translation("monitor-heading-description")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("monitor-last-scan")}</span>
        <strong>{health?.last_scan_ms ? "✓" : "—"}</strong>
        <small>{timestamp(health?.last_scan_ms)}</small>
      </div>
    </header>

    {#if !desktopAvailable}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("archive-desktop-required")}</span>
        <h3>{$translation("monitor-desktop-required")}</h3>
        <p>{$translation("monitor-desktop-required-detail")}</p>
      </section>
    {:else}
      <section
        class="kpi-grid monitor-kpis"
        aria-label={$translation("monitor-health-summary")}
      >
        <article class="kpi-card">
          <header>
            <span>{$translation("monitor-recorder-state")}</span><span
              class="coverage">{$translation("archive-read-only")}</span
            >
          </header>
          <strong>{phaseLabel()}</strong>
          <p>{$translation("monitor-recorder-native")}</p>
          <footer>
            <span>{timestamp(health?.last_filesystem_event_ms)}</span><span
              class="badge"
              data-kind="save_fact"
              >{$translation("monitor-event-watcher")}</span
            >
          </footer>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("monitor-queue-depth")}</span><span
              class="coverage">{$translation("monitor-current")}</span
            >
          </header>
          <strong
            >{formatNumber(health?.queue_depth ?? 0, $activeLocale)}</strong
          >
          <p>{$translation("monitor-queue-detail")}</p>
          <footer>
            <span>{$translation("monitor-durable-sqlite")}</span><span
              class="badge"
              data-kind="calculation"
              >{$translation("evidence-calculation")}</span
            >
          </footer>
        </article>
        <article
          class="kpi-card"
          data-attention={(health?.attention_count ?? 0) > 0}
        >
          <header>
            <span>{$translation("monitor-attention")}</span><span
              class="coverage">{$translation("monitor-retained")}</span
            >
          </header>
          <strong
            >{formatNumber(health?.attention_count ?? 0, $activeLocale)}</strong
          >
          <p>{$translation("monitor-attention-detail")}</p>
          <footer>
            <span>{$translation("monitor-no-core-blocking")}</span><span
              class="badge"
              data-kind="save_fact">{$translation("archive-read-only")}</span
            >
          </footer>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("monitor-latest-latency")}</span><span
              class="coverage">{$translation("monitor-save-sampled")}</span
            >
          </header>
          <strong>{latency(health?.last_processing_latency_ms)}</strong>
          <p>
            {health?.last_completed_file_name ??
              $translation("monitor-no-completed-candidate")}
          </p>
          <footer>
            <span>{timestamp(health?.last_completed_at_ms)}</span><span
              class="badge"
              data-kind="calculation"
              >{$translation("evidence-calculation")}</span
            >
          </footer>
        </article>
      </section>

      <section
        class="monitor-service-panel"
        aria-label={$translation("monitor-service-contract-label")}
      >
        <div>
          <span class="eyebrow">{$translation("monitor-service-contract")}</span
          >
          <h3>{$translation("monitor-service-title")}</h3>
          <p>{$translation("monitor-service-description")}</p>
        </div>
        <div
          class="monitor-service-path"
          aria-label={$translation("monitor-service-path-label")}
        >
          <span>{$translation("monitor-stage-event")}</span><i
            aria-hidden="true">→</i
          >
          <span>{$translation("monitor-stage-stability")}</span><i
            aria-hidden="true">→</i
          >
          <span>{$translation("monitor-stage-read")}</span><i aria-hidden="true"
            >→</i
          >
          <span>{$translation("monitor-stage-record")}</span>
        </div>
      </section>

      <section class="monitor-pulse" id="monitor-pulse">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("monitor-pulse-eyebrow")}</span>
            <h2>{$translation("monitor-pulse-title")}</h2>
            <p>{$translation("monitor-pulse-description")}</p>
          </div>
          <span
            class="status-chip"
            data-status={receiverDataset ? "stable" : "watch"}
            >{$translation(
              receiverDataset ? "evidence-save-fact" : "chart-unavailable",
            )}</span
          >
        </header>

        <div class="chart-grid monitor-chart-grid">
          <ObservatoryChart
            spec={cadenceChart}
            eyebrow={$translation("monitor-section-pulse")}
          />
          <ObservatoryChart
            spec={receiverChangeChart}
            eyebrow={$translation("monitor-latest-change")}
            help={receiverChangeHelp}
          />
        </div>

        {#if comparisonFailed}
          <p class="monitor-warning">
            {$translation("monitor-comparison-unavailable")}
          </p>
        {/if}

        <div class="monitor-pulse-facts">
          <article>
            <span>{$translation("monitor-largest-interval")}</span>
            <strong
              >{largestInterval === null
                ? $translation("chart-unavailable")
                : $translation("monitor-game-days-value", {
                    days: largestInterval,
                  })}</strong
            >
            <p>{$translation("monitor-largest-interval-note")}</p>
          </article>
          <article>
            <span>{$translation("monitor-classified-citizens")}</span>
            <strong
              >{latestPoint
                ? formatNumber(latestPoint.classified_total, $activeLocale)
                : $translation("chart-unavailable")}</strong
            >
            <p>{$translation("monitor-classified-note")}</p>
          </article>
          <article>
            <span>{$translation("monitor-city-scope")}</span>
            <strong
              >{formatNumber(
                latestObservation?.city_snapshot_count ?? 0,
                $activeLocale,
              )}</strong
            >
            <p>
              {$translation("monitor-city-scope-note", {
                fields: latestObservation?.city_snapshot_fields ?? 0,
              })}
            </p>
          </article>
          <article>
            <span>{$translation("monitor-branch-watch")}</span>
            <strong
              >{formatNumber(
                Math.max(0, (archive?.branches.length ?? 0) - 1),
                $activeLocale,
              )}</strong
            >
            <p>
              {$translation("monitor-branch-watch-note", {
                unresolved: archive?.unresolved_state_count ?? 0,
              })}
            </p>
          </article>
        </div>

        <GuidanceSurface
          kind="boundary"
          layout="compact"
          class="sidebar-note monitor-semantics-note"
        >
          <span aria-hidden="true">◇</span>
          <p>{$translation("monitor-unverified-statistics-note")}</p>
        </GuidanceSurface>
      </section>

      <section
        class="monitor-ledger"
        id="monitor-ledger"
        aria-label={$translation("monitor-ledger-label")}
      >
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("monitor-ledger-eyebrow")}</span
            >
            <h2>{$translation("monitor-ledger-title")}</h2>
            <p>{$translation("monitor-ledger-description")}</p>
          </div>
          <span class="coverage"
            >{$translation("monitor-ledger-count", {
              count: health?.latest_entries.length ?? 0,
            })}</span
          >
        </header>
        {#if !health?.latest_entries.length}
          <div class="archive-empty-state compact">
            <h3>{$translation("monitor-ledger-empty")}</h3>
            <p>{$translation("monitor-ledger-empty-detail")}</p>
          </div>
        {:else}
          <div class="monitor-ledger-table" role="table">
            <div role="row" class="monitor-ledger-header">
              <span role="columnheader"
                >{$translation("monitor-column-save")}</span
              >
              <span role="columnheader"
                >{$translation("monitor-column-state")}</span
              >
              <span role="columnheader"
                >{$translation("monitor-column-discovered")}</span
              >
              <span role="columnheader"
                >{$translation("monitor-column-attempts")}</span
              >
              <span role="columnheader"
                >{$translation("monitor-column-source")}</span
              >
            </div>
            {#each health.latest_entries as entry}
              <div
                role="row"
                class="monitor-ledger-row"
                data-status={entry.status}
              >
                <span role="cell"
                  ><strong>{entry.file_name}</strong><small
                    >{formatNumber(entry.file_size, $activeLocale)} B</small
                  ></span
                >
                <span role="cell"
                  ><i aria-hidden="true"></i>{$translation(
                    statusKeys[entry.status],
                  )}</span
                >
                <span role="cell"
                  ><time>{timestamp(entry.discovered_at_ms)}</time></span
                >
                <span role="cell"
                  >{formatNumber(entry.attempt_count, $activeLocale)}</span
                >
                <span role="cell"
                  >{$translation(discoveryKeys[entry.discovery_source])}</span
                >
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("monitor-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("monitor-inspector-eyebrow")}</span>
        <h2>{$translation("monitor-inspector-title")}</h2>
      </div>
      <span
        class="status-chip"
        data-status={latestEntry?.status === "terminal_failure"
          ? "exposed"
          : "stable"}
        >{latestEntry
          ? $translation(statusKeys[latestEntry.status])
          : $translation("archive-none")}</span
      >
    </div>

    {#if latestEntry}
      <div class="selected-reading">
        <span>{$translation("monitor-latest-candidate")}</span>
        <strong class="monitor-file-name">{latestEntry.file_name}</strong>
        <small
          >{timestamp(
            latestEntry.completed_at_ms ??
              latestEntry.last_attempt_at_ms ??
              latestEntry.discovered_at_ms,
          )}</small
        >
        {#if latestEntry.error_code}
          <p>{$translation("monitor-error-summary")}</p>
          <TechnicalDetails
            code={latestEntry.error_code}
            operation="save_recording"
          />
        {:else}
          <p>{$translation("monitor-candidate-safe-detail")}</p>
        {/if}
      </div>
      <div class="fact-grid">
        <article>
          <span>{$translation("monitor-column-attempts")}</span><strong
            >{latestEntry.attempt_count}</strong
          >
        </article>
        <article>
          <span>{$translation("monitor-latest-latency")}</span><strong
            >{latency(latestEntry.processing_latency_ms)}</strong
          >
        </article>
        <article>
          <span>{$translation("monitor-payload")}</span><strong
            >{latestEntry.payload_hash?.slice(0, 12) ??
              $translation("chart-unavailable")}</strong
          >
        </article>
        <article>
          <span>{$translation("monitor-column-source")}</span><strong
            >{$translation(discoveryKeys[latestEntry.discovery_source])}</strong
          >
        </article>
      </div>
    {:else}
      <div class="archive-inspector-empty">
        {$translation("monitor-no-candidates")}
      </div>
    {/if}

    <section class="provenance-key monitor-contract-key">
      <span class="eyebrow">{$translation("monitor-boundaries")}</span>
      <div>
        <i data-kind="save_fact"></i><span
          >{$translation("monitor-boundary-save-sampled")}</span
        >
      </div>
      <div>
        <i data-kind="calculation"></i><span
          >{$translation("monitor-boundary-app-open")}</span
        >
      </div>
      <div>
        <i data-kind="recommendation"></i><span
          >{$translation("monitor-boundary-no-memory")}</span
        >
      </div>
      <div>
        <i data-kind="game_definition"></i><span
          >{$translation("monitor-boundary-no-save-write")}</span
        >
      </div>
    </section>
  </aside>
</section>
