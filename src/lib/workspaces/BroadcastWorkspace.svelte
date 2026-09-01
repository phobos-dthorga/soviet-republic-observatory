<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import type { TranslationKey } from "../i18n/catalog";
  import { formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import { exactObservationChartBindings } from "../navigation/chartBindings";
  import {
    defaultWorkspaceLocation,
    type RelatedDataDestination,
    type WorkspaceFilters,
    type WorkspaceLocation,
  } from "../navigation/relatedData";
  import ReceiverEvidence from "../observations/ReceiverEvidence.svelte";
  import type {
    PublishedMetricContext,
    ReceiverDataset,
  } from "../observations/types";
  import {
    metricContextHelpFor,
    publishedMetricContext,
  } from "../presentation/metricContext";
  import { briefMetricLabel } from "../presentation/republicBrief";
  import { createObservedReceiverChart } from "../presentation/receiverObservation";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";

  let {
    receiverDataset = null,
    metricContexts = [],
    location,
    onlocationchange,
    onrelatednavigate,
  }: {
    receiverDataset?: ReceiverDataset | null;
    metricContexts?: PublishedMetricContext[];
    location: WorkspaceLocation;
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
    { label: "broadcast-section-receivers", href: "#receivers", marker: "01" },
    { label: "broadcast-section-audience", href: "#audience", marker: "02" },
    { label: "broadcast-section-programme", href: "#programme", marker: "03" },
    { label: "broadcast-section-outcomes", href: "#outcomes", marker: "04" },
    { label: "broadcast-section-bulletin", href: "#bulletin", marker: "05" },
  ];
  const stationIds = ["radio", "television"] as const;
  const stationFactKeys = [
    "station-workers",
    "station-professors",
    "station-potential-reach",
    "station-current-audience",
  ] as const satisfies readonly TranslationKey[];
  let selectedStation = $state<(typeof stationIds)[number]>("radio");
  const receiverLadder = $derived(
    receiverDataset
      ? createObservedReceiverChart(receiverDataset, $translation)
      : null,
  );
  const latestReceiverPoint = $derived(receiverDataset?.points.at(-1) ?? null);
  const receiverNavigation = $derived(
    receiverDataset && receiverLadder
      ? exactObservationChartBindings(
          receiverLadder,
          receiverDataset.points,
          defaultWorkspaceLocation("broadcast"),
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
          (metricId) => briefMetricLabel(metricId, $translation),
        )
      : null;
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

  function stationName(station: (typeof stationIds)[number]): string {
    return $translation(
      station === "radio" ? "station-radio" : "station-television",
    );
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
      <span class="edition">v1</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span>
        <strong
          >{receiverDataset?.branch_id ??
            $translation("observation-branch-unavailable")}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("filter-window")}</span>
        <strong
          >{receiverDataset
            ? $translation("observation-records", {
                count: receiverDataset.coverage.chartable_records,
              })
            : $translation("chart-unavailable")}</strong
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
      <p>{$translation("broadcast-evidence-sidebar-note")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="boundary"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
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

    <section id="receivers" class="broadcast-chart-wide">
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
      {:else}
        <GuidanceSurface kind="help" layout="block">
          <strong>{$translation("broadcast-no-receiver-title")}</strong>
          <span>{$translation("broadcast-no-receiver-detail")}</span>
        </GuidanceSurface>
      {/if}
    </section>

    <section id="audience" class="unavailable-laboratory">
      <span class="eyebrow">{$translation("broadcast-section-audience")}</span>
      <h2>{$translation("broadcast-audience-unavailable-title")}</h2>
      <GuidanceSurface kind="instruction" layout="block">
        <strong>{$translation("evidence-binary-research-candidate")}</strong>
        <span>{$translation("broadcast-audience-unavailable-detail")}</span>
      </GuidanceSurface>
    </section>

    <section id="programme" class="unavailable-laboratory">
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

    <section id="outcomes" class="unavailable-laboratory">
      <span class="eyebrow">{$translation("broadcast-section-outcomes")}</span>
      <h2>{$translation("broadcast-outcomes-unavailable-title")}</h2>
      <GuidanceSurface kind="boundary" layout="block">
        <strong>{$translation("causality-association-not-causation")}</strong>
        <span>{$translation("broadcast-outcomes-unavailable-detail")}</span>
      </GuidanceSurface>
    </section>

    <section class="notebook-panel" aria-labelledby="notebook-title">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">{$translation("broadcast-notebook")}</span>
          <h2 id="notebook-title">
            {$translation("broadcast-intervention-ledger")}
          </h2>
          <p>{$translation("evidence-annotations-not-evidence")}</p>
        </div>
        <span class="coverage"
          >{$translation("broadcast-notebook-empty-state")}</span
        >
      </header>
      <GuidanceSurface kind="help" layout="compact">
        <strong>{$translation("broadcast-notebook-empty-title")}</strong>
        <span>{$translation("broadcast-notebook-empty-detail")}</span>
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
      {#each stationFactKeys as key}
        <article>
          <span>{$translation(key)}</span>
          <strong>—</strong>
        </article>
      {/each}
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
  .unavailable-laboratory {
    margin-top: 10px;
    border: 1px solid var(--colour-line-faint);
    padding: 17px;
    background: var(--colour-surface-raised);
    scroll-margin-top: 18px;
  }

  .unavailable-laboratory h2 {
    margin: 5px 0 12px;
    font-size: 22px;
  }

  .boundary-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .notebook-panel :global(.guidance-surface) {
    margin-top: 14px;
  }

  @media (max-width: 900px) {
    .boundary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
